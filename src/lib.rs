//! Opinionated game meta-engine built on Bevy.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

#[doc(inline)]
pub use {bones_ecs as ecs, bones_input as input, bones_render as render, bones_utils as utils};

/// Bones lib prelude
pub mod prelude {
    pub use crate::{
        animation::prelude::*, asset::prelude::*, camera::*, ecs::prelude::*, game::*,
        input::prelude::*, render::prelude::*, utils::prelude::*, FrameTime,
    };
}
use prelude::*;

pub mod animation;
pub mod asset;
pub mod camera;
pub mod game;

/// This is a resource that stores the game's fixed frame time.
///
/// For instance, if the game logic is meant to run at a fixed frame rate of 60 fps, then this
/// should be `1.0 / 60.0`.
///
/// This resource is used by animation or other timing-sensitive code when running code that should
/// run the same, regardless of the games fixed updates-per-second.
#[derive(Clone, HasSchema, Deref, DerefMut)]
#[repr(C)]
pub struct FrameTime(pub f32);

impl Default for FrameTime {
    fn default() -> Self {
        Self(1.0 / 60.0)
    }
}

/// [`BonesPlugin`] that installs the `bones_lib` systems for things such as animation etc.
pub fn plugin(core: &mut Session) {
    core.install_plugin(animation::plugin)
        .install_plugin(camera::plugin)
        .install_plugin(asset::plugin);
}
