use super::{hash_concat, max_index_at_level_reversed, HashDirection, Node, NodeStore, PathTrace};
use crossbeam_queue::SegQueue;
pub fn build_tree<S: NodeStore + Send>(
    tree_cache: &mut S,
    input: impl IntoIterator<Item = Node>,
    items_index_per_level: &mut [usize],
    level_count: usize,
    is_rebuild: bool,
    last_index: usize,
) -> (PathTrace, Node) {
    let nodes: Vec<(PathTrace, Node)> = input
        .into_iter()
        .enumerate()
        .map(|(mut index, data)| {
            index += last_index;
            let direction = HashDirection::from_index(index);
            let path = PathTrace::new(direction, level_count, index);

            (path, data)
        })
        .collect();
    let leaf_count = if is_rebuild {
        last_index + nodes.len()
    } else {
        nodes.len()
    };
    let parallelize = level_count > 14;
    if parallelize {
        let generated: SegQueue<(PathTrace, Node)> = SegQueue::new();
        let result = build_parallel(&nodes, is_rebuild, &generated);
        drop(nodes);
        for (path, node) in generated {
            tree_cache.set(path, node);
        }
        tree_cache.trigger_batch_actions();

        return result;
    }

    build_sequential(
        tree_cache,
        nodes,
        items_index_per_level,
        level_count,
        leaf_count,
        is_rebuild,
    )
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
            let (right, right_node) = cursor.next().unwrap_or((left, node));

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
                is_leaf: false,
                from_duplicate: node.from_duplicate,
            };
            let parent = PathTrace::new(direction, level, *parent_index);
            tree_cache.set(left, node);
            tree_cache.set(right, right_node);
            tree_cache.set(parent, parent_node);
            *parent_index = std::cmp::min(*parent_index + 1, max_index);
            next_level.push((parent, parent_node));
        }
        nodes = next_level;
    }
    tree_cache.trigger_batch_actions();
    nodes.pop().unwrap_or_default()
}

/// build the tree in parallel using divide and conquer
fn build_parallel(
    nodes: &[(PathTrace, Node)],
    is_rebuild: bool,
    output_buffer: &SegQueue<(PathTrace, Node)>,
) -> (PathTrace, Node) {
    if nodes.len() == 1 {
        if let Some(last) = nodes.last() {
            output_buffer.push((last.0, last.1));
            return *last;
        }
    }

    let mid = nodes.len() / 2;
    let (left_slice, right_slice) = nodes.split_at(mid);

    let (left_result, right_result) = rayon::join(
        || build_parallel(left_slice, is_rebuild, output_buffer),
        || build_parallel(right_slice, is_rebuild, output_buffer),
    );

    let (left_path, left_node) = left_result;
    let (_right_path, right_node) = right_result;

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
        is_leaf: false,
        from_duplicate: left_node.from_duplicate || right_node.from_duplicate,
    };
    let parent_path = PathTrace::new(direction, parent_level_in_tree, parent_index);

    output_buffer.push((parent_path, parent_node));

    (parent_path, parent_node)
}
