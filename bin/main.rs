use merkle_tree::{example_data, hash_data, MerkleTree};
fn main() {
    let data = example_data(4);
    let index = data.len() - 1;
    let tree = MerkleTree::construct(&data);
    let h1 = &data[index];
    tree.pretty_print();
    let selected = hex::encode(hash_data(h1));
    let header_bars = "-".repeat(selected.len() / 4);
    println!("{} Path to h1 {} {}", header_bars, &selected, header_bars);
    tree.print_data_route(h1);
    println!("{}", "-".repeat(selected.len() + 20));
    println!("prove for {} at index {index}", &selected);
    let proof = tree.prove(h1);
    let root_hash = tree.root();
    if let Some(proof) = proof {
        // verify here;
        let pretty_proof = proof.get_proof_in_hex();
        println!("{pretty_proof:#?}");
        assert!(MerkleTree::verify_proof(h1, &proof, root_hash));
    }
}
