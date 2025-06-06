use crate::HashDirection;

use super::{Hash, Node, PathTrace};
use indexmap::IndexMap;
pub trait NodeStore {
    fn set(&mut self, key: PathTrace, value: Node) -> Option<Node>;
    fn get(&self, key: &PathTrace) -> Option<&Node>;
    fn get_key_by_hash(&self, hash: &Hash) -> Option<PathTrace>;
    fn get_mut(&mut self, key: &PathTrace) -> Option<&mut Node>;
    /// sort the items by value, for store that support binary_search by value
    fn sort(&mut self);
    /// move all the nodes up by level (expensive for some stores)
    fn shift_nodes_up(&mut self);
    fn exists(&self, key: &PathTrace) -> bool;
    fn reserve(&mut self, items: usize);

    fn entries(&self) -> impl Iterator<Item = (&PathTrace, &Node)>;
}

// Our tree is built bottom up, we use indexes at each level to identify the nodes, and use the index to calculate the parent node
// Our level index ordering is reversed for ease of use and lookup, so our root is at level 0, and the leaves are at the highest level
pub(crate) type TreeCache = IndexMap<PathTrace, Node>;

impl NodeStore for TreeCache {
    fn get_mut(&mut self, key: &PathTrace) -> Option<&mut Node> {
        self.get_mut(key)
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

    fn get(&self, key: &PathTrace) -> Option<&Node> {
        self.get(key)
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
    fn entries(&self) -> impl Iterator<Item = (&PathTrace, &Node)> {
        self.iter()
    }
}
