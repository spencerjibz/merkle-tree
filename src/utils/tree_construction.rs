use super::{HashDirection, Node, NodeStore, PathTrace};
use crate::hashers::{GlobalHasher, Hasher};
use crossbeam_queue::SegQueue;
pub fn build_tree<S: NodeStore + Send>(
    tree_cache: &mut S,
    input: impl IntoIterator<Item = Node>,
    level_count: isize,
    lowest_level: isize,
    is_rebuild: bool,
    last_index: usize,
) -> (PathTrace, Node, usize) {
    let mut previous: Option<Node> = None;
    let mut unique_count = 1;
    let nodes = SegQueue::new();
    input.into_iter().enumerate().for_each(|(mut index, data)| {
        index += last_index;
        let direction = HashDirection::from_index(index);
        let path = PathTrace::new(direction, level_count, index);

        if let Some(prev) = previous {
            if prev.data != data.data {
                unique_count += 1;
            }
        }
        previous.replace(data);
        nodes.push((path, data));
    });
    let parallelize = level_count > 14;
    if parallelize {
        let generated: SegQueue<(PathTrace, Node)> = SegQueue::new();
        let result = build_parallel(nodes, &generated, lowest_level, is_rebuild);
        for (path, node) in generated {
            tree_cache.set(path, node);
        }
        tree_cache.trigger_batch_actions();

        return (result.0, result.1, unique_count);
    }

    let result = build_sequential(tree_cache, nodes, lowest_level, is_rebuild);
    (result.0, result.1, unique_count)
}
// build the tree  sequentiallly
fn build_sequential<S: NodeStore + Send>(
    tree_cache: &mut S,
    mut nodes: SegQueue<(PathTrace, Node)>,
    lowest_level: isize,
    is_rebuild: bool,
) -> (PathTrace, Node) {
    while nodes.len() > 1 {
        //  reduce allocations as length of nodes to process halves at every level up.
        let next_level = SegQueue::new();
        let mut cursor = nodes.into_iter();
        while let Some((left, node)) = cursor.next() {
            let (right, right_node) = cursor.next().unwrap_or((left, node));

            let level = left.level - 1;

            let mut parent_index = left.index / 2;
            let mut direction = HashDirection::from_index(parent_index);
            // when we get the root node
            if level == lowest_level {
                direction = HashDirection::Center;
            }
            // when rebuild, move increase the level-count
            if level == lowest_level && is_rebuild {
                direction = HashDirection::Right;
                parent_index = 1;
            }

            let parent_node = Node {
                data: GlobalHasher::hash_concat(&node.data, &right_node.data),
                is_leaf: false,
                from_duplicate: node.from_duplicate,
            };
            let parent = PathTrace::new(direction, level, parent_index);
            tree_cache.set(left, node);
            tree_cache.set(right, right_node);
            tree_cache.set(parent, parent_node);
            next_level.push((parent, parent_node));
        }
        nodes = next_level;
    }
    tree_cache.trigger_batch_actions();
    nodes.pop().unwrap_or_default()
}

/// build the tree in parallel using divide and conquer
fn build_parallel(
    nodes: SegQueue<(PathTrace, Node)>,
    output_buffer: &SegQueue<(PathTrace, Node)>,
    lowest_level: isize,
    is_rebuild: bool,
) -> (PathTrace, Node) {
    if nodes.len() == 1 {
        if let Some(last) = nodes.pop() {
            output_buffer.push((last.0, last.1));
            return last;
        }
    }

    let mid = nodes.len() / 2;
    let left_slice = SegQueue::new();
    for _ in 0..mid {
        if let Some(value) = nodes.pop() {
            left_slice.push(value);
        }
    }

    let (left_result, right_result) = rayon::join(
        || build_parallel(left_slice, output_buffer, lowest_level, is_rebuild),
        || build_parallel(nodes, output_buffer, lowest_level, is_rebuild),
    );

    let (left_path, left_node) = left_result;
    let (_right_path, right_node) = right_result;

    let parent_hash = GlobalHasher::hash_concat(&left_node.data, &right_node.data);
    let parent_level_in_tree = left_path.level - 1;
    let mut parent_index = left_path.index / 2;

    let mut direction = HashDirection::from_index(parent_index);
    if parent_level_in_tree == lowest_level {
        direction = HashDirection::Center;
    }
    if parent_level_in_tree == lowest_level && is_rebuild {
        direction = HashDirection::Right;
        parent_index = 1
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
