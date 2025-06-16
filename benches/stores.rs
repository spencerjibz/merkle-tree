#[macro_use]
extern crate criterion;
use criterion::Criterion;
use itertools::Itertools;
use std::hint::black_box;
const BIG_TREE_INPUT_SIZE: usize = 2_usize.pow(15);
use indexmap::IndexMap;
use merkle_tree::stores::{
    create_large_input_byes_rockdb, temporary_rocks_db, FjallDbStore, RocksDb, RocksDbStore,
    SledStore,
};
use merkle_tree::{stores::temporary_fjall_db, MerkleTree};

use fjall::Keyspace;
use rand::prelude::*;
use sled::{Config, Db, Mode};
use std::sync::LazyLock;
static SLED_DB: LazyLock<Db> = LazyLock::new(|| {
    let config = Config::new()
        .temporary(true)
        .mode(Mode::HighThroughput)
        .print_profile_on_drop(true);
    config.open().unwrap()
});
static ROCKS_DB: LazyLock<RocksDb> = LazyLock::new(temporary_rocks_db);
static FJALL_DB: LazyLock<Keyspace> = LazyLock::new(temporary_fjall_db);

fn tree_construction_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("Small Tree construction with stores");
    group.sample_size(25);
    let mut rng = rand::rng();
    let (_, values) = create_large_input_byes_rockdb(2_usize.pow(5), &ROCKS_DB);
    let values = values.collect_vec();
    group.bench_function("IndexMap Store", |b| {
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(IndexMap::new())))
    });
    group.bench_function("SledStore - sled_db", |b| {
        let store = SledStore::new(&SLED_DB, "tree_construction").unwrap();
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(store.clone())))
    });

    group.bench_function("RocksDbStore - rocksdb", |b| {
        let store_name: i32 = rng.random();
        let store =
            RocksDbStore::new(&ROCKS_DB, &format!("tree_construction - {store_name}")).unwrap();
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(store.clone())))
    });
    group.bench_function("FjallDbStore - fjall", |b| {
        let store_name: i32 = rng.random();
        let store =
            FjallDbStore::new(&FJALL_DB, format!("tree_construction-{store_name}")).unwrap();
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(store.clone())))
    });
    group.finish();
}
fn tree_construction_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("Large Tree construction with stores");
    group.sample_size(25);
    let mut rng = rand::rng();
    let (_, values) = create_large_input_byes_rockdb(BIG_TREE_INPUT_SIZE, &ROCKS_DB);
    let values = values.collect_vec();
    group.bench_function("IndexMap Store", |b| {
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(IndexMap::new())))
    });
    group.bench_function("SledStore - sled_db", |b| {
        let store = SledStore::new(&SLED_DB, "tree_construction").unwrap();
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(store.clone())))
    });

    group.bench_function("ROCKSDbStore - rocksdb", |b| {
        let store_name: i32 = rng.random();
        let store =
            RocksDbStore::new(&ROCKS_DB, &format!("tree_construction-{store_name}")).unwrap();
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(store.clone())))
    });
    group.bench_function("FjallDbStore - fjall", |b| {
        let store_name: i32 = rng.random();
        let store =
            FjallDbStore::new(&FJALL_DB, format!("tree_construction-{store_name}")).unwrap();
        b.iter(|| MerkleTree::construct(black_box(&values), black_box(store.clone())))
    });
    group.finish();
}

criterion_group!(benches, tree_construction_small, tree_construction_large,);
criterion_main!(benches);
