//! Extensions of bones_ecs for utility functions in resetting a World
use crate::prelude::*;

/// Resource that allows user to trigger a reset of world, clearing entities, components, and resources.
/// This is supported in bone's default session runners - but if implementing custom runner, must call `world.handle_world_reset` inside step.
///
/// `reset_world: ResMutInit<ResetWorld>` and setting `reset_world.reset = true;` may be used to trigger a reset from system execution.
#[derive(Copy, Clone, HasSchema, Default)]
pub struct ResetWorld {
    /// Set to true to trigger reset of [`World`].
    pub reset: bool,
}

/// Extension of [`World`]
pub trait WorldExt {
    /// May be called by [`SessionRunner`] before or after [`SystemStages::run`] to allow triggering reset
    /// of world with the [`ResetWorld`] resource.
    fn handle_world_reset(&mut self);

    /// Check if reset has been triggered by [`ResetWorld`] resource. If [`SessionRunner`] needs to do anything special
    /// for a world reset (preserve managed resources, etc) - can check this before calling `handle_world_reset` to see
    /// if a rese will occur.
    fn reset_triggered(&self) -> bool;

    /// Provides a low-level interface for resetting entities, components, and resources.
    ///
    /// Waring: Calling this function on World in a [`Session`] using network session runner is likely to introduce non-determinism.
    /// It is strongly recommended to use the [`ResetWorld`] Resource to trigger this in manner compatible with net rollback.
    fn reset_internals(&mut self);
}

impl WorldExt for World {
    fn handle_world_reset(&mut self) {
        if self.reset_triggered() {
            self.reset_internals();
        }
    }

    fn reset_triggered(&self) -> bool {
        self.get_resource::<ResetWorld>().map_or(false, |r| r.reset)
    }

    fn reset_internals(&mut self) {
        // Clear all component stores
        self.components = ComponentStores::default();

        // save copy of special resources that should always remain in session / managed by bones
        let time = self.get_resource::<Time>().map(|t| *t);
        let session_opts = self.get_resource::<SessionOptions>().map(|x| *x);

        // Remove any owned resources (preserves shared resources)
        // This allows world resources to be reset inside session run safely, as shared resources are only injected
        // outside of session run, and a runner may take another step after the reset.
        self.resources.clear_owned_resources();

        // Entities were cleared with resources - ensure is present.
        self.init_resource::<Entities>();

        // Re-insert preserved resources
        if let Some(time) = time {
            self.insert_resource(time);
        }
        if let Some(session_opts) = session_opts {
            self.insert_resource(session_opts);
        }
    }
}
