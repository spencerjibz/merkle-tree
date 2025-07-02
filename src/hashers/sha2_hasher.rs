use super::Hasher;
use crate::Hash;
use sha2::Digest;
#[derive(Debug, Clone, Copy)]
pub struct Sha2Hasher;

impl Hasher for Sha2Hasher {
    fn hash_data<T: AsRef<[u8]>>(data: &T) -> Hash {
        let hash = sha2::Sha256::digest(data.as_ref());
        hash.into()
    }
}
