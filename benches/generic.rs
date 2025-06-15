#[macro_use]
extern crate criterion;

use criterion::Criterion;
use indexmap::IndexMap;
use merkle::MerkleTree;
use rand::RngCore;
use ring::digest::{Algorithm, SHA512};
use std::hint::black_box;

static DIGEST: &Algorithm = &SHA512;
use merkle_tree::MerkleTree as OurTree;
/// 2^23,
const BIG_TREE_INPUT_SIZE: usize = 2_usize.pow(10);

fn bench_small_str_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("Build small_str_tree");
    let values = vec!["one", "two", "three", "four"];
    group.bench_function("Merkle.rs::from_vec - small", |b| {
        b.iter(|| MerkleTree::from_vec(DIGEST, black_box(values.clone())))
    });
    group.bench_function("Current::construct - small", |b| {
        let values: Vec<_> = values.iter().map(|s| s.as_bytes().to_vec()).collect();
        b.iter(|| OurTree::construct(black_box(&values), black_box(IndexMap::new())))
    });
    group.finish();
}

fn bench_small_str_proof_gen(c: &mut Criterion) {
    let mut group = c.benchmark_group("Proof for small tree");
    let values = vec!["one", "two", "three", "four"];
    group.bench_function("MerkleTree.rs::gen_proof - small", |b| {
        let tree = MerkleTree::from_vec(DIGEST, values.clone());

        b.iter(|| {
            for value in &values {
                tree.gen_proof(black_box(value));
            }
        })
    });
    group.bench_function("Current::prove - small", |b| {
        let values: Vec<_> = values.iter().map(|s| s.as_bytes().to_vec()).collect();
        let store = IndexMap::new();
        let tree = OurTree::construct(&values, store);

        b.iter(|| {
            for value in &values {
                tree.prove(black_box(value));
            }
        })
    });
    group.finish();
}

fn bench_small_str_proof_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("Proof Check for small tree ");
    let values = vec!["one", "two", "three", "four"];
    group.bench_function("MerkleTree::validate_proof - small", |b| {
        let tree = MerkleTree::from_vec(DIGEST, values.clone());
        let proofs = values
            .iter()
            .map(|v| tree.gen_proof(v).unwrap())
            .collect::<Vec<_>>();

        b.iter(|| {
            for proof in &proofs {
                proof.validate(black_box(tree.root_hash()));
            }
        })
    });
    group.bench_function("Current::verify_proof - small", |b| {
        let values: Vec<_> = values.iter().map(|s| s.as_bytes().to_vec()).collect();
        let store = IndexMap::new();
        let tree = OurTree::construct(&values, store);
        let proofs = tree.proof_multiple(&values);
        let root_hash = tree.root();
        b.iter(|| {
            for (i, proof) in proofs.iter().enumerate() {
                OurTree::<IndexMap<_, _>>::verify_proof(
                    black_box(&values[i]),
                    black_box(proof),
                    black_box(root_hash),
                );
            }
        })
    });
    group.finish();
}

fn bench_big_rnd_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("Build Big tree");
    let mut values = vec![vec![0u8; 256]; BIG_TREE_INPUT_SIZE];
    let mut rng = rand::rng();

    group.sample_size(50);

    for v in &mut values {
        rng.fill_bytes(v);
    }
    group.bench_function("MerkleTree::from_vec - big", |b| {
        b.iter(|| MerkleTree::from_vec(DIGEST, black_box(values.clone())))
    });
    group.bench_function("OurTree::construct - big", |b| {
        b.iter(|| OurTree::construct(black_box(&values), black_box(IndexMap::new())))
    });
    group.finish();
}

fn bench_big_rnd_proof_gen(c: &mut Criterion) {
    let mut group = c.benchmark_group("Generate proof for Large tree inputs");
    let mut values = vec![vec![0u8; 256]; BIG_TREE_INPUT_SIZE];
    let mut rng = rand::rng();

    group.sample_size(50);
    for v in &mut values {
        rng.fill_bytes(v);
    }
    group.bench_function("MerkleTree::gen_proof - big", |b| {
        let tree = MerkleTree::from_vec(DIGEST, values.clone());

        b.iter(|| {
            for value in &values {
                tree.gen_proof(black_box(value.clone()));
            }
        })
    });
    group.bench_function("Current::prove - big", |b| {
        let store = IndexMap::new();
        let tree = OurTree::construct(&values, store);

        b.iter(|| {
            for value in &values {
                tree.prove(black_box(value));
            }
        })
    });
    group.finish();
}

fn bench_big_rnd_proof_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("Proof Verification for Large tree");
    let mut values = vec![vec![0u8; 256]; BIG_TREE_INPUT_SIZE];
    let mut rng = rand::rng();
    group.sample_size(90);
    for v in &mut values {
        rng.fill_bytes(v);
    }

    group.bench_function("MerkleTree::validate_proof - big", |b| {
        let tree = MerkleTree::from_vec(DIGEST, values.clone());
        let proofs = values
            .clone()
            .into_iter()
            .map(|v| tree.gen_proof(v).unwrap())
            .collect::<Vec<_>>();

        b.iter(|| {
            for proof in &proofs {
                proof.validate(black_box(tree.root_hash()));
            }
        })
    });
    group.bench_function("Current::verify_proof - big", |b| {
        let store = IndexMap::new();
        let tree = OurTree::construct(&values, store);
        let proofs = tree.proof_multiple(&values);
        let root_hash = tree.root();
        b.iter(|| {
            for (i, proof) in proofs.iter().enumerate() {
                OurTree::<IndexMap<_, _>>::verify_proof(
                    black_box(&values[i]),
                    black_box(proof),
                    black_box(root_hash),
                );
            }
        })
    });
    group.finish();
}

fn bench_big_rnd_iter(c: &mut Criterion) {
    c.bench_function("MerkleTree::iter - big", |b| {
        let mut values = vec![vec![0u8; 256]; 160];
        let mut rng = rand::rng();

        for v in &mut values {
            rng.fill_bytes(v);
        }

        let tree = MerkleTree::from_vec(DIGEST, values);
        b.iter(|| {
            for value in &tree {
                black_box(value);
            }
        })
    });
}

criterion_group!(
    benches,
    bench_small_str_tree,
    bench_small_str_proof_gen,
    bench_small_str_proof_check,
    bench_big_rnd_tree,
    bench_big_rnd_proof_gen,
    bench_big_rnd_proof_check,
    bench_big_rnd_iter,
);

criterion_main!(benches);
