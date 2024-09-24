//! General utilities for [Bones] meta-engine crates.
//!
//! [Bones]: https://fishfolk.org/development/bones/introduction/
//!
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]

mod collections;
mod default;
#[cfg(feature = "ulid")]
mod labeled_id;
mod names;
#[cfg(feature = "turborand")]
mod random;
#[cfg(feature = "ulid")]
mod ulid;

/// Helper to export the same types in the crate root and in the prelude.
macro_rules! pub_use {
    () => {
        #[cfg(feature = "turborand")]
        pub use crate::random::*;
        pub use crate::{collections::*, default::*, names::*};
        #[cfg(feature = "ulid")]
        pub use crate::{labeled_id::*, ulid::*};
        pub use bones_utils_macros::*;
    };
}
pub_use!();

/// The prelude.
pub mod prelude {
    pub_use!();
}
