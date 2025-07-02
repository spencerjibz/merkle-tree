use super::Hasher;
use crate::Hash;
#[derive(Debug, Clone, Copy)]
pub struct Blake3;

impl Hasher for Blake3 {
    fn hash_data<T: AsRef<[u8]>>(data: &T) -> Hash {
        let hash = blake3::hash(data.as_ref());
        hash.into()
    }
}
