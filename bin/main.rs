use merkle_tree::{
    hash_data,
    stores::{create_bytes_stream, TreeCache},
    MerkleTree,
};
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profile = dhat::Profiler::new_heap();
    let now = std::time::Instant::now();
    append_multiple_to_un_balanced_tree();
    println!("{:?}", now.elapsed());
}
fn append_multiple_to_un_balanced_tree() {
    let index = 3;
    use indexmap::IndexMap;
    let store = IndexMap::new();
    let data = create_bytes_stream(index);
    let mut tree = MerkleTree::from_iter(data, index, store);
    let input: Vec<_> = (80..=85).map(|d| vec![d]).collect();
    for (i, h) in input.iter().enumerate() {
        tree.append(h);
        tree.pretty_print();
        let selected = hex::encode(hash_data(h));
        let header_bars = "-".repeat(selected.len() / 4);
        println!("{} Path to h{i} {} {}", header_bars, &selected, header_bars);
        tree.print_data_route(h);
        let root_hash = tree.root();
        if let Some(proof) = tree.prove(h) {
            assert!(MerkleTree::<TreeCache>::verify_proof(h, &proof, root_hash))
        }
    }
    tree.pretty_print();
}
