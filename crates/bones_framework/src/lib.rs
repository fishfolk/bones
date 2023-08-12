use bones_lib::prelude::*;

mod animation;
mod camera;

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
