//! UUID-related utilities such as UUID map and type UUIDs.
//!
//! - [`TypeUlid`] comes from the [`type_uuid`] crate
//! - [`Ulid`] comes from the [`uuid`] crate.
//!
//! [`type_uuid`]: https://docs.rs/type_uuid
//! [`uuid`]: https://docs.rs/uuid

use fxhash::FxHashMap;

pub use type_ulid::{TypeUlid, Ulid};

/// Faster hash map using [`FxHashMap`] and a ULID key.
pub type UlidMap<T> = FxHashMap<Ulid, T>;
