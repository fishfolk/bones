//! Bitset implementation.
//!
//! Bitsets are powered by the [`bitset_core`] crate.
//!
//! [`bitset_core`]: https://docs.rs/bitset_core

use crate::prelude::*;

// 2^32 gives  4 billion concurrent entities for 512MB   of ram per component
// 2^24 gives 16 million concurrent entities for 2MB     of ram per component
// 2^20 gives  1 million concurrent entities for 128KB   of ram per component
// 2^16 gives 65536      concurrent entities for 8KB     of ram per component
// 2^12 gives 4096       concurrent entities for 512B    of ram per component
// SIMD processes 256 bits/entities (32 bytes) at once when comparing bitsets.
#[cfg(feature = "keysize16")]
const BITSET_EXP: u32 = 16;
#[cfg(all(
    feature = "keysize20",
    not(feature = "keysize16"),
    not(feature = "keysize24"),
    not(feature = "keysize32")
))]
const BITSET_EXP: u32 = 20;
#[cfg(all(
    feature = "keysize24",
    not(feature = "keysize16"),
    not(feature = "keysize20"),
    not(feature = "keysize32")
))]
const BITSET_EXP: u32 = 24;
#[cfg(all(
    feature = "keysize32",
    not(feature = "keysize16"),
    not(feature = "keysize20"),
    not(feature = "keysize24")
))]
const BITSET_EXP: u32 = 32;

pub use bitset_core::*;

pub(crate) const BITSET_SIZE: usize = 2usize.saturating_pow(BITSET_EXP);
pub(crate) const BITSET_SLICE_COUNT: usize = BITSET_SIZE / (32 * 8 / 8);

/// The type of bitsets used to track entities in component storages.
/// Mostly used to create caches.
#[derive(Deref, DerefMut, Clone, Debug)]
pub struct BitSetVec(pub Vec<[u32; 8]>);

impl Default for BitSetVec {
    fn default() -> Self {
        create_bitset()
    }
}

impl BitSetVec {
    /// Check whether or not the bitset contains the given entity.
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.bit_test(entity.index() as usize)
    }
}

/// Creates a bitset big enough to contain the index of each entity.
/// Mostly used to create caches.
pub fn create_bitset() -> BitSetVec {
    BitSetVec(vec![[0u32; 8]; BITSET_SLICE_COUNT])
}
