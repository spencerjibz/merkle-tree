use merkle_tree::{
    hashers::{GlobalHasher, Hasher},
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
    let _store: TreeCache = IndexMap::with_capacity(2 * index - 1);
    #[cfg(feature = "sled")]
    use merkle_tree::stores::{temporary_sled_db, SledStore};
    #[cfg(feature = "sled")]
    let db = temporary_sled_db();
    #[cfg(feature = "sled")]
    let _store = SledStore::new(&db, "test_db").expect("failed to create store");
    //
    #[cfg(feature = "rocksdb")]
    use merkle_tree::stores::{temporary_rocks_db, RocksDbStore};
    #[cfg(feature = "rocksdb")]
    let db = temporary_rocks_db();
    #[cfg(feature = "rocksdb")]
    let _store = RocksDbStore::new(&db, "test_db").expect("failed to create store");

    #[cfg(feature = "fjall")]
    use merkle_tree::stores::{temporary_fjall_db, FjallDbStore};
    #[cfg(feature = "fjall")]
    let db = temporary_fjall_db();
    #[cfg(feature = "fjall")]
    let _store = FjallDbStore::new(&db, "test_db").expect("failed to create store");
    let data = create_bytes_stream(index);
    let mut tree = MerkleTree::from_iter(data, index, _store);
    let input: Vec<_> = (80..=87).map(|d| vec![d]).collect();
    for (i, h) in input.iter().enumerate() {
        tree.append(h);
        tree.pretty_print();
        let selected = hex::encode(GlobalHasher::hash_data(h));
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
