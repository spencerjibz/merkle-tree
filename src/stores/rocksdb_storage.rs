use super::NodeStore;
use crate::{Node, PathTrace};
use rocksdb::{
    BoundColumnFamily, DBWithThreadMode, Error, IteratorMode, MultiThreaded, Options, ReadOptions,
    WriteBatch, WriteOptions,
};
use std::sync::{Arc, Mutex};
pub type RocksDb = DBWithThreadMode<MultiThreaded>;
#[derive(Clone)]
pub struct RocksDbStore<'a> {
    pub db: &'a RocksDb,
    cf_node_store: Arc<BoundColumnFamily<'a>>,
    cf_hash_key_store: Arc<BoundColumnFamily<'a>>,
    node_store_batch: Arc<Mutex<WriteBatch>>,
    hash_key_tree_batch: Arc<Mutex<WriteBatch>>,
}
impl<'a> RocksDbStore<'a> {
    pub fn new(db: &'a RocksDb, name_space: &str) -> Result<Self, Error> {
        let node_store_name = name_space;
        let lookup_store = format!("{node_store_name}-lookup");
        db.create_cf(node_store_name, &Options::default())?;
        db.create_cf(&lookup_store, &Options::default())?;
        let cf_node_store = db.cf_handle(node_store_name).expect("failed to get handle");
        let cf_hash_key_store = db.cf_handle(&lookup_store).expect("failed to get handle");

        Ok(Self {
            db,
            cf_node_store,
            cf_hash_key_store,
            node_store_batch: Arc::default(),
            hash_key_tree_batch: Arc::default(),
        })
    }
}

impl NodeStore for RocksDbStore<'_> {
    fn store_type(&self) -> super::StoreType {
        super::StoreType::RocksDb
    }
    fn set(&mut self, key: PathTrace, value: Node) -> Option<Node> {
        let path: Vec<u8> = bincode::serialize(&key).ok()?;
        let node: Vec<u8> = bincode::serialize(&value).ok()?;
        let hash: Vec<u8> = bincode::serialize(&value.data).ok()?;
        let mut node_store_batch = self.node_store_batch.lock().unwrap();
        let mut hash_key_tree_batch = self.hash_key_tree_batch.lock().unwrap();
        node_store_batch.put_cf(&self.cf_node_store, &path, node);
        // skip updating this for duplicates
        if !self.db.key_may_exist_cf(&self.cf_hash_key_store, &hash) {
            let _ = self.db.put_cf(&self.cf_hash_key_store, hash, path);
        }
        Some(value)
    }

    fn get(&self, key: &PathTrace) -> Option<Node> {
        let path: Vec<u8> = bincode::serialize(&key).ok()?;
        self.db
            .get_cf(&self.cf_node_store, &path)
            .ok()?
            .and_then(|v| bincode::deserialize(&v).ok())
    }

    fn get_key_by_hash(&self, hash: &crate::Hash) -> Option<PathTrace> {
        if let Some(path_bytes) = self.db.get_cf(&self.cf_hash_key_store, hash).ok()? {
            return bincode::deserialize(&path_bytes).ok();
        }

        None
    }

    fn sort(&mut self) {
        // not needed, as we can easily find the the pathtrace of a node by hash
        // only needed for a  binary_search_hash
    }

    fn exists(&self, key: &PathTrace) -> bool {
        let key: Vec<u8> = bincode::serialize(&key).unwrap();
        self.db.key_may_exist_cf(&self.cf_node_store, &key)
    }

    fn reserve(&mut self, _items: usize) {
        // not required
    }

    fn update_value(&mut self, key: &PathTrace, next_value: Node) {
        self.set(*key, next_value);
        self.trigger_batch_actions();
    }

    fn entries(&self) -> impl Iterator<Item = (PathTrace, Node)> {
        let mut opts = ReadOptions::default();
        opts.fill_cache(false);
        let mut raw = self.db.raw_iterator_cf_opt(&self.cf_node_store, opts);
        raw.seek_to_first();
        let iter = std::iter::from_fn(move || {
            if raw.valid() {
                let key = raw.key().unwrap();
                let val = raw.value().unwrap();
                let path: PathTrace = bincode::deserialize(key).unwrap();
                let node: Node = bincode::deserialize(val).unwrap();
                raw.next();
                Some((path, node))
            } else {
                None
            }
        });

        iter
    }

    fn trigger_batch_actions(&mut self) {
        let mut opts = WriteOptions::default();
        opts.set_sync(false);
        opts.disable_wal(true);
        let mut node_store_batch = self.node_store_batch.lock().unwrap();
        let mut hash_key_tree_batch = self.hash_key_tree_batch.lock().unwrap();
        let node_store_batch = std::mem::take(&mut *node_store_batch);
        let hash_key_tree_batch = std::mem::take(&mut *hash_key_tree_batch);
        let _ = self.db.write_opt(node_store_batch, &opts);
        let _ = self.db.write_opt(hash_key_tree_batch, &opts);
    }

    fn remove_node(&mut self, key: PathTrace) {
        if let Ok(key_v) = bincode::serialize(&key) {
            if let Some(node) = self.get(&key) {
                if self.db.delete_cf(&self.cf_node_store, key_v).is_ok() {
                    //remove it from hash_key_store
                    let hash = node.data;
                    let _ = self.db.delete_cf(&self.cf_hash_key_store, hash);
                }
            }
        }
    }
}

pub fn temporary_rocks_db() -> RocksDb {
    let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = RocksDb::open(&opts, temp_dir.path()).expect("failed to open temporary RocksDB");
    db
}
pub fn create_large_input_byes_rockdb(
    size: usize,
    db: &RocksDb,
) -> (usize, impl Iterator<Item = Box<[u8]>> + use<'_>) {
    let column = format!("large-{size}-bytes");
    let mut opts = WriteOptions::default();
    opts.set_sync(false);
    opts.disable_wal(true);
    db.create_cf(&column, &Options::default())
        .expect("failed to create tree");
    let cf = db.cf_handle(&column).unwrap();
    let mut batch = WriteBatch::default();
    for i in 0..size {
        let bytes = i.to_be_bytes();
        batch.put_cf(&cf, bytes, bytes);
    }
    let _ = db.write_opt(batch, &opts);
    (
        size,
        db.iterator_cf(&cf, IteratorMode::Start)
            .flatten()
            .map(|(_, value)| value),
    )
}
