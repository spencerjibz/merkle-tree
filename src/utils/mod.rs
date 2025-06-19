pub use crate::stores::NodeStore;
use itertools::{peek_nth, Itertools};
use serde::{Deserialize, Serialize};
use sha2::Digest;
mod tree_construction;
pub use tree_construction::*;
pub type Data = Vec<u8>;
pub type Hash = [u8; 32];
/// Which side to put Hash on when concatinating proof hashes
#[repr(u8)]
#[derive(
    Debug,
    Clone,
    Default,
    Copy,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
    std::hash::Hash,
    Serialize,
    Deserialize,
)]
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
            HashDirection::Right => index - 1,
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub struct Node {
    pub is_leaf: bool,
    pub data: Hash,
    pub from_duplicate: bool,
}
impl Node {
    pub fn new<I: AsRef<[u8]>>(data: I, is_leaf: bool) -> Self {
        let data = hash_data(&data);
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
    pub hashes: Vec<(usize, HashDirection, Hash)>, // (level, direction, hash)
}
impl Proof {
    pub fn get_proof_in_hex(&self) -> Vec<(usize, HashDirection, String)> {
        self.hashes
            .iter()
            .map(|(level, direction, hash)| (*level, *direction, hex::encode(hash)))
            .collect()
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, std::hash::Hash, Serialize, Deserialize)]
pub struct PathTrace {
    pub level: usize,
    pub direction: HashDirection,
    pub index: usize,
}
impl PathTrace {
    pub fn root() -> Self {
        Self::default()
    }
    pub fn new(direction: HashDirection, level: usize, index: usize) -> Self {
        Self {
            level,
            direction,
            index,
        }
    }
    pub fn get_parent_path(&self) -> Option<Self> {
        match self.level {
            0 => None,
            1 => Some(Self::new(HashDirection::Center, 0, 0)),
            _ => {
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
        }
    }
    pub fn get_sibling_path(&self) -> Self {
        let next_index = self.direction.next_node_index(self.index);
        Self {
            index: next_index,
            direction: self.direction.reverse(),
            ..*self
        }
    }
    pub fn generate_route(&self) -> impl Iterator<Item = Self> {
        // only generate the path on demand
        use std::iter::successors;
        // To get the route, just work out the parent at level
        successors(Some(*self), |current| current.get_parent_path())
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
// ------------------------- UTILITY FUNCTIONS --------------------------------------------------
pub fn hash_data<T: AsRef<[u8]>>(data: &T) -> Hash {
    let hash = sha2::Sha256::digest(data.as_ref());
    hash.into()
}

pub fn hash_concat<T: AsRef<[u8]>>(h1: &T, h2: &T) -> Hash {
    hash_data(&[h1.as_ref(), h2.as_ref()].concat())
}
pub fn get_level_count(leaf_count: usize) -> usize {
    if leaf_count == 0 {
        return 0;
    }
    ((leaf_count as f64).log2().ceil()) as usize
}

pub fn example_data(n: usize) -> Vec<Data> {
    let mut data = vec![];
    for i in 0..n {
        data.push(vec![i as u8]);
    }
    data
}

pub fn max_index_at_level_reversed(leaf_count: usize, depth: usize, level: usize) -> usize {
    let shift = depth - 1 - level;
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
        let parent = pt.get_parent_path().unwrap();
        assert_eq!(parent.level, 2);
        assert_eq!(parent.index, 2);
        assert_eq!(parent.direction, HashDirection::Left);

        let pt2 = PathTrace::new(HashDirection::Left, 1, 0);
        let parent2 = pt2.get_parent_path().unwrap();
        assert_eq!(parent2.level, 0);
        assert_eq!(parent2.direction, HashDirection::Center);
        assert_eq!(parent2.index, 0);

        let pt3 = PathTrace::new(HashDirection::Left, 0, 0);
        assert!(pt3.get_parent_path().is_none());
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
        let route: Vec<_> = pt.generate_route().collect();
        assert_eq!(route.len(), 4);
        assert_eq!(route[0].level, 3);
        assert_eq!(route[1].level, 2);
        assert_eq!(route[2].level, 1);
        assert_eq!(route[3].level, 0);
    }
}
