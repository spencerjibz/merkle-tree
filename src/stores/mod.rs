use crate::HashDirection;
mod fjall_storage;
mod rocksdb_storage;
mod sled_storage;
use super::{Hash, Node, PathTrace};
pub use fjall_storage::*;
use indexmap::IndexMap;
pub use rocksdb_storage::*;
pub use sled_storage::*;

#[derive(Debug, Clone, Copy)]
pub enum StoreType {
    Sled,
    RocksDb,
    IndexMap,
}
pub trait NodeStore {
    fn store_type(&self) -> StoreType {
        StoreType::IndexMap
    }
    // add new values to the store, (this could also be scheduling a batch insert)
    fn set(&mut self, key: PathTrace, value: Node) -> Option<Node>;
    fn get(&self, key: &PathTrace) -> Option<Node>;
    fn get_key_by_hash(&self, hash: &Hash) -> Option<PathTrace>;
    /// sort the items by value, for store that support binary_search by value
    fn sort(&mut self);
    /// move all the nodes up by level (expensive for some stores)
    fn shift_nodes_up(&mut self);
    fn exists(&self, key: &PathTrace) -> bool;
    fn reserve(&mut self, items: usize);
    fn update_value(&mut self, key: &PathTrace, next_value: Node);
    fn entries(&self) -> impl Iterator<Item = (PathTrace, Node)>;
    fn trigger_batch_actions(&mut self);
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
        self.sort_unstable_by(|_, node1, _, node2| node1.data.cmp(&node2.data));
    }
    fn shift_nodes_up(&mut self) {
        let temp: Vec<_> = self.drain(..).collect();
        temp.into_iter().for_each(|(mut key, value)| {
            key.level += 1;
            if key.direction == HashDirection::Center {
                key.direction = HashDirection::Left;
                key.index = 0;
            }
            self.set(key, value);
        });
    }
    fn entries(&self) -> impl Iterator<Item = (PathTrace, Node)> {
        self.iter().map(|(k, v)| (*k, v.clone()))
    }

    fn update_value(&mut self, key: &PathTrace, next_value: Node) {
        if let Some(current) = self.get_mut(key) {
            *current = next_value
        }
    }
}
