//! Standardized rendering components for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

pub mod camera;
pub mod sprite;
pub mod tilemap;
pub mod transform;

/// The prelude
pub mod prelude {
    pub use {bones_asset::prelude::*, bones_ecs::prelude::*, glam::*, type_ulid::TypeUlid};

    pub use crate::{camera::*, sprite::*, tilemap::*, transform::*};
}
