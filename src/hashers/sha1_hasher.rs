use crate::{Hash, Hasher};
use sha1::{Digest, Sha1};
#[derive(Debug, Clone, Copy)]
pub struct Sha1Hasher;
impl Hasher for Sha1Hasher {
    fn hash_data<T: AsRef<[u8]>>(data: &T) -> Hash {
        let mut output = [0; 32];
        let mut hasher = Sha1::new();
        hasher.update(data.as_ref());
        let hash = hasher.finalize();
        output[0..20].copy_from_slice(&hash);
        output
    }
}
