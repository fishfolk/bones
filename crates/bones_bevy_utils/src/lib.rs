//! Bevy plugin for rendering Bones framework games.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::sync::Arc;

use type_ulid::TypeUlid;

/// The prelude.
pub mod prelude {
    pub use crate::*;
}

/// Helper trait for converting bones types to Bevy types.
pub trait IntoBevy<To> {
    /// Convert the type to a Bevy type.
    fn into_bevy(self) -> To;
}

/// Resource that contains a bevy world.
///
/// This may be used to give the bones ECS direct access to the bevy world.
///
/// One way to do this is to [`std::mem::swap`] an empty world in the [`BevyWorld`]` resource, with
/// the actual Bevy world, immediatley before running the bones ECS systems. Then you can swap it
/// back once the bones systems finish.
#[derive(TypeUlid, Clone, Default)]
#[ulid = "01GNX5CJAAHS31DA9HXZ2CF74B"]
pub struct BevyWorld(Arc<bevy_ecs::world::World>);

impl std::ops::Deref for BevyWorld {
    type Target = Arc<bevy_ecs::world::World>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BevyWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
