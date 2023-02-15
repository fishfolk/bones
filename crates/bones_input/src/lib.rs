//! Standardized types meant to be provided to Bones games from the outside environment.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use type_ulid::TypeUlid;

pub mod time;

/// The prelude.
pub mod prelude {
    pub use crate::*;
    pub use time::*;
}

/// Information about the window the game is running in.
#[derive(Clone, Copy, Debug, Default, TypeUlid)]
#[ulid = "01GP70WMVH4HV4YHZ240E0YC7X"]
pub struct Window {
    /// The logical size of the window's client area.
    pub size: glam::Vec2,
}
