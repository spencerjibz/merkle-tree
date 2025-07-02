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
                           │    H7│
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

pub mod hashers;
pub mod stores;
pub mod utils;
use hashers::{GlobalHasher, Hasher};
use std::collections::BTreeMap;
pub use stores::NodeStore;
pub use utils::*;
#[derive(Debug)]
pub struct MerkleTree<Store: NodeStore> {
    root: Hash,
    is_padded: bool,
    pub leaf_count: usize,
    level_count: isize,
    lowest_level: isize,
    unique_leaf_count: usize,
    padding_start: usize,
    // for faster lookup , (level, Direction, index), eg. for H2, (depthlength, Right, index)
    pub tree_cache: Store, // O(1) - path look ups, O(n) search by hash
}

impl<Store: NodeStore + Send> MerkleTree<Store> {
    /// Gets root hash for this tree
    pub fn root(&self) -> &Hash {
        &self.root
    }

    /// Constructs a Merkle tree from given input data
    pub fn construct<B, I, U>(input: I, store: Store) -> Self
    where
        B: AsRef<[u8]> + std::hash::Hash + Eq + Clone,
        I: IntoIterator<IntoIter = U>,
        U: Iterator<Item = B>,
    {
        let input = input.into_iter();
        let size_hint = input.size_hint().1.unwrap_or_default();
        Self::from_iter(input, size_hint, store)
    }
    pub fn from_iter<B, I>(input: I, size_hint: usize, store: Store) -> Self
    where
        B: AsRef<[u8]> + std::hash::Hash + Eq + Clone + Sized,
        I: Iterator<Item = B>,
    {
        let is_padded = !size_hint.is_power_of_two();
        let (leaf_count, input) = pad_input(input, size_hint);
        let level_count = get_level_count(leaf_count);
        let mut tree_cache = store;
        let lowest_level = 0;
        let (_root_path, root, count) =
            build_tree(&mut tree_cache, input, level_count, lowest_level, false, 0);

        let unique_leaf_count = count;
        let padding_start = unique_leaf_count.saturating_sub(1);

        tree_cache.sort();

        Self {
            lowest_level,
            padding_start,
            root: root.data,
            unique_leaf_count,
            is_padded,
            leaf_count,
            level_count,
            tree_cache,
        }
    }
    /// update th target_hash with a new one.
    pub fn update(&mut self, target_hash: &Hash, new: Hash) {
        if let Some(current) = self.fetch_cache_pathtrace(target_hash) {
            if let Some(mut target_node) = self.tree_cache.get(&current) {
                target_node.data = new;
                self.tree_cache.update_value(&current, target_node);
                self.cascade_update(current);
            }
        }
    }

    /// updates the hash up every level to the root.
    pub fn cascade_update(&mut self, current: PathTrace) {
        current.generate_route(self.lowest_level).for_each(|path| {
            if let (Some(current_node), Some(sibling_node)) = (
                self.tree_cache.get(&path),
                self.tree_cache.get(&path.get_sibling_path()),
            ) {
                // check for direction
                let next_parent_hash = if path.direction == HashDirection::Left {
                    GlobalHasher::hash_concat(&current_node.data, &sibling_node.data)
                } else {
                    GlobalHasher::hash_concat(&sibling_node.data, &current_node.data)
                };
                path.get_parent_path(self.lowest_level)
                    .and_then(|parent_path| {
                        if let Some(mut parent_node) = self.tree_cache.get(&parent_path) {
                            parent_node.data = next_parent_hash;
                            self.tree_cache.update_value(&parent_path, parent_node);
                        }
                        None as Option<Node>
                    });
                //parent_node.data = next_parent_hash;
            }
        });
        // update self. root;
        if let Some(new_root) = self.tree_cache.get(&PathTrace::root(self.lowest_level)) {
            self.root = new_root.data;
        }
    }

    pub fn append<D: AsRef<[u8]>>(&mut self, data: &D) {
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
    pub fn expand_tree<D: AsRef<[u8]>>(&mut self, data: &D) {
        // leaf_counts is already a is_power_of_two
        let next_needed_nodes = (self.leaf_count + 1).next_power_of_two() - self.leaf_count;
        let node = Node::new(data, true);
        let input = std::iter::repeat_n(node, next_needed_nodes);
        let total_tree_nodes = 2 * next_needed_nodes - 1;
        // shift_root_to_left
        self.tree_cache.shift_root_to_left(self.lowest_level);
        self.tree_cache.reserve(total_tree_nodes);
        let (_last, last_node, _) = build_tree(
            &mut self.tree_cache,
            input,
            self.level_count,
            self.lowest_level,
            true,
            self.leaf_count,
        );
        let next_root = GlobalHasher::hash_concat(self.root(), &last_node.data);
        self.root = next_root;
        self.leaf_count += next_needed_nodes;
        let root = Node {
            data: next_root,
            is_leaf: false,
            from_duplicate: false,
        };
        self.lowest_level -= 1;
        self.tree_cache
            .set(PathTrace::root(self.lowest_level), root);
        self.tree_cache.trigger_batch_actions();
        self.unique_leaf_count += 1;
        self.padding_start = self.unique_leaf_count - 1;
        self.is_padded = !self.unique_leaf_count.is_power_of_two();
    }
    pub fn expand_padded<D: AsRef<[u8]>>(&mut self, data: &D) {
        let padding_start = self.padding_start;
        // replace the first padded copy with unique pair;
        let hashed_data = GlobalHasher::hash_data(&data);
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
        // this is a complete dupliate pair, we need to replace both;
        let mut previous = first_padded;
        previous.index -= 2;

        let any_previous_contain_current_hash = [previous, previous.get_sibling_path()]
            .iter()
            .any(|path| self.compare_hashes(&first_padded, path));
        if self.compare_hashes(&first_padded, &sibling_path) {
            if let Some(mut first_padded_node) = self.tree_cache.get(&first_padded) {
                if any_previous_contain_current_hash {
                    first_padded_node.data = hashed_data;
                    first_padded_node.from_duplicate = false;
                    self.tree_cache
                        .update_value(&first_padded, first_padded_node);
                    self.tree_cache.trigger_batch_actions();
                }
            }
        }
        // replace the sibling of the first padding_hahsh;
        if let Some(mut sibling) = self.tree_cache.get(&sibling_path) {
            sibling.data = hashed_data;
            self.tree_cache.update_value(&sibling_path, sibling);
            self.tree_cache.trigger_batch_actions();
            self.cascade_update(first_padded);
        }

        for next_index in (sibling_path.index..self.leaf_count).step_by(2).skip(1) {
            let next_node = PathTrace::new(HashDirection::Right, sibling_path.level, next_index);
            if let Some(mut found_right) = self.tree_cache.get(&next_node) {
                found_right.data = hashed_data;
                self.tree_cache.update_value(&next_node, found_right);
                let left = Node {
                    data: hashed_data,
                    is_leaf: true,
                    from_duplicate: true,
                };
                self.tree_cache.set(next_node.get_sibling_path(), left);
                self.cascade_update(next_node);
            }
        }
        self.tree_cache.trigger_batch_actions();
        self.unique_leaf_count += 1;
        self.padding_start = if self.is_padded {
            std::cmp::min(self.padding_start + 2, self.unique_leaf_count)
        } else {
            self.unique_leaf_count - 1
        };
        self.is_padded = !self.unique_leaf_count.is_power_of_two();
    }
    pub fn compare_hashes(&self, left: &PathTrace, right: &PathTrace) -> bool {
        self.tree_cache.get(left).map(|node| node.data)
            == self.tree_cache.get(right).map(|node| node.data)
    }
    pub fn get_parent_hash(&self, path_trace: &PathTrace) -> Option<Hash> {
        // numbers of items of that level is 2^level;
        // our parent is always one level up;
        path_trace
            .get_parent_path(self.lowest_level)
            .and_then(|path| self.tree_cache.get(&path).map(|node| node.data))
    }

    pub fn fetch_cache_pathtrace(&self, target_hash: &Hash) -> Option<PathTrace> {
        self.tree_cache.get_key_by_hash(target_hash)
    }

    pub fn print_data_route<D: AsRef<[u8]>>(&self, data: &D) {
        let route = self.find_data_route(data);
        route.iter().for_each(|path| {
            println!(
                "level {}:  {:?} Node {} -> {}",
                path.level,
                path.direction,
                path.index,
                self.tree_cache
                    .get(path)
                    .map(|node| hex::encode(node.data))
                    .unwrap_or_default()
            );
        });
    }
    /// generate proof for multiple sets of data
    pub fn proof_multiple<D: AsRef<[u8]>>(&self, input: &[D]) -> Vec<Proof> {
        input.iter().flat_map(|data| self.prove(data)).collect()
    }

    pub fn find_data_route<D: AsRef<[u8]>>(&self, data: &D) -> Vec<PathTrace> {
        // faster path, fetch the path from leaf_set;
        let target_hash = GlobalHasher::hash_data(data);
        if let Some(trace) = self.tree_cache.get_key_by_hash(&target_hash) {
            return trace.generate_route(self.lowest_level).collect();
        }
        let target_hash = GlobalHasher::hash_data(data);
        if let Some(cached_path) = self.fetch_cache_pathtrace(&target_hash) {
            return cached_path.generate_route(self.lowest_level).collect();
        }
        vec![]
    }

    /// Verifies that the given input data produces the given root hash
    pub fn verify<D: AsRef<[u8]> + Eq + Clone + std::hash::Hash, I, U, B>(
        input: I,
        root_hash: &D,
        store: Store,
    ) -> bool
    where
        I: IntoIterator<IntoIter = U>,
        U: Iterator<Item = B> + Clone,
        B: AsRef<[u8]> + std::hash::Hash + Eq + Clone,
    {
        let root_hash = root_hash.as_ref();
        let generated_tree = Self::construct(input, store);
        generated_tree.root() == root_hash
    }

    /// Verifies that the given data and proof_path correctly produce the given root_hash
    pub fn verify_proof<D: AsRef<[u8]>>(data: &D, proof: &Proof, root_hash: &Hash) -> bool {
        let hashed_data = GlobalHasher::hash_data(data);
        let generated =
            proof
                .hashes
                .iter()
                .fold(hashed_data, |acc, &(_, direction, ref next_hash)| {
                    let is_leaf = direction == HashDirection::Left;
                    if is_leaf {
                        return GlobalHasher::hash_concat(next_hash, &acc);
                    }
                    GlobalHasher::hash_concat(&acc, next_hash)
                });
        &generated == root_hash
    }

    /// Returns a list of hashes that can be used to prove that the given data is in this tree
    pub fn prove<D: AsRef<[u8]>>(&self, data: &D) -> Option<Proof> {
        // we use our tree_cache and some math to calculate the sibling_node at each parent level
        // See PathTrace for math
        let target_hash = GlobalHasher::hash_data(&data);
        if let Some(trace) = self.fetch_cache_pathtrace(&target_hash) {
            let hashes: Vec<_> = trace
                .generate_route(self.lowest_level)
                .take_while(|path| path.level >= (self.lowest_level + 1)) // ingnore the root
                .flat_map(|mut path| {
                    path = path.get_sibling_path();
                    self.tree_cache
                        .get(&path)
                        .map(|node| (path.level, path.direction, node.data))
                })
                .collect();

            return Some(Proof { hashes });
        }
        None
    }

    pub fn pretty_print(&self) {
        let mut nodes_per_level: BTreeMap<PathTrace, Vec<(PathTrace, Node)>> = BTreeMap::new();

        self.tree_cache.entries().for_each(|(path, node)| {
            if let Some(parent) = path.get_parent_path(self.lowest_level) {
                let entry = nodes_per_level.entry(parent).or_default();
                entry.push((path, node));
            }
        });
        println!("-----------------Tree-----------------------------");
        for (parent, mut nodes) in nodes_per_level {
            if let Some(parent_hash) = self
                .tree_cache
                .get(&parent)
                .map(|node| hex::encode(node.data))
            {
                let header = if parent.level == self.lowest_level {
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
                    let is_leaf = path.direction == HashDirection::Left;
                    let prefix = "    ";
                    let branch = if is_leaf { "├── " } else { "└── " };
                    println!(
                        "{}{}{:?} {}: {}",
                        branch,
                        prefix,
                        path.direction,
                        path.index,
                        hex::encode(node.data)
                    );
                });
            }
        }
    }
}
