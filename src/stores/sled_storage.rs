use super::NodeStore;
use crate::Node;
use crate::PathTrace;
use sled::{Batch, Config, Db, Mode, Result};
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
        let result = self.node_store.get(key).ok()?;
        if let Some(node) = result {
            let value = bincode::deserialize(&node).ok();
            return value;
        }
        None
    }
}
impl NodeStore for SledStore {
    fn store_type(&self) -> super::StoreType {
        super::StoreType::Sled
    }
    fn set(&mut self, key: crate::PathTrace, value: crate::Node) -> Option<crate::Node> {
        let path: IVec = bincode::serialize(&key).ok()?.into();
        let node: IVec = bincode::serialize(&value).ok()?.into();
        let hash = value.data;
        let _ = self.node_store.insert(&path, node);
        // skip updating this for duplicates
        if !self.hash_key_tree.contains_key(hash).unwrap_or_default() {
            let _ = self.hash_key_tree.insert(hash, path);
        }
        Some(value)
    }

    fn get(&self, key: &crate::PathTrace) -> Option<crate::Node> {
        let key: Vec<_> = bincode::serialize(&key).ok()?;
        self.get_node(key)
    }

    fn get_key_by_hash(&self, hash: &crate::Hash) -> Option<crate::PathTrace> {
        if let Some(path_bytes) = self.hash_key_tree.get(hash).ok()? {
            return bincode::deserialize(&path_bytes).ok();
        }

        None
    }

    fn sort(&mut self) {
        // do nothing, as can't binary_search by hash_value
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

    fn remove_node(&mut self, key: PathTrace) {
        if let Ok(key) = bincode::serialize(&key) {
            if let Some(node) = self
                .node_store
                .remove(key)
                .ok()
                .flatten()
                .and_then(|value| bincode::deserialize::<Node>(value.as_ref()).ok())
            {
                // remove it from the hash_key_tree;
                let _ = self.hash_key_tree.remove(node.data);
            }
        }
    }
}

pub fn create_large_input_byes_sled(size: usize, db: &Db) -> (usize, impl Iterator<Item = IVec>) {
    let tree = db
        .open_tree(format!("large-{size}-bytes"))
        .expect("failed to create tree");
    let mut batch = Batch::default();
    for i in 0..size {
        let bytes = i.to_be_bytes();
        batch.insert(&bytes, &bytes);
    }
    let _ = tree.apply_batch(batch);
    (size, tree.iter().values().flat_map(|v| v.ok()))
}
pub fn temporary_sled_db() -> Db {
    let config = Config::default().temporary(true).mode(Mode::HighThroughput);
    config.open().expect("failed to load temporary sled-db")
}
