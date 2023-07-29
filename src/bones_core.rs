//! The [`BonesCore`] and related types.

use crate::prelude::*;

/// A bones [`World`] along with it's [`SystemStages`].
#[derive(Default)]
pub struct BonesCore {
    /// The ECS world for the core.
    pub world: World,
    /// The system
    pub stages: SystemStages,
}

impl BonesCore {
    /// Create an empty [`BonesCore`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a plugin.
    pub fn install_plugin(&mut self, plugin: impl BonesPlugin) -> &mut Self {
        plugin.install(self);
        self
    }

    /// Snapshot the world state.
    ///
    /// This is the same as `core.world.clone()`, but it is more explicit.
    pub fn snapshot(&self) -> World {
        self.world.clone()
    }

    /// Restore the world state.
    ///
    /// Re-sets the world state to that of the provided `world`, which may or may not have been
    /// created with [`snapshot()`][Self::snapshot].
    ///
    /// This is the same as doing an [`std::mem::swap`] on `self.world`, but it is more explicit.
    pub fn restore(&mut self, world: &mut World) {
        std::mem::swap(&mut self.world, world)
    }
}

/// Trait for plugins that can be installed into a [`BonesCore`].
pub trait BonesPlugin {
    /// Install the plugin into the [`BonesCore`].
    fn install(self, core: &mut BonesCore);
}

impl<F: FnOnce(&mut BonesCore)> BonesPlugin for F {
    fn install(self, core: &mut BonesCore) {
        (self)(core)
    }
}
