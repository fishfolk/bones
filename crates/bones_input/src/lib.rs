//! Standardized types meant to be provided to Bones games from the outside environment.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use type_ulid::TypeUlid;

/// The prelude.
pub mod prelude {
    pub use crate::*;
}

/// Resource representing the current game time.
#[derive(Clone, Copy, Debug, TypeUlid, Default)]
#[ulid = "01GNR4DNDZRH0E9XCSV79WRGXH"]
pub struct Time {
    /// The time elapsed since the start of the game session.
    pub elapsed: f32,
}

/// Information about the window the game is running in.
#[derive(Clone, Copy, Debug, Default, TypeUlid)]
#[ulid = "01GP70WMVH4HV4YHZ240E0YC7X"]
pub struct Window {
    /// The logical size of the window's client area.
    pub size: glam::Vec2,
}
