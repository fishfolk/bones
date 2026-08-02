//! Conveniences for resetting a World to its' default state.
use crate::prelude::*;

/// Convenience type for selecting resources to keep before clearing the
/// components, entities, and resources on a [`World`].
///  ```ignore
/// ResetWorld::standard()
///     .keep::<Scoreboard>()
///     .keep::<WritingRust>()
///     .keep::<TheBugsAway>()
///     .reset(world);
/// ```
/// Reseting the world should likely be the first call in your systems when
/// applicable so that components are rendered continuously without disappearing
/// for a frame before respawning world startup components.
///
/// You can create the type with [`Self::standard`], which is recommended for
/// keeping certain special resources like [`SessionOptions`] & [`Entities`] on
/// creation, or you can use [`Self::empty`] to create a new type that won't
/// keep any resources on creation.
///
/// To add more resources to the list you can use [`self.keep`](Self::keep) with
/// the specified type, no data needed, and chain the methods like the example
/// above.
///
/// Uses a [`HashSet`] of [`SchemaId`] to determine the resources that will not
/// be removed from the world on reset.
#[derive(HasSchema, Default, Clone, Deref, DerefMut)]
pub struct ResetWorld(pub HashSet<SchemaId>);

impl ResetWorld {
    /// Create a [`ResetWorld`] with no resources kept as opposed to the
    /// [`Self::standard`] which keeps certain special resources on creation.
    pub fn empty() -> Self {
        Self(HashSet::default())
    }
    /// Create a [`ResetWorld`] that already keeps some basic special resources
    /// that are usually needed like [`SessionOptions`] & [`Entities`].
    pub fn standard() -> Self {
        Self::empty()
            // Even though Entities is completely cleared on reset, it is always
            // expected everywhere so we can't remove it from the World.
            .keep::<Entities>()
            // SessionOptions is re-inserted every frame, but may still be
            // wanted right after reset and is expected in `Game` right now.
            .keep::<SessionOptions>()
    }
    /// Adds the [`SchemaId`] of the type given to the set of resources to keep
    /// on reset.
    ///
    /// Returns `Self` to allow chaining the method:
    /// ```ignore
    /// Self::standard()
    ///     .keep::<Scoreboard>()
    ///     .keep::<WritingRust>()
    ///     .keep::<TheBugsAway>()
    ///     .reset(world);
    /// ```
    pub fn keep<T: HasSchema>(mut self) -> Self {
        self.insert(T::schema().id());
        self
    }
    /// Reset the given [`World`], clearing resources that are not included in
    /// `self` as well as components and entities.
    pub fn reset(self, world: &World) {
        world.resource_mut::<Entities>().kill_all(); // `kill_all` for component clearing
        world.maintain(); // clear components
        *world.resource_mut::<Entities>() = Entities::default(); // fully reset entities
        world.retain_owned_resources(|schema_id| self.contains(&schema_id));
    }
    /// Reset the given [`World`]'s resources that are not included in `self`.
    pub fn reset_resources(self, world: &World) {
        world.retain_owned_resources(|schema_id| self.contains(&schema_id));
    }
}
