use merkle_tree::{example_data, hash_data, MerkleTree};
fn main() {
    let now = std::time::Instant::now();
    /* let data = example_data(100000);
        let index = data.len() - 1;
        let tree = MerkleTree::construct(&data);
        let h1 = &data[index];
        //tree.pretty_print();
        let selected = hex::encode(hash_data(h1));
        let header_bars = "-".repeat(selected.len() / 4);
        println!("{} Path to h1 {} {}", header_bars, &selected, header_bars);
        //tree.print_data_route(h1);
        println!("{}", "-".repeat(selected.len() + 20));
        println!("prove for {} at index {index}", &selected);
        let proof = tree.prove(h1);
        let root_hash = tree.root();
        if let Some(proof) = proof {
            // verify here;
            let pretty_proof = proof.get_proof_in_hex();
            //println!("{pretty_proof:#?}");
            assert!(MerkleTree::verify_proof(h1, &proof, root_hash));
        }
    */
    append_multiple_to_un_balanced_tree();
    println!("{:?}", now.elapsed());
}
fn append_multiple_to_un_balanced_tree() {
    let index = 3;
    let data = example_data(index);
    let mut tree = MerkleTree::construct(&data);
    let input: Vec<_> = (80..=85).map(|d| vec![d]).collect();
    //tree.append_multiple(&input);
    for (i, h) in input.iter().enumerate() {
        tree.append(h);
        tree.pretty_print();
        let selected = hex::encode(hash_data(h));
        let header_bars = "-".repeat(selected.len() / 4);
        println!("{} Path to h{i} {} {}", header_bars, &selected, header_bars);
        tree.print_data_route(h);
        let root_hash = tree.root();
        if let Some(proof) = tree.prove(h) {
            assert!(MerkleTree::verify_proof(h, &proof, root_hash))
        }
    }
    tree.pretty_print();
}
