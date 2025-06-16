use crate::{HashDirection, Node, NodeStore, PathTrace};
use fjall::{Batch, Config, Error, Keyspace, Partition, PartitionCreateOptions, Slice};
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Clone)]
pub struct FjallDbStore<'a> {
    db: &'a Keyspace,
    node_store: Partition,    // (path_trace, Node)
    hash_key_tree: Partition, // (hash, path_trace)
    node_store_batch: Arc<Mutex<Batch>>,
    hash_key_tree_batch: Arc<Mutex<Batch>>,
}

impl<'a> FjallDbStore<'a> {
    pub fn new(db: &'a Keyspace, name: impl AsRef<str>) -> Result<Self, Error> {
        let name = name.as_ref();
        let look_up = format!("{name}-lookup");
        let node_store = db.open_partition(name, PartitionCreateOptions::default())?;

        let hash_key_tree = db.open_partition(&look_up, PartitionCreateOptions::default())?;
        let node_store_batch = Arc::new(Mutex::new(db.batch()));
        let hash_key_tree_batch = Arc::new(Mutex::new(db.batch()));
        Ok(Self {
            db,
            node_store,
            hash_key_tree,
            node_store_batch,
            hash_key_tree_batch,
        })
    }
}

impl NodeStore for FjallDbStore<'_> {
    fn set(&mut self, key: PathTrace, value: Node) -> Option<Node> {
        let path: Vec<_> = bincode::serialize(&key).ok()?;
        let node: Vec<_> = bincode::serialize(&value).ok()?;
        let hash: Vec<_> = bincode::serialize(&value.data).ok()?;
        self.node_store_batch
            .lock()
            .unwrap()
            .insert(&self.node_store, &path, node);
        // skip updating this for duplicates
        if !self.hash_key_tree.contains_key(&hash).unwrap_or_default() {
            self.hash_key_tree_batch
                .lock()
                .unwrap()
                .insert(&self.hash_key_tree, hash, path);
        }
        Some(value)
    }

    fn get(&self, key: &PathTrace) -> Option<Node> {
        let path: Vec<u8> = bincode::serialize(&key).ok()?;
        self.node_store
            .get(&path)
            .ok()?
            .and_then(|v| bincode::deserialize(&v).ok())
    }

    fn get_key_by_hash(&self, hash: &crate::Hash) -> Option<PathTrace> {
        let key: Vec<u8> = bincode::serialize(&hash).ok()?;
        if let Some(path_bytes) = self.hash_key_tree.get(key).ok()? {
            return bincode::deserialize(&path_bytes).ok();
        }

        None
    }

    fn sort(&mut self) {
        // not required
    }

    fn shift_nodes_up(&mut self) {
        let mut batch = self.db.batch();
        let mut delete_batch = self.db.batch();
        let mut hash_key_batch = self.db.batch();
        for (mut path_trace, node) in self.entries() {
            let old_key = path_trace;
            path_trace.level += 1;
            if path_trace.direction == HashDirection::Center {
                path_trace.direction = HashDirection::Left;
                path_trace.index = 0;
            }
            let key: Vec<u8> = bincode::serialize(&path_trace).unwrap();
            let old_key: Vec<u8> = bincode::serialize(&old_key).unwrap();
            let hash: Vec<u8> = bincode::serialize(&node.data).unwrap();
            let node_bytes: Vec<u8> = bincode::serialize(&node).unwrap();
            if !node.from_duplicate {
                hash_key_batch.insert(&self.hash_key_tree, &hash, &key);
            }
            batch.insert(&self.node_store, &key, node_bytes);
            delete_batch.remove(&self.node_store, &old_key);
        }
        let _ = delete_batch.commit();
        let _ = batch.commit();
        let _ = hash_key_batch.commit();
    }

    fn exists(&self, key: &PathTrace) -> bool {
        let key: Vec<u8> = bincode::serialize(&key).unwrap();
        self.node_store.contains_key(key).ok().unwrap_or_default()
    }

    fn reserve(&mut self, _items: usize) {
        // not required
    }

    fn update_value(&mut self, key: &PathTrace, next_value: Node) {
        self.set(*key, next_value);
        self.trigger_batch_actions();
    }

    fn entries(&self) -> impl Iterator<Item = (PathTrace, Node)> {
        self.node_store.iter().flatten().flat_map(|(key, value)| {
            if let (Ok(path), Ok(node)) = (
                bincode::deserialize(key.as_ref()),
                bincode::deserialize(value.as_ref()),
            ) {
                //dbg!(&path, &node);
                return Some((path, node));
            }
            None
        })
    }

    fn trigger_batch_actions(&mut self) {
        let node_store_batch =
            std::mem::replace(&mut *self.node_store_batch.lock().unwrap(), self.db.batch());
        let hash_key_tree_batch = std::mem::replace(
            &mut *self.hash_key_tree_batch.lock().unwrap(),
            self.db.batch(),
        );
        node_store_batch.commit().expect("failed to insert");
        hash_key_tree_batch.commit().expect("failed to insert")
    }
}
pub fn temporary_fjall_db() -> Keyspace {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    Config::new(temp_dir)
        .open()
        .expect("failed to open temporary RocksDB")
}
pub fn create_large_input_byes_fjall(
    size: usize,
    db: &Keyspace,
) -> (usize, impl Iterator<Item = Slice> + use<'_>) {
    let column = format!("large-{size}-bytes");
    let cf = db
        .open_partition(&column, PartitionCreateOptions::default())
        .expect("failed to create tree");
    let mut batch = db.batch();
    for i in 0..size {
        let bytes = i.to_be_bytes();
        batch.insert(&cf, bytes, bytes);
    }
    let _ = batch.commit();
    (size, cf.iter().flatten().map(|(_, value)| value))
}
