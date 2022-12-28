//! Opinionated game meta-engine built on Bevy.

/// Entity component system for the bones library.
pub mod ecs {
    pub use bones_ecs::*;
}

/// This crate provides 2D camera shake using the methodology described in this excellent [GDC
/// talk](https://www.youtube.com/watch?v=tu-Qe66AvtY) by Squirrel Eiserloh.
#[cfg(feature = "camera_shake")]
pub mod camera_shake {
    pub use bones_camera_shake::*;
}
