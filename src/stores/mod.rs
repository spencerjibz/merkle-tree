use crate::HashDirection;
mod fjall_storage;
mod rocksdb_storage;
mod sled_storage;
use super::{Hash, Node, PathTrace};
pub use fjall_storage::*;
use indexmap::IndexMap;
use itertools::Itertools;
pub use rocksdb_storage::*;
pub use sled_storage::*;
#[derive(Debug, Clone, Copy)]
pub enum StoreType {
    Sled,
    RocksDb,
    IndexMap,
    Fjall,
}
pub trait NodeStore {
    fn store_type(&self) -> StoreType {
        StoreType::IndexMap
    }
    /// change the direction of the current root from (level, Center, 0) to (level, left, 0)
    fn shift_root_to_left(&mut self, lowest_level: isize) {
        let mut root_path = PathTrace::root(lowest_level);
        if let Some(root_node) = self.get(&root_path) {
            self.remove_node(root_path);
            root_path.direction = HashDirection::Left;
            let _ = self.set(root_path, root_node);
        }
    }
    /// add new values to the store, (this could also be scheduling a batch insert)
    fn set(&mut self, key: PathTrace, value: Node) -> Option<Node>;
    fn get(&self, key: &PathTrace) -> Option<Node>;
    fn get_key_by_hash(&self, hash: &Hash) -> Option<PathTrace>;
    /// sort the items by value, for store that support binary_search by value
    fn sort(&mut self);
    fn exists(&self, key: &PathTrace) -> bool;
    fn reserve(&mut self, items: usize);
    fn update_value(&mut self, key: &PathTrace, next_value: Node);
    fn entries(&self) -> impl Iterator<Item = (PathTrace, Node)>;
    fn trigger_batch_actions(&mut self);
    fn remove_node(&mut self, key: PathTrace);
    fn unique_leaf_count(&self) -> usize {
        self.entries()
            .filter(|pairs| pairs.1.is_leaf)
            .unique_by(|pairs| pairs.1.data)
            .count()
    }
}

// Our tree is built bottom up, we use indexes at each level to identify the nodes, and use the index to calculate the parent node
// Our level index ordering is reversed for ease of use and lookup, so our root is at level 0, and the leaves are at the highest level
pub(crate) type TreeCache = IndexMap<PathTrace, Node>;

impl NodeStore for TreeCache {
    fn trigger_batch_actions(&mut self) {
        // do nothing as this is not supported for this store
    }
    fn reserve(&mut self, items: usize) {
        self.reserve(items);
    }
    fn exists(&self, key: &PathTrace) -> bool {
        self.contains_key(key)
    }
    fn set(&mut self, key: PathTrace, value: Node) -> Option<Node> {
        self.insert(key, value)
    }

    fn get(&self, key: &PathTrace) -> Option<Node> {
        self.get(key).cloned()
    }

    fn get_key_by_hash(&self, target_hash: &Hash) -> Option<PathTrace> {
        self.binary_search_by(|_, node| node.data.cmp(target_hash))
            .ok()
            .map(|index| self.get_index(index).map(|(key, _)| *key))?
    }
    fn sort(&mut self) {
        self.sort_unstable_by(|path, node1, path2, node2| {
            node1.data.cmp(&node2.data).then_with(|| path.cmp(path2))
        });
    }
    fn entries(&self) -> impl Iterator<Item = (PathTrace, Node)> {
        self.iter().map(|(k, v)| (*k, *v))
    }

    fn update_value(&mut self, key: &PathTrace, next_value: Node) {
        if let Some(current) = self.get_mut(key) {
            *current = next_value
        }
    }
    fn remove_node(&mut self, key: PathTrace) {
        let _ = self.shift_remove(&key);
    }
}
