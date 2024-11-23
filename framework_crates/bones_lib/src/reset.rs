//! Extensions of bones_ecs for utility functions in resetting a World
use crate::prelude::*;

/// Resource that allows user to trigger a reset of world, clearing entities, components, and resources.
/// This is supported in bone's default session runners - but if implementing custom runner, must call `world.handle_world_reset` inside step.
///
/// `reset_world: ResMutInit<ResetWorld>` and setting `reset_world.reset = true;` may be used to trigger a reset from system execution.
#[derive(HasSchema, Clone, Default)]
pub struct ResetWorld {
    /// Set to true to trigger reset of [`World`].
    pub reset: bool,

    /// List of resources that will be inserted into [`World`] after the reset.
    /// These override any `startup resources` captured during session build.
    /// If want to preserve a resource instead of having it reset, insert it here.
    pub reset_resources: UntypedResourceSet,
}

impl ResetWorld {
    /// Insert a resource that will be applied after reset. If resource was created
    /// on session iniialization, this will overwrite it using reset resource instead.
    pub fn insert_reset_resource<T: HasSchema>(&mut self, resource: T) {
        self.reset_resources.insert_resource(resource);
    }

    /// Get a mutable reference to a reset resource if found.
    pub fn reset_resource_mut<T: HasSchema>(&self) -> Option<RefMut<T>> {
        self.reset_resources.resource_mut::<T>()
    }

    /// Insert resource in "empty" state - If resource was created
    /// on session iniialization, instead of being reset to that state,
    /// after reset this resource will not be on [`World`].
    pub fn insert_empty_reset_resource<T: HasSchema>(&mut self) {
        self.reset_resources.insert_empty::<T>();
    }
}

/// Extension of [`World`]
pub trait WorldExt {
    /// May be called by [`SessionRunner`] before or after [`SystemStages::run`] to allow triggering reset
    /// of world with the [`ResetWorld`] resource.
    ///
    /// `stages` is required, so that after reset, will immediatelly run startup tasks, instead of waiting until next step of [`SystemStages`].
    /// This avoids edge cases where other sessions may read expected resources from this session before its next step.
    fn handle_world_reset(&mut self, stages: &mut SystemStages);

    /// Check if reset has been triggered by [`ResetWorld`] resource. If [`SessionRunner`] needs to do anything special
    /// for a world reset (preserve managed resources, etc) - can check this before calling `handle_world_reset` to see
    /// if a rese will occur.
    fn reset_triggered(&self) -> bool;

    /// Provides a low-level interface for resetting entities, components, and resources.
    ///
    /// `stages` is required, so that after reset, will immediatelly run startup tasks, instead of waiting until next step of [`SystemStages`].
    /// This avoids edge cases where other sessions may read expected resources from this session before its next step.
    ///
    /// Waring: Calling this function on World in a [`Session`] using network session runner is likely to introduce non-determinism.
    /// It is strongly recommended to use the [`ResetWorld`] Resource to trigger this in manner compatible with net rollback.
    fn reset_internals(&mut self, stages: &mut SystemStages);
}

impl WorldExt for World {
    fn handle_world_reset(&mut self, stages: &mut SystemStages) {
        if self.reset_triggered() {
            self.reset_internals(stages);
        }
    }

    fn reset_triggered(&self) -> bool {
        self.get_resource::<ResetWorld>().map_or(false, |r| r.reset)
    }

    fn reset_internals(&mut self, stages: &mut SystemStages) {
        // Copy resources to be inserted after the reset.
        let post_reset_resources = self
            .get_resource::<ResetWorld>()
            .map(|x| x.reset_resources.clone());

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

        // Immediately run startup tasks, ensuring startup resources are present and run startup systems.
        stages.handle_startup(self);

        // Apply any reset resources to world, overwriting startup resources.
        if let Some(resources) = post_reset_resources {
            let remove_empty = true;
            resources.insert_on_world(self, remove_empty);
        }
    }
}
