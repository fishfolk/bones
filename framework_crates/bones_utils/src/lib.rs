//! General utilities for [Bones] meta-engine crates.
//!
//! [Bones]: https://fishfolk.org/development/bones/introduction/
//!
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]

mod collections;
mod default;
mod desync_hash;
mod labeled_id;
mod names;
mod random;
mod ulid;

/// Helper to export the same types in the crate root and in the prelude.
macro_rules! pub_use {
    () => {
        pub use crate::{
            collections::*, default::*, desync_hash::*, labeled_id::*, names::*, random::*, ulid::*,
        };
        pub use bones_utils_macros::*;
        pub use branches::{likely, unlikely};
        pub use futures_lite as futures;
        pub use fxhash;
        pub use hashbrown;
        pub use maybe_owned::*;
        pub use parking_lot;
        pub use smallvec::*;
        pub use turborand::*;
        pub use ustr::{ustr, Ustr, UstrMap, UstrSet};
    };
}
pub_use!();

/// The prelude.
pub mod prelude {
    pub_use!();
}
