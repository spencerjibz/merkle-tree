### Merkle Tree

[![CI](https://github.com/spencerjibz/merkle-tree/actions/workflows/ci.yaml/badge.svg)](https://github.com/spencerjibz/merkle-tree/actions/workflows/ci.yaml)

A minimal and extensible Merkle Tree implementation in Rust.

### Features

- Supports both balanced and unbalanced Merkle Trees
- Computes Merkle root hashes from arbitrary data
- Generates cryptographic inclusion proofs for any node
- Verifies proofs efficiently against the root hash
- Lazy path generation for efficient proof construction
- Extensible node storage options (implement the NodeStore trait)
- Sled as a node storage option
- RocksDB integration
- Fjall integration
- Support for incremental updates and appending to the tree;

### Getting Started

### Prerequisites

- Rust (via [https://rustup.rs](https://rustup.rs))

### Build and Run an example

```bash
git clone https://github.com/spencerjibz/merkle-tree.git
cd merkle-tree
cargo run
```

Find a usage example in [bin](bin/main.rs)

### Tests

Run the test suite with:

```bash
cargo test
```

Tests cover core functionality including tree construction, root computation, proof generation, and verification.

### Benchmarks

[Here](https://spencerjibz.github.io/merkle-tree) are basic benchmarks for construction, proof_generation and verification between our implementation & [merkle.rs](https://github.com/SpinResearch/merkle.rs) and benchmarks between stores (RocksDb, Sled, IndexMap and Fjall).

run ` cargo bench` to run the benchmarks locally.

### License

MIT License. See the [LICENSE](LICENSE) file for details.
