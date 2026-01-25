//! Contains the ECS [`World`].

use crate::prelude::*;

/// The [`World`] is simply a collection of [`Resources`], and [`ComponentStores`].
///
/// Also stored in the world is the [`Entities`], but it is stored as a resource.
///
/// [`World`] is designed to be trivially [`Clone`]ed to allow for snapshotting the world state. The
/// is especially useful in the context of rollback networking, which requires the ability to
/// snapshot and restore state.
#[derive(Clone)]
pub struct World {
    /// Stores the world resources.
    pub resources: Resources,
    /// Stores the world components.
    pub components: ComponentStores,
}
impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World").finish()
    }
}

impl Default for World {
    fn default() -> Self {
        let resources = Resources::new();

        // Always initialize an Entities resource
        resources.insert(Entities::default());

        Self {
            resources,
            components: Default::default(),
        }
    }
}

impl World {
    /// Create a new [`World`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new world that uses the provided entities resource.
    ///
    /// This allows multiple worlds to avoid allocating the same entity IDs.
    pub fn with_entities(entities: AtomicResource<Entities>) -> Self {
        let resources = Resources::new();
        resources
            .untyped()
            .insert_cell(entities.into_untyped())
            .unwrap();
        World {
            resources,
            components: default(),
        }
    }

    /// Remove the component info for dead entities.
    ///
    /// This should be called every game frame to cleanup entities that have been killed.
    ///
    /// This will remove the component storage for all killed entities, and allow their slots to be
    /// re-used for any new entities.
    pub fn maintain(&self) {
        let mut entities = self.resource_mut::<Entities>();
        for components in self.components.components.read_only_view().values() {
            let mut components = components.borrow_mut();
            let killed = entities.killed();
            for &entity in killed {
                // Safe: We don't provide an out pointer, so it doesn't overlap the component's
                // internal storage.
                unsafe {
                    components.remove_raw(entity, None);
                }
            }
        }
        entities.clear_killed();
    }

    /// Run a system once.
    ///
    /// This is good for initializing the world with setup systems.
    pub fn run_system<'system, R, In, Out, S>(&self, system: S, input: In) -> Out
    where
        In: 'system,
        Out: 'system,
        S: IntoSystem<R, In, Out>,
        S::Sys: 'system,
    {
        let mut s = system.system();
        s.run(self, input)
    }

    /// Get an entity's components.
    ///
    /// # Panics
    ///
    /// Panics if the entity does not have the required components from the query.
    pub fn entity_components<Q: QueryItem>(
        &self,
        entity: Entity,
        query: Q,
    ) -> <Q::Iter as Iterator>::Item {
        self.get_entity_components(entity, query).unwrap()
    }

    /// Get an entity's components.
    pub fn get_entity_components<Q: QueryItem>(
        &self,
        entity: Entity,
        query: Q,
    ) -> Option<<Q::Iter as Iterator>::Item> {
        let mut bitset = BitSetVec::default();
        if self.resource::<Entities>().bitset().contains(entity) {
            bitset.set(entity);
        }
        query.apply_bitset(&mut bitset);
        match query.get_single_with_bitset(bitset.into()) {
            Ok(components) => Some(components),
            Err(QuerySingleError::NoEntities) => None,
            Err(QuerySingleError::MultipleEntities) => {
                panic!(
                    "Query returned a MultipleEntities error for a bitset that \
                    contains at most one enabled bit"
                )
            }
        }
    }

    /// Initialize a resource of type `T` by inserting it's default value.
    pub fn init_resource<R: HasSchema + FromWorld>(&mut self) -> RefMut<'_, R> {
        if unlikely(!self.resources.contains::<R>()) {
            let value = R::from_world(self);
            self.resources.insert(value);
        }
        self.resource_mut()
    }

    /// Insert a resource.
    pub fn insert_resource<R: HasSchema>(&self, resource: R) -> Option<R> {
        self.resources.insert(resource)
    }

    /// Borrow a resource from the world.
    /// # Panics
    /// Panics if the resource does not exist in the store.
    #[track_caller]
    pub fn resource<T: HasSchema>(&self) -> Ref<'_, T> {
        match self.resources.get::<T>() {
            Some(r) => r,
            None => panic!(
                "Requested resource {} does not exist in the `World`. \
                Did you forget to add it using `world.insert_resource` / `world.init_resource`?",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Borrow a resource from the world.
    /// # Panics
    /// Panics if the resource does not exist in the store.
    #[track_caller]
    pub fn resource_mut<T: HasSchema>(&self) -> RefMut<'_, T> {
        match self.resources.get_mut::<T>() {
            Some(r) => r,
            None => panic!(
                "Requested resource {} does not exist in the `World`. \
                Did you forget to add it using `world.insert_resource` / `world.init_resource`?",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Borrow a resource from the world, if it exists.
    pub fn get_resource<T: HasSchema>(&self) -> Option<Ref<'_, T>> {
        self.resources.get()
    }

    /// Borrow a resource from the world, if it exists.
    pub fn get_resource_mut<T: HasSchema>(&self) -> Option<RefMut<'_, T>> {
        self.resources.get_mut()
    }

    /// Borrow a component store from the world.
    /// # Panics
    /// Panics if the component store does not exist in the world.
    #[track_caller]
    pub fn component<T: HasSchema>(&self) -> Ref<'_, ComponentStore<T>> {
        self.components.get::<T>().borrow()
    }

    /// Mutably borrow a component store from the world.
    /// # Panics
    /// Panics if the component store does not exist in the world.
    #[track_caller]
    pub fn component_mut<T: HasSchema>(&self) -> RefMut<'_, ComponentStore<T>> {
        self.components.get::<T>().borrow_mut()
    }

    /// Load snapshot of [`World`] into self.
    pub fn load_snapshot(&mut self, snapshot: World) {
        self.components = snapshot.components;
        self.resources = snapshot.resources;
    }
}

/// Creates an instance of the type this trait is implemented for
/// using data from the supplied [`World`].
///
/// This can be helpful for complex initialization or context-aware defaults.
pub trait FromWorld {
    /// Creates `Self` using data from the given [`World`].
    fn from_world(world: &World) -> Self;
}

impl<T: Default> FromWorld for T {
    fn from_world(_world: &World) -> Self {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    use super::FromWorld;

    #[derive(Clone, Copy, HasSchema, Debug, Eq, PartialEq, Default)]
    #[repr(C)]
    struct C(u32);

    #[derive(Clone, HasSchema, Debug, Eq, PartialEq, Default)]
    #[repr(C)]
    struct Pos(i32, i32);

    #[derive(Clone, HasSchema, Debug, Eq, PartialEq, Default)]
    #[repr(C)]
    struct Vel(i32, i32);

    #[derive(Clone, HasSchema, Debug, Eq, PartialEq, Default)]
    struct Marker;

    // Sets up the world with a couple entities.
    fn setup_world(
        mut entities: ResMut<Entities>,
        mut pos_comps: CompMut<Pos>,
        mut vel_comps: CompMut<Vel>,
        mut marker_comps: CompMut<Marker>,
    ) {
        let ent1 = entities.create();
        pos_comps.insert(ent1, Pos(0, 100));
        vel_comps.insert(ent1, Vel(0, -1));

        let ent2 = entities.create();
        pos_comps.insert(ent2, Pos(0, 0));
        vel_comps.insert(ent2, Vel(1, 1));
        marker_comps.insert(ent2, Marker);
    }

    /// Mutates the positions based on the velocities.
    fn pos_vel_system(entities: Res<Entities>, mut pos: CompMut<Pos>, vel: Comp<Vel>) {
        for (_, (pos, vel)) in entities.iter_with((&mut pos, &vel)) {
            pos.0 += vel.0;
            pos.1 += vel.1;
        }
    }

    /// Tests that the world's components matches the state it should after running `setup_world`.
    fn test_after_setup_state(
        entities: Res<Entities>,
        pos: Comp<Pos>,
        vel: Comp<Vel>,
        marker: Comp<Marker>,
    ) {
        let mut i = 0;
        for (entity, (pos, vel)) in entities.iter_with((&pos, &vel)) {
            let marker = marker.get(entity);
            match (i, pos, vel, marker) {
                (0, Pos(0, 100), Vel(0, -1), None) | (1, Pos(0, 0), Vel(1, 1), Some(Marker)) => (),
                x => unreachable!("{:?}", x),
            }
            i += 1;
        }
        assert_eq!(i, 2);
    }

    /// Tests that the worlds components matches the state it should after running the
    /// pos_vel_system one time.
    fn test_pos_vel_1_run(
        entities: Res<Entities>,
        pos: Comp<Pos>,
        vel: Comp<Vel>,
        marker: Comp<Marker>,
    ) {
        let mut i = 0;
        for (entity, (pos, vel)) in entities.iter_with((&pos, &vel)) {
            let marker = marker.get(entity);
            match (i, pos, vel, marker) {
                (0, Pos(0, 99), Vel(0, -1), None) | (1, Pos(1, 1), Vel(1, 1), Some(Marker)) => (),
                x => unreachable!("{:?}", x),
            }
            i += 1;
        }
        assert_eq!(i, 2);
    }

    #[test]
    fn sanity_check() {
        let world = World::new();

        world.run_system(setup_world, ());

        // Make sure our entities exist visit properly during iteration
        let test = || {};
        world.run_system(test, ());

        // Mutate and read some components
        world.run_system(pos_vel_system, ());

        // Make sure the mutations were applied
        world.run_system(test_pos_vel_1_run, ());
    }

    #[test]
    fn snapshot() {
        let world1 = World::new();
        world1.run_system(setup_world, ());

        // Snapshot world1
        let snap = world1.clone();

        // Make sure the snapshot represents world1's state
        snap.run_system(test_after_setup_state, ());

        // Run the pos_vel system on world1
        world1.run_system(pos_vel_system, ());

        // Make sure world1 has properly update
        world1.run_system(test_pos_vel_1_run, ());

        // Make sure the snapshot hasn't changed
        snap.run_system(test_after_setup_state, ());

        // Run the pos vel system once on the snapshot
        snap.run_system(pos_vel_system, ());

        // Make sure the snapshot has updated
        world1.run_system(test_pos_vel_1_run, ());
    }

    #[test]
    fn world_is_send() {
        fn send<T: Send>(_: T) {}
        send(World::new())
    }

    #[test]
    fn get_entity_components() {
        let w = World::default();

        let (e1, e2) = {
            let mut entities = w.resource_mut::<Entities>();
            (entities.create(), entities.create())
        };

        let state = w.components.get::<C>();
        let mut comp = state.borrow_mut();

        let c2 = C(2);
        comp.insert(e2, c2);

        assert_eq!(w.get_entity_components(e1, &comp), None);
        assert_eq!(w.get_entity_components(e2, &comp), Some(&c2));
    }

    // ============
    //  From World
    // ============

    #[derive(Clone, HasSchema, Default)]
    struct TestResource(u32);

    #[derive(Clone, HasSchema)]
    #[schema(opaque, no_default)]
    struct TestFromWorld(u32);
    impl FromWorld for TestFromWorld {
        fn from_world(world: &World) -> Self {
            let b = world.resource::<TestResource>();
            Self(b.0)
        }
    }

    #[test]
    fn init_resource_does_not_overwrite() {
        let mut w = World::default();
        w.insert_resource(TestResource(0));
        w.init_resource::<TestFromWorld>();
        w.insert_resource(TestResource(1));

        let resource = w.resource::<TestFromWorld>();
        assert_eq!(resource.0, 0);
    }
}
