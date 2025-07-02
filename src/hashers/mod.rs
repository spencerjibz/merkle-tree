use crate::Hash;
#[cfg(feature = "sha2")]
mod sha2_hasher;

#[cfg(feature = "blake3")]
mod blake3_hasher;
#[cfg(feature = "sha1")]
mod sha1_hasher;
pub trait Hasher: Send + Clone + Copy {
    // ------------------------- UTILITY FUNCTIONS --------------------------------------------------
    fn hash_data<T: AsRef<[u8]>>(data: &T) -> Hash;

    fn hash_concat<T: AsRef<[u8]>>(h1: &T, h2: &T) -> Hash {
        Self::hash_data(&[h1.as_ref(), h2.as_ref()].concat())
    }
}
#[cfg(feature = "sha2")]
pub type GlobalHasher = sha2_hasher::Sha2Hasher;
#[cfg(all(feature = "blake3", not(feature = "sha2")))]
pub type GlobalHasher = blake3_hasher::Blake3;
#[cfg(all(feature = "sha1", not(feature = "sha2"), not(feature = "blake3")))]
pub type GlobalHasher = sha1_hasher::Sha1Hasher;
