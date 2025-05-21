/*

Building a simple Merkle Tree

Exercise 1:
    Given a set of data D, construct a Merkle Tree.

Assume that D is a power of 2 (the binary tree is perfect).

Example input:
    D = [A1, A2, A3, A4]

Example output:

                               Root
                           ┌──────────┐
                           │    H7    │
                           │ H(H5|H6) │
                  ┌────────┴──────────┴──────────┐
                  │                              │
                  │                              │
             ┌────┴─────┐                  ┌─────┴────┐
             │    H5    │                  │    H6    │
             │ H(H1|H2) │                  │ H(H3|H4) │
             └─┬─────┬──┘                  └─┬──────┬─┘
               │     │                       │      │
     ┌─────────┴┐   ┌┴─────────┐    ┌────────┴─┐  ┌─┴────────┐
     │   H1     │   │    H2    │    │    H3    │  │    H4    │
     │  H(A1)   │   │   H(A2)  │    │   H(A3)  │  │   H(A4)  │
     └───┬──────┘   └────┬─────┘    └────┬─────┘  └────┬─────┘
         │               │               │             │
         A1              A2              A3            A4


Exercise 1b:
    Write a function that will verify a given set of data with a given root hash.

Exercise 2:
    Write a function that will use a proof like the one in Exercise 3 to verify that the proof is indeed correct.

Exercise 3 (Hard):
    Write a function that returns a proof that a given data is in the tree.

    Hints:
        -   The proof should be a set of ordered data hashes and their positions (left 0 or right 1).
        -   Let's say we are asked to prove that H3 (A3) is in this tree. We have the entire tree so we can traverse it and find H3.
            Then we only need to return the hashes that can be used to calculate with the hash of the given data to calculate the root hash.
            i.e Given a data H3, a proof [(1, H4), (0, H5)] and a root:
                H3|H4 => H6 => H5|H6 => H7 = root

 */
pub mod utils;
use std::collections::{BTreeMap, HashMap};
pub use utils::*;
/*
* My previous implementation,
#[derive(Debug, Default, Clone)]
struct MerkleNode {
    hash: Hash,
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
    level: usize,
    is_leaf: bool,
    index: usize,
}
impl MerkleNode {
    pub fn new(hash: Hash, level: usize) -> Self {
        Self {
            hash,
            level,
            ..Default::default()
        }
    }
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}
*/
/// add padding to support unbalanced trees
/// we resize to the nearest power of 2 and pad the last element
fn pad_input(input: &[Data]) -> Vec<Data> {
    let mut padded = input.to_vec();
    let length = padded.len();
    if !length.is_power_of_two() {
        let next_power = length.next_power_of_two();
        if let Some(last) = input.last() {
            padded.resize(next_power, last.clone());
        }
    }

    padded
}
#[derive(Debug)]
pub struct MerkleTree {
    root: Hash,
    // for faster lookup , (level, Direction, index), eg. for H2, (depthlength, Right, index)
    pub tree_cache: TreeCache, // O(1) - path look ups, O(n) search by hash
}

impl MerkleTree {
    /// Gets root hash for this tree
    pub fn root(&self) -> &Hash {
        &self.root
    }

    /// Constructs a Merkle tree from given input data
    pub fn construct(input: &[Data]) -> MerkleTree {
        let input = pad_input(input);
        let level_count = get_level_count(input.len());
        let mut tree_cache = HashMap::new();
        let mut nodes: Vec<(PathTrace, Hash)> = input
            .iter()
            .enumerate()
            .map(|(index, data)| {
                let data = hash_data(data);
                let direction = HashDirection::from_index(index);
                let path = PathTrace::new(direction, level_count, index);

                (path, data)
            })
            .collect();
        while nodes.len() > 1  {
            //  reduce allocations as length of nodes to process halves at every level up.
            let mut next_level = Vec::with_capacity(nodes.len() / 2);
            let mut cursor = nodes.into_iter();
            let mut items_index_per_level: HashMap<usize, usize> = HashMap::new();
            while let Some((left, hash)) = cursor.next() {
                let (right, right_hash) = cursor.next().unwrap_or_else(|| (left, hash.clone()));

                let level = left.level - 1;
                let parent_index = items_index_per_level.entry(level).or_default();
                let mut direction = HashDirection::from_index(*parent_index);
                // when we get the root node
                if level == 0 {
                    direction = HashDirection::Center;
                }
                let parent_hash = hash_concat(&hash, &right_hash);
                let parent = PathTrace::new(direction, level, *parent_index);
                tree_cache.insert(left, hash);
                tree_cache.insert(right, right_hash);
                tree_cache.insert(parent, parent_hash.to_vec());
                *parent_index += 1;
                next_level.push((parent, parent_hash));
            }
            nodes = next_level;
        }
        let (_root_path, root) = nodes.pop().unwrap();
        Self { root, tree_cache }
    }

    pub fn get_parent_hash(&self, path_trace: &PathTrace) -> Option<&Hash> {
        // numbers of items of that level is 2^level;
        // our parent is always one level up;
        path_trace
            .get_parent_path()
            .and_then(|path| self.tree_cache.get(&path))
    }

    pub fn fetch_cache_pathtrace(&self, target_hash: &Hash) -> Option<PathTrace> {
        self.tree_cache
            .iter()
            .find_map(|(path, hash)| (hash == target_hash).then_some(*path))
    }

    pub fn print_data_route(&self, data: &Data) {
        let route = self.find_data_route(data);
        route.iter().for_each(|path| {
            println!(
                "level {}:  {:?} Node {} -> {}",
                path.level,
                path.direction,
                path.index,
                self.tree_cache
                    .get(path)
                    .map(hex::encode)
                    .unwrap_or_default()
            );
        });
    }

    pub fn find_data_route(&self, data: &Data) -> Vec<PathTrace> {
        let target_hash = hash_data(data);
        if let Some(cached_path) = self.fetch_cache_pathtrace(&target_hash) {
            return cached_path.generate_route().collect();
        }
        vec![]
    }

    /// Verifies that the given input data produces the given root hash
    pub fn verify(input: &[Data], root_hash: &Hash) -> bool {
        let generated_tree = Self::construct(input);
        generated_tree.root() == root_hash
    }

    /// Verifies that the given data and proof_path correctly produce the given root_hash
    pub fn verify_proof(data: &Data, proof: &Proof, root_hash: &Hash) -> bool {
        let hashed_data = hash_data(data);
        let generated = proof
            .hashes
            .iter()
            .fold(hashed_data, |acc, &(_, direction, next_hash)| {
                let is_left = direction == HashDirection::Left;
                if is_left {
                    return hash_concat(next_hash, &acc);
                }
                hash_concat(&acc, next_hash)
            });
        &generated == root_hash
    }

    /// Returns a list of hashes that can be used to prove that the given data is in this tree
    pub fn prove(&self, data: &Data) -> Option<Proof> {
        // we use our tree_cache and some math to calculate the sibling_node at each parent level
        // See PathTrace for math
        let target_hash = hash_data(&data);
        if let Some(trace) = self.fetch_cache_pathtrace(&target_hash) {
            let hashes: Vec<_> = trace
                .generate_route()
                .take_while(|path| path.level >= 1) // ingnore the root
                .flat_map(|mut path| {
                    path = path.get_sibling_path();
                    self.tree_cache
                        .get(&path)
                        .map(|hash| (path.level, path.direction, hash))
                })
                .collect();

            return Some(Proof { hashes });
        }
        None
    }

    pub fn pretty_print(&self) {
        let mut nodes_per_level: BTreeMap<PathTrace, Vec<(PathTrace, &Hash)>> = BTreeMap::new();
        self.tree_cache.iter().for_each(|(path, hash)| {
            if let Some(parent) = path.get_parent_path() {
                let entry = nodes_per_level.entry(parent).or_default();
                entry.push((*path, hash));
            }
        });
        println!("-----------------Tree-----------------------------");
        for (parent, mut nodes) in nodes_per_level {
            if let Some(parent_hash) = self.tree_cache.get(&parent).map(hex::encode) {
                let header = if parent.level == 0 {
                    format!("Root: {}", parent_hash)
                } else {
                    format!(
                        " (Parent: {}-{:?}-{}): {}",
                        parent.level, parent.direction, parent.index, parent_hash
                    )
                };
                println!("{}", header);
                nodes.sort_unstable_by(|a, b| a.0.cmp(&b.0));
                nodes.into_iter().for_each(|(path, hash)| {
                    let is_left = path.direction == HashDirection::Left;
                    let prefix = "    ";
                    let branch = if is_left { "├── " } else { "└── " };
                    println!(
                        "{}{}{:?} {}: {}",
                        branch,
                        prefix,
                        path.direction,
                        path.index,
                        hex::encode(hash)
                    );
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructions() {
        let data = example_data(4);
        let tree = MerkleTree::construct(&data);
        let expected_root = "9675e04b4ba9dc81b06e81731e2d21caa2c95557a85dcfa3fff70c9ff0f30b2e";
        assert_eq!(hex::encode(tree.root()), expected_root);

        //Uncomment if your implementation allows for unbalanced trees
        let data = example_data(3);
        let tree = MerkleTree::construct(&data);
        // using dupliate padding of the last value
        // (correction here) let expected_root = "773a93ac37ea78b3f14ac31872c83886b0a0f1fec562c4e848e023c889c2ce9f";
        let expected_root = "f2dcdd96791b6bac5d554f2d320e594b834f5da1981812c3707e7772234cb0ad";
        assert_eq!(hex::encode(tree.root()), expected_root);

        let data = example_data(8);
        let tree = MerkleTree::construct(&data);
        let expected_root = "0727b310f87099c1ba2ec0ba408def82c308237c8577f0bdfd2643e9cc6b7578";
        assert_eq!(hex::encode(tree.root()), expected_root);
    }
    #[test]
    fn test_verifing_proof() {
        (1_usize..100).for_each(|index| {
            let data = example_data(index);
            let tree = MerkleTree::construct(&data);
            let root_hash = tree.root();
            for h1 in data.iter() {
                if let Some(proof) = tree.prove(h1) {
                    assert!(MerkleTree::verify_proof(h1, &proof, root_hash))
                }
            }
        });
    }
    #[test]
    fn verifies_data_set_forms_root() {
        let pairs = [
            (
                7,
                "e263b77a6d80c1c56f3f67d1e0d803ad8eb2ac9d66c82f78735207c886a1592c",
            ),
            (
                4,
                "9675e04b4ba9dc81b06e81731e2d21caa2c95557a85dcfa3fff70c9ff0f30b2e",
            ),
            (
                8,
                "0727b310f87099c1ba2ec0ba408def82c308237c8577f0bdfd2643e9cc6b7578",
            ),
        ];
        for (input, hash) in pairs {
            let root_hash = hex::decode(hash).unwrap();
            let data = example_data(input);
            assert!(MerkleTree::verify(&data, &root_hash))
        }
    }
}
