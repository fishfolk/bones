//! Opinionated game meta-engine built on Bevy.

#[doc(inline)]
pub use {
    bones_asset as asset, bones_bevy_utils as bevy_utils, bones_ecs as ecs, bones_input as input,
    bones_render as render,
};

/// Bones lib prelude
pub mod prelude {
    pub use crate::{asset::prelude::*, ecs::prelude::*, input::prelude::*, render::prelude::*};

    #[cfg(feature = "bevy")]
    pub use crate::bevy_utils::*;
}

/// This crate provides 2D camera shake using the methodology described in this excellent [GDC
/// talk](https://www.youtube.com/watch?v=tu-Qe66AvtY) by Squirrel Eiserloh.
#[cfg(feature = "camera_shake")]
pub mod camera_shake {
    pub use bones_camera_shake::*;
}
