//! Standardized rendering components for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

pub mod audio;
pub mod camera;
pub mod color;
pub mod line;
pub mod sprite;
pub mod tilemap;
pub mod transform;

/// The prelude
pub mod prelude {
    pub use {bones_asset::prelude::*, bones_ecs::prelude::*, glam::*};

    pub use crate::{audio::*, camera::*, color::*, line::*, sprite::*, tilemap::*, transform::*};
}
