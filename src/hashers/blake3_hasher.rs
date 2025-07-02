use super::Hasher;
use crate::Hash;
#[derive(Debug, Clone, Copy)]
pub struct Blake3;

impl Hasher for Blake3 {
    fn hash_data<T: AsRef<[u8]>>(data: &T) -> Hash {
        let hash = blake3::hash(data.as_ref());
        hash.into()
    }

    fn hash_concat<T: AsRef<[u8]>>(h1: &T, h2: &T) -> Hash {
        Self::hash_data(&[h1.as_ref(), h2.as_ref()].concat())
    }
}
