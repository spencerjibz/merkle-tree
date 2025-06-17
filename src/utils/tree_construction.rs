use super::{
    hash_concat, hash_data, max_index_at_level_reversed, HashDirection, Node, NodeStore, PathTrace,
};
use itertools::Itertools;
use rayon::prelude::*;
pub fn build_tree<S: NodeStore + Send>(
    tree_cache: &mut S,
    input: impl IntoIterator<Item = Node>,
    items_index_per_level: &mut [usize],
    level_count: usize,
    is_rebuild: bool,
    last_index: usize,
) -> (PathTrace, Node) {
    let parallelize = level_count > 14;
    if parallelize {
        let nodes = input.into_iter().collect_vec();
        let nodes = nodes.into_par_iter();
        let nodes: Vec<_> = nodes
            .enumerate()
            .map(|(mut index, mut data)| {
                index += last_index;
                let hash = hash_data(&data.data);
                let direction = HashDirection::from_index(index);
                let path = PathTrace::new(direction, level_count, index);
                data.data = hash;
                (path, data)
            })
            .collect();

        let result = build_parallel(&nodes, is_rebuild);

        for (path, node) in result.generated_nodes {
            tree_cache.set(path, node);
        }
        tree_cache.trigger_batch_actions();

        return result.root_node;
    }

    let nodes: Vec<(PathTrace, Node)> = input
        .into_iter()
        .enumerate()
        .map(|(mut index, mut data)| {
            index += last_index;
            let hash = hash_data(&data.data);
            let direction = HashDirection::from_index(index);
            let path = PathTrace::new(direction, level_count, index);
            data.data = hash;
            (path, data)
        })
        .collect();
    let leaf_count = if is_rebuild {
        last_index + nodes.len()
    } else {
        nodes.len()
    };
    build_sequential(
        tree_cache,
        nodes,
        items_index_per_level,
        level_count,
        leaf_count,
        is_rebuild,
    )
}
#[derive(Debug)]
struct BuildResult {
    root_node: (PathTrace, Node),
    generated_nodes: Vec<(PathTrace, Node)>,
}
/// build the tree in parallel using divide and conquer
fn build_parallel(nodes: &[(PathTrace, Node)], is_rebuild: bool) -> BuildResult {
    if nodes.len() == 1 {
        let (path, node) = nodes[0].clone();
        return BuildResult {
            root_node: (path, node.clone()),
            generated_nodes: vec![(path, node)],
        };
    }

    let mid = nodes.len() / 2;
    let (left_slice, right_slice) = nodes.split_at(mid);

    let (left_result, right_result) = rayon::join(
        || build_parallel(left_slice, is_rebuild),
        || build_parallel(right_slice, is_rebuild),
    );

    let (left_path, left_node) = left_result.root_node;
    let (_right_path, right_node) = right_result.root_node;

    let parent_hash = hash_concat(&left_node.data, &right_node.data);

    let parent_level_in_tree = left_path.level - 1;

    let parent_index = left_path.index / 2;
    let mut direction = HashDirection::from_index(parent_index);

    if parent_level_in_tree == 0 {
        direction = HashDirection::Center;
    }
    if parent_level_in_tree == 1 && is_rebuild {
        direction = HashDirection::Right;
    }

    let parent_node = Node {
        data: parent_hash,
        is_left: false,
        from_duplicate: left_node.from_duplicate || right_node.from_duplicate,
    };
    let parent_path = PathTrace::new(direction, parent_level_in_tree, parent_index);

    let mut generated_nodes = left_result.generated_nodes;
    generated_nodes.extend(right_result.generated_nodes);
    generated_nodes.push((parent_path, parent_node.clone()));

    BuildResult {
        root_node: (parent_path, parent_node),
        generated_nodes,
    }
}
// build the tree  sequentiallly
fn build_sequential<S: NodeStore + Send>(
    tree_cache: &mut S,
    mut nodes: Vec<(PathTrace, Node)>,
    items_index_per_level: &mut [usize],
    level_count: usize,
    leaf_count: usize,
    is_rebuild: bool,
) -> (PathTrace, Node) {
    while nodes.len() > 1 {
        //  reduce allocations as length of nodes to process halves at every level up.
        let mut next_level = Vec::with_capacity(nodes.len() / 2);
        let mut cursor = nodes.into_iter();
        while let Some((left, node)) = cursor.next() {
            let (right, right_node) = cursor.next().unwrap_or_else(|| (left, node.clone()));

            let level = left.level - 1;

            let max_index = max_index_at_level_reversed(leaf_count, level_count, level);
            let parent_index = items_index_per_level.get_mut(level).unwrap();
            let mut direction = HashDirection::from_index(*parent_index);
            // when we get the root node
            if level == 0 {
                direction = HashDirection::Center;
            }
            // when rebuild, move increase the level-count
            if level == 1 && is_rebuild {
                direction = HashDirection::Right;
                *parent_index = 1;
            }

            let parent_node = Node {
                data: hash_concat(&node.data, &right_node.data),
                is_left: false,
                from_duplicate: node.from_duplicate,
            };
            let parent = PathTrace::new(direction, level, *parent_index);
            tree_cache.set(left, node);
            tree_cache.set(right, right_node);
            tree_cache.set(parent, parent_node.clone());
            *parent_index = std::cmp::min(*parent_index + 1, max_index);
            next_level.push((parent, parent_node));
        }
        nodes = next_level;
    }
    tree_cache.trigger_batch_actions();
    nodes.pop().unwrap_or_default()
}
