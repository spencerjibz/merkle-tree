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
use bytes::Bytes;
use itertools::Itertools;
use std::collections::BTreeMap;
pub use utils::*;
#[derive(Debug)]
pub struct MerkleTree<Store: NodeStore> {
    root: Hash,
    is_padded: bool,
    pub leaf_count: usize,
    level_count: usize,
    unique_leaf_count: usize, // for faster lookups of leaves by data
    items_index_per_level: Vec<usize>,
    padding_start: usize,
    // for faster lookup , (level, Direction, index), eg. for H2, (depthlength, Right, index)
    pub tree_cache: Store, // O(1) - path look ups, O(n) search by hash
}

impl<Store: NodeStore> MerkleTree<Store> {
    /// Gets root hash for this tree
    pub fn root(&self) -> &Hash {
        &self.root
    }

    /// Constructs a Merkle tree from given input data
    pub fn construct(input: &[Data], store: Store) -> Self {
        let unique_leaf_count = input.iter().unique().size_hint().1.unwrap_or_default();
        let is_padded = !input.len().is_power_of_two();
        let (leaf_count, input) = pad_input(input);
        let level_count = get_level_count(leaf_count);
        let mut items_index_per_level = vec![0; level_count];
        let mut tree_cache = store;
        let (_root_path, root) = build_tree(
            &mut tree_cache,
            input,
            &mut items_index_per_level,
            level_count,
            false,
            0,
        );
        let padding_start = unique_leaf_count - 1;

        tree_cache.sort();

        Self {
            items_index_per_level,
            padding_start,
            root: root.data,
            unique_leaf_count,
            is_padded,
            leaf_count,
            level_count,
            tree_cache,
        }
    }
    /// update the target_hash with a new one.
    pub fn update(&mut self, target_hash: &Hash, new: Hash) {
        if let Some(current) = self.fetch_cache_pathtrace(target_hash) {
            if let Some(target_node) = self.tree_cache.get_mut(&current) {
                target_node.data = new;
                self.cascade_update(current);
            }
        }
    }

    /// updates the hash up every level to the root.
    pub fn cascade_update(&mut self, current: PathTrace) {
        current.generate_route().for_each(|path| {
            if let (Some(current_node), Some(sibling_node)) = (
                self.tree_cache.get(&path),
                self.tree_cache.get(&path.get_sibling_path()),
            ) {
                // check for direction
                let next_parent_hash = if path.direction == HashDirection::Left {
                    hash_concat(&current_node.data, &sibling_node.data)
                } else {
                    hash_concat(&sibling_node.data, &current_node.data)
                };
                if let Some(parent_node) = path
                    .get_parent_path()
                    .and_then(|parent_path| self.tree_cache.get_mut(&parent_path))
                {
                    parent_node.data = next_parent_hash;
                }
            }
        });
        // update self. root;
        if let Some(new_root) = self.tree_cache.get(&PathTrace::root()) {
            self.root = Bytes::copy_from_slice(&new_root.data);
        }
    }
    pub fn append(&mut self, data: &Data) {
        if !self.is_padded {
            self.expand_tree(data);
            self.tree_cache.sort();
            return;
        }
        // handling cases of adding to un already unbalanced tree with padding;
        self.expand_padded(data);
        self.tree_cache.sort();
    }
    /// expands the tree by the next_power_of_two
    pub fn expand_tree(&mut self, data: &Data) {
        // leaf_counts is already a is_power_of_two
        let next_needed_nodes = (self.leaf_count + 1).next_power_of_two() - self.leaf_count;
        let node = Node::new(data, true);
        let input = std::iter::repeat_n(node, next_needed_nodes);
        let total_tree_nodes = 2 * next_needed_nodes - 1;
        // move every path up by level up
        self.tree_cache.shift_nodes_up();
        self.tree_cache.reserve(total_tree_nodes);
        self.level_count += 1;
        // set item_per_level too;
        let mut next = vec![0];
        next.append(&mut self.items_index_per_level);
        self.items_index_per_level = next;
        let (_last, last_node) = build_tree(
            &mut self.tree_cache,
            input,
            &mut self.items_index_per_level,
            self.level_count,
            true,
            self.leaf_count,
        );
        let next_root = hash_concat(self.root(), &last_node.data);
        self.root = next_root.clone();
        // increase the leaf_count
        self.leaf_count += next_needed_nodes;
        self.is_padded = true;
        let root = Node {
            data: next_root,
            is_left: false,
            from_duplicate: false,
        };
        self.tree_cache.set(PathTrace::root(), root);
        self.unique_leaf_count += 1;
        self.padding_start = self.unique_leaf_count - 1;
        self.is_padded = !self.unique_leaf_count.is_power_of_two();
    }
    pub fn expand_padded(&mut self, data: &Data) {
        let padding_start = self.padding_start;
        // replace the first padded copy with unique pair;
        let hashed_data = hash_data(&data);
        let mut first_padded = PathTrace::new(
            HashDirection::from_index(padding_start),
            self.level_count,
            padding_start,
        );
        if !self.tree_cache.exists(&first_padded) {
            first_padded.index += 1;
        }
        let sibling_path = first_padded.get_sibling_path();
        // if both the current and sibling are the same; and previous contains current,
        // this is a complete dupliate pair, we need to replace it;
        let mut previous = first_padded;
        previous.index -= 2;

        let any_previous_contain_current_hash = [previous, previous.get_sibling_path()]
            .iter()
            .any(|path| self.compare_hashes(&first_padded, path));
        if self.compare_hashes(&first_padded, &sibling_path) {
            if let Some(first_padded_node) = self.tree_cache.get_mut(&first_padded) {
                if any_previous_contain_current_hash {
                    first_padded_node.data = hashed_data.clone();
                }
            }
        }
        // replace the sibling of the first padding_hahsh;
        if let Some(sibling) = self.tree_cache.get_mut(&sibling_path) {
            sibling.data = hashed_data.clone();
            self.cascade_update(first_padded);
        }

        for next_index in (sibling_path.index..self.leaf_count).step_by(2).skip(1) {
            let next_node = PathTrace::new(HashDirection::Right, sibling_path.level, next_index);
            if let Some(found_right) = self.tree_cache.get_mut(&next_node) {
                found_right.data = hashed_data.clone();
                let left = Node {
                    data: hashed_data.clone(),
                    is_left: true,
                    from_duplicate: true,
                };
                self.tree_cache.set(next_node.get_sibling_path(), left);
                self.cascade_update(next_node);
            }
        }
        self.unique_leaf_count += 1;
        self.is_padded = !self.unique_leaf_count.is_power_of_two();

        self.padding_start = if self.is_padded {
            std::cmp::min(self.padding_start + 2, self.unique_leaf_count - 1)
        } else {
            self.unique_leaf_count - 1
        };
    }
    pub fn compare_hashes(&self, left: &PathTrace, right: &PathTrace) -> bool {
        self.tree_cache.get(left).map(|node| &node.data)
            == self.tree_cache.get(right).map(|node| &node.data)
    }
    pub fn get_parent_hash(&self, path_trace: &PathTrace) -> Option<&Hash> {
        // numbers of items of that level is 2^level;
        // our parent is always one level up;
        path_trace
            .get_parent_path()
            .and_then(|path| self.tree_cache.get(&path).map(|node| &node.data))
    }

    pub fn fetch_cache_pathtrace(&self, target_hash: &Hash) -> Option<PathTrace> {
        self.tree_cache.get_key_by_hash(target_hash)
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
                    .map(|node| hex::encode(&node.data))
                    .unwrap_or_default()
            );
        });
    }
    /// generate proof for multiple sets of data
    pub fn proof_multiple(&self, input: &[Data]) -> Vec<Proof> {
        input.iter().flat_map(|data| self.prove(data)).collect()
    }

    pub fn find_data_route(&self, data: &Data) -> Vec<PathTrace> {
        // faster path, fetch the path from leaf_set;
        let target_hash = hash_data(data);
        if let Some(trace) = self.tree_cache.get_key_by_hash(&target_hash) {
            return trace.generate_route().collect();
        }
        let target_hash = hash_data(data);
        if let Some(cached_path) = self.fetch_cache_pathtrace(&target_hash) {
            return cached_path.generate_route().collect();
        }
        vec![]
    }

    /// Verifies that the given input data produces the given root hash
    pub fn verify(input: &[Data], root_hash: &Data, store: Store) -> bool {
        let generated_tree = Self::construct(input, store);
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
        generated == root_hash
    }
    /// appends multiple leaves to our tree
    pub fn append_multiple(&mut self, input: &[Data]) {
        input.iter().for_each(|data| self.append(data));
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
                        .map(|node| (path.level, path.direction, &node.data))
                })
                .collect();

            return Some(Proof { hashes });
        }
        None
    }

    pub fn pretty_print(&self) {
        let mut nodes_per_level: BTreeMap<PathTrace, Vec<(PathTrace, &Node)>> = BTreeMap::new();
        self.tree_cache.entries().for_each(|(path, node)| {
            if let Some(parent) = path.get_parent_path() {
                let entry = nodes_per_level.entry(parent).or_default();
                entry.push((*path, node));
            }
        });
        println!("-----------------Tree-----------------------------");
        for (parent, mut nodes) in nodes_per_level {
            if let Some(parent_hash) = self
                .tree_cache
                .get(&parent)
                .map(|node| hex::encode(&node.data))
            {
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
                nodes.into_iter().for_each(|(path, node)| {
                    let is_left = path.direction == HashDirection::Left;
                    let prefix = "    ";
                    let branch = if is_left { "├── " } else { "└── " };
                    println!(
                        "{}{}{:?} {}: {}",
                        branch,
                        prefix,
                        path.direction,
                        path.index,
                        hex::encode(&node.data)
                    );
                });
            }
        }
    }
}

#[cfg(test)]
mod merkle_tree {
    use super::*;
    use indexmap::IndexMap;
    #[test]
    fn test_constructions() {
        let data = example_data(4);
        let store = IndexMap::new();
        let tree = MerkleTree::construct(&data, store);
        let expected_root = "9675e04b4ba9dc81b06e81731e2d21caa2c95557a85dcfa3fff70c9ff0f30b2e";
        assert_eq!(hex::encode(tree.root()), expected_root);

        //Uncomment if your implementation allows for unbalanced trees
        let data = example_data(3);
        let store = IndexMap::new();
        let tree = MerkleTree::construct(&data, store);
        // using dupliate padding of the last value
        // (correction here) let expected_root = "773a93ac37ea78b3f14ac31872c83886b0a0f1fec562c4e848e023c889c2ce9f";
        let expected_root = "f2dcdd96791b6bac5d554f2d320e594b834f5da1981812c3707e7772234cb0ad";
        assert_eq!(hex::encode(tree.root()), expected_root);

        let data = example_data(8);
        let store = IndexMap::new();
        let tree = MerkleTree::construct(&data, store);
        let expected_root = "0727b310f87099c1ba2ec0ba408def82c308237c8577f0bdfd2643e9cc6b7578";
        assert_eq!(hex::encode(tree.root()), expected_root);
    }
    #[test]
    fn test_verifing_proof() {
        (1_usize..100).for_each(|index| {
            let data = example_data(index);
            let store = IndexMap::new();
            let tree = MerkleTree::construct(&data, store);
            let root_hash = tree.root();

            for (h1, proof) in data.iter().zip(tree.proof_multiple(&data)) {
                assert!(MerkleTree::<IndexMap<PathTrace, Node>>::verify_proof(
                    h1, &proof, root_hash
                ))
            }
        });
    }
    #[test]
    fn append_data_to_balanced_items() {
        let extra = vec![100];
        (1_usize..100)
            .filter(|i| i.is_power_of_two())
            .for_each(|index| {
                let data = example_data(index);
                let store = IndexMap::new();
                let mut tree = MerkleTree::construct(&data, store);
                tree.append(&extra);
                let root_hash = tree.root();
                if let Some(proof) = tree.prove(&extra) {
                    assert!(MerkleTree::<IndexMap<_, _>>::verify_proof(
                        &extra, &proof, root_hash
                    ))
                }
            });
    }
    #[test]
    fn append_multiple_to_balanced_tree() {
        (1_usize..100)
            .filter(|i| i.is_power_of_two())
            .for_each(|index| {
                let data = example_data(index);
                let store = IndexMap::new();
                let mut tree = MerkleTree::construct(&data, store);
                let input: Vec<_> = (112..130).map(|d| vec![d]).collect();
                tree.append_multiple(&input);
                let root_hash = tree.root();
                for h in input.iter() {
                    if let Some(proof) = tree.prove(h) {
                        assert!(MerkleTree::<IndexMap<_, _>>::verify_proof(
                            h, &proof, root_hash
                        ))
                    }
                }
            });
    }
    #[test]
    fn append_multiple_to_un_balanced_tree() {
        (1_usize..100).for_each(|index| {
            let data = example_data(index);
            let store = IndexMap::new();
            let mut tree = MerkleTree::construct(&data, store);
            let input: Vec<_> = (112..130).map(|d| vec![d]).collect();
            tree.append_multiple(&input);
            let root_hash = tree.root();
            for h in input.iter() {
                if let Some(proof) = tree.prove(h) {
                    assert!(MerkleTree::<IndexMap<_, _>>::verify_proof(
                        h, &proof, root_hash
                    ))
                }
            }
        });
    }
    #[test]
    fn append_data_to_unbalanced() {
        let extra = vec![100];
        (1_usize..100).for_each(|index| {
            let data = example_data(index);
            let store = IndexMap::new();
            let mut tree = MerkleTree::construct(&data, store);
            tree.append(&extra);
            let root_hash = tree.root();
            if let Some(proof) = tree.prove(&extra) {
                assert!(MerkleTree::<IndexMap<_, _>>::verify_proof(
                    &extra, &proof, root_hash
                ))
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
            let store = IndexMap::new();
            assert!(MerkleTree::verify(&data, &root_hash, store))
        }
    }
    #[test]
    //#[ignore]
    fn update_works() {
        let data = example_data(4);
        let store = IndexMap::new();
        let mut tree = MerkleTree::construct(&data, store);
        // update the first node at index 0, left to a 5;
        let update = vec![5];
        tree.pretty_print();
        tree.update(&hash_data(&vec![0]), hash_data(&update));
        tree.pretty_print();
        assert_eq!(
            tree.tree_cache
                .get(&PathTrace::new(HashDirection::Left, 2, 0))
                .unwrap()
                .data,
            hash_data(&update)
        );
        // generate prove for update data used;
        let root_hash = tree.root();
        if let Some(proof) = tree.prove(&update) {
            assert!(MerkleTree::<IndexMap<_, _>>::verify_proof(
                &update, &proof, root_hash
            ));
        }
    }
}
