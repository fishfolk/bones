//! Conveniences for resetting a World to its' startup state.
use crate::prelude::*;

/// Convenience resource for reseting the [`World`] while retaining a selection of resources if desired.
///
/// See [`Self::reset_world`]
#[derive(HasSchema, Clone, Default)]
pub struct ResetWorld {
    /// Set of [`SchemaId`]s for the resources that will not be removed from the
    /// world when calling [`Self::reset_world`].
    pub reset_resources: HashSet<SchemaId>,
}

impl ResetWorld {
    /// Adds the [`SchemaId`] of the type given to the set of resources to
    /// retain on reset.
    pub fn retain_resource<T: HasSchema>(&mut self) {
        self.reset_resources.insert(T::schema().id());
    }

    /// Removes the [`SchemaId`] of the type given from the set of resources to
    /// retain on reset.
    pub fn drop_resource<T: HasSchema>(&mut self) {
        self.reset_resources.remove(&T::schema().id());
    }

    /// Convenience method for reseting the [`World`]. Clears components,
    /// entities, and resources out of the world.
    ///
    /// Resources in [`self.reset_resources`](Self::reset_resources) field are
    /// not removed from the world and can be easily added via
    /// [`Self::retain_resource`] or removed via [`Self::drop_resource`].
    ///
    /// [`Entities`], [`Time`], [`SessionOptions`], and [`ResetWorld`] are
    /// always retained when calling.
    ///
    /// This should likely be the first call in your systems so that components
    /// are rendered continuously without dropping out for one frame before
    /// respawning world startup components.
    ///
    /// Safe to call while borrowing `Self` as a resource from the world without
    /// panicking.
    pub fn reset_world(&mut self, world: &World) {
        self.retain_resource::<ResetWorld>(); // This prevents a mutable borrow in `retain_owned_resources`.
        self.retain_resource::<Entities>();
        self.retain_resource::<Time>(); // TODO: make sure we need the `Time` resource retained.
        self.retain_resource::<SessionOptions>();

        world.resource_mut::<Entities>().kill_all(); // `kill_all` for component clearing
        world.maintain(); // clear components
        *world.resource_mut::<Entities>() = Entities::default(); // fully reset entities
        world.retain_owned_resources(|schema_id| self.reset_resources.contains(&schema_id));
    }
}
