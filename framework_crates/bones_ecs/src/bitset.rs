//! Bitset implementation.
//!
//! Bitsets are powered by the [`bitset_core`] crate.
//!
//! A Bones bitset is a vector of 32-byte sectors. The size of a bitset can be controlled by the
//! `keysize*` features (note that they are all mutually exclusive with each other). The `keysize`
//! is exponentially correlated to the number of bits, or entities, that the set can track; where
//! the number of bits equals two raised to the power of `keysize`. Below are the available
//! `keysize`s and the size of their resulting bitsets.
//!
//! | Keysize | Bit Count  | Sectors    | Memory |
//! | ------- | ---------- | ---------- | ------ |
//! |      32 |  4 billion | 16 million | 512 MB |
//! |      24 | 16 million |      65536 |   2 MB |
//! |      20 |  1 million |       4096 | 128 KB |
//! |      16 |      65536 |        256 |   8 KB |
//! |      12 |       4096 |         16 | 512  B |
//! |      10 |       1024 |          4 | 128  B |
//!
//! Keysize (`K`) refers to the name of the feature. A keysize of 16 refers to the `keysize16`
//! feature.
//!
//! Bit Count is the total number of bits in the bitset (`2^K`). Due to the nature of binary
//! numbers this means that a value with more than `K` bits should not be used to index into the
//! bitset since e.g. `u16::MAX` is the index of the last bit in a `keysize16` bitset.
//!
//! Sectors is the number of sub-arrays within the bitset `Vec`. Sectors are 256 bits as they are
//! comprised of 8 `u32`s. Note that SIMD instructions process 256 bits/entities at a time.
//!
//! Memory is the total amount of memory that the bitset will occupy.
//!
//! [`bitset_core`]: https://docs.rs/bitset_core

use crate::prelude::*;

#[cfg(all(
    feature = "keysize10",
    not(feature = "keysize12"),
    not(feature = "keysize16"),
    not(feature = "keysize20"),
    not(feature = "keysize24"),
    not(feature = "keysize32")
))]
#[allow(missing_docs)]
pub const BITSET_EXP: u32 = 10;

#[cfg(all(
    feature = "keysize12",
    not(feature = "keysize10"),
    not(feature = "keysize16"),
    not(feature = "keysize20"),
    not(feature = "keysize24"),
    not(feature = "keysize32")
))]
#[allow(missing_docs)]
pub const BITSET_EXP: u32 = 12;

// 16 is the default, if no `keysize*` features are enabled then use this one.
#[cfg(any(
    feature = "keysize16",
    all(
        not(feature = "keysize10"),
        not(feature = "keysize12"),
        not(feature = "keysize20"),
        not(feature = "keysize24"),
        not(feature = "keysize32")
    )
))]
#[allow(missing_docs)]
pub const BITSET_EXP: u32 = 16;

#[cfg(all(
    feature = "keysize20",
    not(feature = "keysize10"),
    not(feature = "keysize12"),
    not(feature = "keysize16"),
    not(feature = "keysize24"),
    not(feature = "keysize32")
))]
#[allow(missing_docs)]
pub const BITSET_EXP: u32 = 20;

#[cfg(all(
    feature = "keysize24",
    not(feature = "keysize10"),
    not(feature = "keysize12"),
    not(feature = "keysize16"),
    not(feature = "keysize20"),
    not(feature = "keysize32")
))]
#[allow(missing_docs)]
pub const BITSET_EXP: u32 = 24;

#[cfg(all(
    feature = "keysize32",
    not(feature = "keysize10"),
    not(feature = "keysize12"),
    not(feature = "keysize16"),
    not(feature = "keysize20"),
    not(feature = "keysize24")
))]
#[allow(missing_docs)]
pub const BITSET_EXP: u32 = 32;

pub use bitset_core::*;

/// The number of bits in a bitset.
pub(crate) const BITSET_SIZE: usize = 2usize.saturating_pow(BITSET_EXP);

/// The number of bits in a bitset "sector" (one of the sub-arrays).
///
/// A sector is an array of 8 `u32` values, so there are `8 * 32` bits in a sector.
///
/// This value is constant, as a sector is appropriately sized to fit into a vector register for
/// SIMD instructions.
const BITSET_SECTOR_SIZE: usize = 32 * 8;

const BITSET_SECTOR_COUNT: usize = BITSET_SIZE / BITSET_SECTOR_SIZE;

/// The type of bitsets used to track entities in component storages.
/// Mostly used to create caches.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Set an entity on the the bitset.
    #[inline]
    pub fn set(&mut self, entity: Entity) {
        self.bit_set(entity.index() as usize);
    }
}

/// Creates a bitset big enough to contain the index of each entity.
/// Mostly used to create caches.
pub fn create_bitset() -> BitSetVec {
    BitSetVec(vec![[0u32; 8]; BITSET_SECTOR_COUNT])
}
