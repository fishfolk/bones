//! General utilities for [Bones] meta-engine crates.
//!
//! [Bones]: https://fishfolk.org/development/bones/introduction/
//!
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]

mod collections;
mod default;
mod key_mod;
mod labeled_id;
mod names;
mod ptr;

/// Helper to export the same types in the crate root and in the prelude.
macro_rules! pub_use {
    () => {
        pub use crate::{collections::*, default::*, key_mod::*, labeled_id::*, names::*, ptr::*};
        pub use bevy_ptr::*;
        pub use bones_utils_macros::*;
        pub use hashbrown;
        pub use maybe_owned::*;
        pub use parking_lot;
        pub use fxhash;
    };
}
pub_use!();

/// The prelude.
pub mod prelude {
    pub_use!();
    pub use crate::key;
}

/// Create a new const [`Key`][key_mod::Key] parsed at compile time.
#[macro_export]
macro_rules! key {
    ($s:literal) => {{
        const KEY: Key = match Key::new($s) {
            Ok(key) => key,
            Err(KeyError::TooLong) => panic!("Key too long"),
            Err(KeyError::NotAscii) => panic!("Key not ascii"),
        };
        KEY
    }};
}
