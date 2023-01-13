//! Opinionated game meta-engine built on Bevy.

#[doc(inline)]
pub use {bones_asset as asset, bones_ecs as ecs, bones_input as input, bones_render as render};

#[cfg(feature = "bevy")]
pub use bones_bevy_utils as bevy_utils;

/// Bones lib prelude
pub mod prelude {
    pub use crate::{
        animation::prelude::*, asset::prelude::*, ecs::prelude::*, input::prelude::*,
        render::prelude::*, FrameTime,
    };

    #[cfg(feature = "bevy")]
    pub use crate::bevy_utils::*;
}
use prelude::*;

pub mod animation;

/// This is a resource that stores the game's fixed frame time.
///
/// For instance, if the game logic is meant to run at a fixed frame rate of 60 fps, then this
/// should be `1.0 / 60.0`.
///
/// This resource is used by animation or other timing-sensitive code when running code that should
/// run the same, regardless of the games fixed updates-per-second.
#[derive(Clone, TypeUlid, Deref, DerefMut)]
#[ulid = "01GP1VWPKF2H7CKDCD987PHBWV"]
pub struct FrameTime(pub f32);

impl Default for FrameTime {
    fn default() -> Self {
        Self(1.0 / 60.0)
    }
}

/// Install the `bones_lib` systems for things such as animation etc. into a [`SystemStages`].
pub fn install(stages: &mut SystemStages) {
    animation::install(stages);
}
