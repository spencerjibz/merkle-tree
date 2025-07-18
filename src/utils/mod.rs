pub use crate::stores::NodeStore;
use itertools::{peek_nth, Itertools};
#[cfg(any(
    feature = "sled",
    feature = "all-stores",
    feature = "rocksdb",
    feature = "fjall"
))]
use serde::{Deserialize, Serialize};
mod tree_construction;
use crate::hashers::{GlobalHasher, Hasher};
pub use tree_construction::*;
pub type Data = Vec<u8>;
pub type Hash = [u8; 32];
/// Which side to put Hash on when concatinating proof hashes
#[cfg_attr(
    any(
        feature = "sled",
        feature = "rocksdb",
        feature = "all-stores",
        feature = "fjall"
    ),
    derive(Serialize, Deserialize)
)]
#[repr(u8)]
#[derive(Debug, Clone, Default, Copy, PartialOrd, Ord, PartialEq, Eq, std::hash::Hash)]
pub enum HashDirection {
    Left = 0,
    Right = 1,
    #[default]
    Center = 2, // only for the root
}
impl HashDirection {
    pub fn reverse(&self) -> Self {
        match self {
            HashDirection::Left => HashDirection::Right,
            HashDirection::Right => HashDirection::Left,
            HashDirection::Center => HashDirection::Center,
        }
    }
    pub fn next_node_index(&self, index: usize) -> usize {
        match self {
            HashDirection::Left => index + 1,
            HashDirection::Right => index.saturating_sub(1),
            HashDirection::Center => index,
        }
    }
    pub fn from_index(index: usize) -> Self {
        if index % 2 == 0 {
            return HashDirection::Left;
        }
        HashDirection::Right
    }
}

// Our nodes
#[cfg_attr(
    any(
        feature = "sled",
        feature = "rocksdb",
        feature = "all-stores",
        feature = "fjall"
    ),
    derive(Serialize, Deserialize)
)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Node {
    pub is_leaf: bool,
    pub data: Hash,
    pub from_duplicate: bool,
}
impl Node {
    pub fn new<I: AsRef<[u8]>>(data: I, is_leaf: bool) -> Self {
        let data = GlobalHasher::hash_data(&data);
        Self {
            is_leaf,
            data,
            from_duplicate: false,
        }
    }
}
#[derive(Debug, Default)]
pub struct Proof {
    /// The hashes to use when verifying the proof
    /// The first element of the tuple is which side the hash should be on when concatinating
    /// Add level to the proof eases visualization of the proof
    pub hashes: Vec<(isize, HashDirection, Hash)>, // (level, direction, hash)
}
impl Proof {
    pub fn get_proof_in_hex(&self) -> Vec<(isize, HashDirection, String)> {
        self.hashes
            .iter()
            .map(|(level, direction, hash)| (*level, *direction, hex::encode(hash)))
            .collect()
    }
}

#[cfg_attr(
    any(
        feature = "sled",
        feature = "rocksdb",
        feature = "all-stores",
        feature = "fjall"
    ),
    derive(Serialize, Deserialize)
)]
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, std::hash::Hash)]
pub struct PathTrace {
    pub level: isize,
    pub direction: HashDirection,
    pub index: usize,
}
impl PathTrace {
    pub fn root(lowest_level: isize) -> Self {
        Self {
            level: lowest_level,
            ..Default::default()
        }
    }
    pub fn new(direction: HashDirection, level: isize, index: usize) -> Self {
        Self {
            level,
            direction,
            index,
        }
    }
    pub fn get_parent_path(&self, lowest_level: isize) -> Option<Self> {
        if self.level == lowest_level {
            return None;
        }
        if self.level == lowest_level + 1 {
            return Some(Self::new(HashDirection::Center, lowest_level, 0));
        }
        let level = self.level.saturating_sub(1);
        // we since we know the index of the child (item in the chunk)
        // parent_index =  child_index / chunkSize, we use only chunk of two
        // each level,  we have 2.pow(level) chunks, so root level 2^^0 = 1 chunk;
        let parent_index = self.index / 2;
        let direction = HashDirection::from_index(parent_index);
        Some(Self {
            level,
            direction,
            index: parent_index,
        })
    }
    pub fn get_sibling_path(&self) -> Self {
        let next_index = self.direction.next_node_index(self.index);
        Self {
            index: next_index,
            direction: self.direction.reverse(),
            ..*self
        }
    }
    pub fn generate_route(&self, lowest_level: isize) -> impl Iterator<Item = Self> {
        // only generate the path on demand
        use std::iter::successors;
        // To get the route, just work out the parent at level
        successors(Some(*self), move |current| {
            current.get_parent_path(lowest_level)
        })
    }
}
impl Ord for PathTrace {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // we want to ignore the direction because its based of the index any, (0,1)
        // since we have actual indexes at each level, the same index at different levels are not equal

        self.level
            .cmp(&other.level)
            .then_with(|| self.index.cmp(&other.index))
    }
}
impl PartialOrd for PathTrace {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
pub fn get_level_count(leaf_count: usize) -> isize {
    if leaf_count == 0 {
        return 0;
    }
    ((leaf_count as f64).log2().ceil()) as isize
}

pub fn example_data(n: usize) -> Vec<Data> {
    let mut data = vec![];
    for i in 0..n {
        data.push(vec![i as u8]);
    }
    data
}

pub fn max_index_at_level_reversed(leaf_count: usize, depth: isize, level: isize) -> usize {
    let shift = (depth - 1 - level) as usize;
    let nodes = (leaf_count + (1 << shift) - 1) >> shift; // ceil division
    nodes.saturating_sub(1)
}
/// add padding to support unbalanced trees
/// we resize to the nearest power of 2 and pad the last element
pub fn pad_input<R, I>(
    input: I,
    size_hint: usize,
) -> (usize, impl Iterator<Item = Node> + use<I, R>)
where
    R: AsRef<[u8]> + Clone,
    I: Iterator<Item = R>,
{
    let mut length = size_hint;
    assert!(!length > 1, "can't support less than 2 inputs");
    let input = input.map(|data| Node::new(data, true));
    let mut input = peek_nth(input);
    let last = input.peek_nth(length.saturating_sub(1));

    let fill_count = if !length.is_power_of_two() {
        length.next_power_of_two().saturating_sub(length)
    } else {
        0
    };
    length += fill_count;
    let mut last = *last.unwrap();
    last.from_duplicate = true;
    (length, input.pad_using(length, move |_| last))
}
#[cfg(test)]
mod path_trace {
    use super::*;

    #[test]
    fn getting_parent_paths() {
        let pt = PathTrace::new(HashDirection::Left, 3, 4);
        let lowest_level = 0;
        let parent = pt.get_parent_path(lowest_level).unwrap();
        assert_eq!(parent.level, 2);
        assert_eq!(parent.index, 2);
        assert_eq!(parent.direction, HashDirection::Left);

        let pt2 = PathTrace::new(HashDirection::Left, 1, 0);
        let parent2 = pt2.get_parent_path(lowest_level).unwrap();
        assert_eq!(parent2.level, 0);
        assert_eq!(parent2.direction, HashDirection::Center);
        assert_eq!(parent2.index, 0);

        let pt3 = PathTrace::new(HashDirection::Left, 0, 0);
        assert!(pt3.get_parent_path(lowest_level).is_none());
    }

    #[test]
    fn getting_sibling_paths() {
        let pt = PathTrace::new(HashDirection::Left, 2, 4);
        let sibling = pt.get_sibling_path();
        assert_eq!(sibling.index, 5);
        assert_eq!(sibling.direction, HashDirection::Right);
        assert_eq!(sibling.level, 2);

        let pt2 = PathTrace::new(HashDirection::Right, 2, 5);
        let sibling2 = pt2.get_sibling_path();
        assert_eq!(sibling2.index, 4);
        assert_eq!(sibling2.direction, HashDirection::Left);
        assert_eq!(sibling2.level, 2);
    }

    #[test]
    fn generating_routes() {
        let pt = PathTrace::new(HashDirection::Left, 3, 4);
        let lowest_level = -1;
        let route: Vec<_> = pt.generate_route(lowest_level).collect();
        dbg!(&route);
        assert_eq!(route.len(), 5);
        assert_eq!(route[0].level, 3);
        assert_eq!(route[1].level, 2);
        assert_eq!(route[2].level, 1);
        assert_eq!(route[3].level, 0);
    }
}
