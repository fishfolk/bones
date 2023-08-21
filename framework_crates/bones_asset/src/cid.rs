/// A unique content ID.
///
/// Represents the Sha-256 hash of the contents of a [`LoadedAsset`][crate::LoadedAsset].
#[derive(Clone, Copy, Eq, PartialEq, Hash, Default, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Cid(pub [u8; 32]);

impl std::fmt::Display for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl std::fmt::Debug for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cid({})", self)
    }
}

impl Cid {
    /// Update the CID by combining it's current data with the hash of the provided bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        use sha2::Digest;
        let pre_hash = self.0;
        let mut hasher = sha2::Sha256::new();
        hasher.update(pre_hash);
        hasher.update(bytes);
        let result = hasher.finalize();
        self.0.copy_from_slice(&result);
    }
}
