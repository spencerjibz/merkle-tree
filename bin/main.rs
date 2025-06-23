use merkle_tree::{hash_data, stores::create_large_input_byes_sled, MerkleTree};
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
    use merkle_tree::stores::SledStore;
    use sled::{Config, Mode};
    let config = Config::default().temporary(true).mode(Mode::HighThroughput);
    let index = 3;
    let db = config.open().unwrap();
    let store = SledStore::new(&db, "test_db").unwrap();
    /*  uncomment to use index map
    use indexmap::IndexMap;
    *
    let store = IndexMap::new();
    *  */
    let (size, data) = create_large_input_byes_sled(index, &db);
    let mut tree = MerkleTree::from_iter(data, size, store);
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
            assert!(MerkleTree::<SledStore>::verify_proof(h, &proof, root_hash))
        }
    }
    tree.pretty_print();
}
