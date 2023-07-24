//! Standardized types meant to be provided to Bones games from the outside environment.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

mod time;
mod window;

/// Helper to export the same types in the crate root and in the prelude.
macro_rules! pub_use {
    () => {
        pub use crate::{time::*, window::*};
    };
}
pub_use!();

/// The prelude.
pub mod prelude {
    pub_use!();
}
