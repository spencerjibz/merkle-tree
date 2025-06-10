use super::NodeStore;
use crate::PathTrace;
use crate::{HashDirection, Node};
use sled::{Batch, Db, Result};
use sled::{IVec, Tree};
#[derive(Clone, Debug)]
pub struct SledStore {
    node_store: Tree,    // (path_trace, Node)
    hash_key_tree: Tree, // (hash, path_trace)
    node_store_batch: Batch,
    hash_key_tree_batch: Batch,
}

impl SledStore {
    pub fn new(db: &Db, name: &str) -> Result<Self> {
        let node_store = db.open_tree(name)?;
        let hash_key_tree = db.open_tree(format!("{name}-lookup"))?;
        let node_store_batch = Batch::default();
        let hash_key_tree_batch = Batch::default();
        Ok(Self {
            node_store,
            hash_key_tree,
            node_store_batch,
            hash_key_tree_batch,
        })
    }
    pub fn get_node(&self, key: impl AsRef<[u8]>) -> Option<Node> {
        //dbg!(self.node_store.len());
        let result = self.node_store.get(key).ok()?;
        if let Some(node) = result {
            let value = bincode::deserialize(&node).ok();
            return value;
        }
        None
    }
}
impl NodeStore for SledStore {
    fn set(&mut self, key: crate::PathTrace, value: crate::Node) -> Option<crate::Node> {
        let path: Vec<u8> = bincode::serialize(&key).ok()?;
        let node: Vec<u8> = bincode::serialize(&value).ok()?;
        let hash = bincode::serialize(&value.data).ok()?;
        self.node_store_batch.insert(path.clone(), node);
        // skip updating this for duplicates
        if !self.hash_key_tree.contains_key(&hash).unwrap_or_default() {
            self.hash_key_tree_batch.insert(hash, path);
        }
        Some(value)
    }

    fn get(&self, key: &crate::PathTrace) -> Option<crate::Node> {
        let key: Vec<_> = bincode::serialize(&key).ok()?;
        self.get_node(key)
    }

    fn get_key_by_hash(&self, hash: &crate::Hash) -> Option<crate::PathTrace> {
        let key: Vec<_> = bincode::serialize(&hash).ok()?;
        if let Some(path_bytes) = self.hash_key_tree.get(key).ok()? {
            return bincode::deserialize(&path_bytes).ok();
        }

        None
    }

    fn sort(&mut self) {
        // do nothing, as can't binary_search by hash_value
    }

    fn shift_nodes_up(&mut self) {
        let cursor = self.node_store.iter().keys();
        let mut batch_delete = sled::Batch::default();
        let mut batch_insert = sled::Batch::default();
        let mut batch_hash_path_update = sled::Batch::default();

        for path in cursor.flatten() {
            let mut path_trace =
                bincode::deserialize::<PathTrace>(path.as_ref()).unwrap_or_default();
            // mode the path_trace up by level
            path_trace.level += 1;
            if path_trace.direction == HashDirection::Center {
                path_trace.direction = HashDirection::Left;
                path_trace.index = 0;
            }
            let value = self
                .node_store
                .get(&path)
                .unwrap_or_default()
                .unwrap_or_default();
            let node: Node = bincode::deserialize(&value).unwrap_or_default();

            let key: Vec<_> = bincode::serialize(&path_trace).ok().unwrap_or_default();
            let hash = bincode::serialize(&node.data).unwrap_or_default();
            if !node.from_duplicate {
                batch_hash_path_update.insert(hash, key.clone());
            }

            batch_insert.insert(key, &value);
            batch_delete.remove(&path);
        }
        let _ = self.node_store.apply_batch(batch_delete);
        let _ = self.node_store.apply_batch(batch_insert);
        let _ = self.hash_key_tree.apply_batch(batch_hash_path_update);
    }

    fn exists(&self, key: &crate::PathTrace) -> bool {
        let key: Vec<_> = bincode::serialize(&key).ok().unwrap_or_default();
        self.node_store.contains_key(key).unwrap_or_default()
    }

    fn reserve(&mut self, _items: usize) {
        // not supported by sled, do nothing
    }

    fn entries(&self) -> impl Iterator<Item = (crate::PathTrace, crate::Node)> {
        self.node_store.iter().flat_map(|entry| {
            if let Ok((key, value)) = entry {
                if let (Ok(path), Ok(node)) = (
                    bincode::deserialize(key.as_ref()),
                    bincode::deserialize(value.as_ref()),
                ) {
                    //dbg!(&path, &node);
                    return Some((path, node));
                }
            }
            None
        })
    }

    fn update_value(&mut self, key: &PathTrace, next_value: Node) {
        self.set(*key, next_value);
        self.trigger_batch_actions();
    }

    fn trigger_batch_actions(&mut self) {
        let node_store_batch = std::mem::take(&mut self.node_store_batch);
        let hash_key_tree_batch = std::mem::take(&mut self.hash_key_tree_batch);
        self.node_store
            .apply_batch(node_store_batch)
            .expect("failed to insert");
        self.hash_key_tree
            .apply_batch(hash_key_tree_batch)
            .expect("failed to insert")
    }
}

pub fn create_large_input_byes(size: usize, db: &Db) -> (usize, impl Iterator<Item = IVec>) {
    let tree = db
        .open_tree(format!("large-{size}-bytes"))
        .expect("failed to create tree");
    let mut batch = Batch::default();
    for i in 0..size {
        let bytes = i.to_ne_bytes();
        batch.insert(&bytes, &bytes);
    }
    let _ = tree.apply_batch(batch);
    (tree.len(), tree.iter().values().flat_map(|v| v.ok()))
}
