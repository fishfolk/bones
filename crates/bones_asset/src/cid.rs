/// A unique content ID.
///
/// Represents the Sha-256 hash of the contents of a [`LoadedAsset`][crate::LoadedAsset].
#[derive(Clone, Copy, Eq, PartialEq, Hash, Default)]
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
