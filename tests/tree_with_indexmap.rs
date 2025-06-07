#[cfg(test)]
mod tree_with_indexmap_store {
    use indexmap::IndexMap;
    use merkle_tree::{example_data, hash_data, HashDirection, MerkleTree, Node, PathTrace};
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
