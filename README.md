### Merkle Tree 

[![CI](https://github.com/spencerjibz/merkle-tree/actions/workflows/ci.yaml/badge.svg)](https://github.com/spencerjibz/merkle-tree/actions/workflows/ci.yaml)

A minimal and extensible Merkle Tree implementation in Rust.
### Features
- Supports both full and sparse Merkle Trees  
- Computes Merkle root hashes from arbitrary data  
- Generates cryptographic inclusion proofs for any node  
- Verifies proofs efficiently against the root hash  
- Lazy path generation for efficient proof construction  
### Getting Started
### Prerequisites
- Rust (via [https://rustup.rs](https://rustup.rs))

### Build and Run

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

### License
MIT License. See the [LICENSE](LICENSE) file for details.
