use crate::{Hash, Hasher};

use sha3::Digest;
#[derive(Debug, Clone, Copy)]
pub struct KeccakHasher;

impl Hasher for KeccakHasher {
    fn hash_data<T: AsRef<[u8]>>(data: &T) -> Hash {
        let hash = sha3::Sha3_256::digest(data.as_ref());
        hash.into()
    }
}
