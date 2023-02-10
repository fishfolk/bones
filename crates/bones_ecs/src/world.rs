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
    pub(crate) resources: Resources,
    /// Stores the world components.
    pub components: ComponentStores,
}

impl Default for World {
    fn default() -> Self {
        let mut resources = Resources::new();

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

    /// Remove the component info for dead entities.
    ///
    /// This should be called every game frame to cleanup entities that have been killed.
    ///
    /// This will remove the component storage for all killed entities, and allow their slots to be
    /// re-used for any new entities.
    pub fn maintain(&mut self) {
        let entities = self.resources.get::<Entities>();
        let mut entities = entities.borrow_mut();

        for components in &mut self.components.components.values_mut() {
            let mut components = components.borrow_mut();
            let killed = entities.killed();
            for &entity in killed {
                // Safe: We don't provide an out pointer, so it doesn't overlap the component's
                // internal storage.
                unsafe {
                    components.remove(entity, None);
                }
            }
        }
        entities.clear_killed();
    }

    /// Run a system once.
    ///
    /// This is good for initializing the world with setup systems.
    pub fn run_system<R, Out, S: IntoSystem<R, Out>>(&mut self, system: S) -> SystemResult<Out> {
        let mut s = system.system();

        s.initialize(self);
        s.run(self)
    }

    /// Run a system once, assuming any necessary initialization has already been performed for that
    /// system.
    ///
    /// This **will not** intialize the system, like [`run_system()`][Self::run_system] will, but it
    /// only requires an immutable reference to the world.
    ///
    /// # Panics
    ///
    /// Panics may occur if you pass in a system, for example, that takes a component type argument
    /// and that component has not been initialized yet.
    ///
    /// If all the system parameters have already been initialized, by calling
    /// [`initialize()`][System::initialize] on the system, then this will work fine.
    pub fn run_initialized_system<R, Out, S: IntoSystem<R, Out>>(
        &self,
        system: S,
    ) -> SystemResult<Out> {
        let mut s = system.system();
        s.run(self)
    }

    /// Initialize a resource of type `T` by inserting it's default value.
    pub fn init_resource<R: TypedEcsData + FromWorld>(&mut self) {
        if !self.resources.contains::<R>() {
            let value = R::from_world(self);
            self.resources.insert(value)
        }
    }

    /// Insert a resource.
    ///
    /// # Panics
    ///
    /// Panics if you try to insert a Rust type with a different [`TypeId`], but the same
    /// [`TypeUlid`] as another resource in the store.
    pub fn insert_resource<R: TypedEcsData>(&mut self, resource: R) {
        self.resources.insert(resource)
    }

    /// Get a resource handle from the store.
    ///
    /// This is not the resource itself, but a handle, may be cloned cheaply.
    ///
    /// In order to access the resource you must call [`borrow()`][AtomicResource::borrow] or
    /// [`borrow_mut()`][AtomicResource::borrow_mut] on the returned value.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist in the store.
    pub fn resource<R: TypedEcsData>(&self) -> AtomicResource<R> {
        self.resources.get::<R>()
    }

    /// Gets a resource handle from the store if it exists.
    pub fn get_resource<R: TypedEcsData>(&self) -> Option<AtomicResource<R>> {
        self.resources.try_get::<R>()
    }
}

/// Creates an instance of the type this trait is implemented for
/// using data from the supplied [World].
///
/// This can be helpful for complex initialization or context-aware defaults.
pub trait FromWorld {
    /// Creates `Self` using data from the given [World]
    fn from_world(world: &mut World) -> Self;
}

impl<T: Default> FromWorld for T {
    fn from_world(_world: &mut World) -> Self {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    use super::FromWorld;

    #[derive(Clone, TypeUlid, Debug, Eq, PartialEq)]
    #[ulid = "01GNDN2QYC1TRE763R54HVWZ0W"]
    struct Pos(i32, i32);

    #[derive(Clone, TypeUlid, Debug, Eq, PartialEq)]
    #[ulid = "01GNDN3HCY2F1SGYE8Z0GGDMXB"]
    struct Vel(i32, i32);

    #[derive(Clone, TypeUlid, Debug, Eq, PartialEq)]
    #[ulid = "01GNDN3QJD1SP7ANTZ0TG6Q804"]
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
            dbg!(i, entity);
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
        let mut world = World::new();

        world.run_system(setup_world).unwrap();

        // Make sure our entities exist visit properly during iteration
        let test = || {};
        world.run_system(test).unwrap();

        // Mutate and read some components
        world.run_system(pos_vel_system).unwrap();

        // Make sure the mutations were applied
        world.run_system(test_pos_vel_1_run).unwrap();
    }

    #[test]
    fn snapshot() {
        let mut world1 = World::new();
        world1.run_system(setup_world).unwrap();

        // Snapshot world1
        let mut snap = world1.clone();

        // Make sure the snapshot represents world1's state
        snap.run_system(test_after_setup_state).unwrap();

        // Run the pos_vel system on world1
        world1.run_system(pos_vel_system).unwrap();

        // Make sure world1 has properly update
        world1.run_system(test_pos_vel_1_run).unwrap();

        // Make sure the snapshot hasn't changed
        snap.run_system(test_after_setup_state).unwrap();

        // Run the pos vel system once on the snapshot
        snap.run_system(pos_vel_system).unwrap();

        // Make sure the snapshot has updated
        world1.run_system(test_pos_vel_1_run).unwrap();
    }

    #[test]
    #[should_panic(expected = "TypeUlidCollision")]
    fn no_duplicate_component_uuids() {
        #[derive(Clone, TypeUlid)]
        #[ulid = "01GNDN440Q4FYH34TY8MV8CTTB"]
        struct A;

        /// This struct has the same UUID as struct [`A`]. Big no no!!
        #[derive(Clone, TypeUlid)]
        #[ulid = "01GNDN440Q4FYH34TY8MV8CTTB"]
        struct B;

        let mut w = World::default();
        w.components.init::<A>();
        w.components.init::<B>();
    }

    #[test]
    fn world_is_send() {
        send(World::new())
    }

    fn send<T: Send>(_: T) {}

    // ============
    //  From World
    // ============

    #[derive(Clone, TypeUlid)]
    #[ulid = "01GRWJV4NRXY9NJBBDMD2D9QK3"]
    struct TestResource(u32);

    #[derive(Clone, TypeUlid)]
    #[ulid = "01GRWJW44YGNSXQ81W395J0D52"]
    struct TestFromWorld(u32);
    impl FromWorld for TestFromWorld {
        fn from_world(world: &mut World) -> Self {
            let b = world.resource::<TestResource>();
            let b = b.borrow();
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
