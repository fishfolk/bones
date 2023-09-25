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
        let mut entities = self.resources.get_mut::<Entities>().unwrap();

        for components in &mut self.components.components.values_mut() {
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
    pub fn run_system<'system, R, In, Out, S>(&mut self, system: S, input: In) -> Out
    where
        In: 'system,
        Out: 'system,
        S: IntoSystem<R, In, Out>,
        S::Sys: 'system,
    {
        let mut s = system.system();

        s.initialize(self);
        s.run(self, input)
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
    ///
    /// You can also use [`world.init_param()`][Self::init_param] to manually initialize specific
    /// parameters if you know which ones will need to be initialized.
    pub fn run_initialized_system<'system, Args, In, Out, S>(&self, system: S, input: In) -> Out
    where
        In: 'system,
        Out: 'system,
        S: IntoSystem<Args, In, Out>,
        S::Sys: 'system,
    {
        let mut s = system.system();
        s.run(self, input)
    }

    /// Initialize a resource of type `T` by inserting it's default value.
    pub fn init_resource<R: HasSchema + FromWorld>(&mut self) -> RefMut<'_, R> {
        if unlikely(!self.resources.contains::<R>()) {
            let value = R::from_world(self);
            self.resources.insert(value);
        }
        self.resource_mut()
    }

    /// Initialize a system parameter.
    ///
    /// It is not necessary to do this manually unless you are going to run a system using
    /// [`world.run_initialized_system()`][Self::run_initialized_system()] and you need to make sure
    /// one of it's parameters are pre-initialized.
    pub fn init_param<P: SystemParam>(&mut self) -> &mut Self {
        P::initialize(self);
        self
    }

    /// Insert a resource.
    pub fn insert_resource<R: HasSchema>(&mut self, resource: R) -> Option<AtomicResource<R>> {
        self.resources.insert(resource)
    }

    /// Borrow a resource from the world.
    /// # Panics
    /// Panics if the resource does not exist in the store.
    #[track_caller]
    pub fn resource<T: HasSchema>(&self) -> Ref<T> {
        match self.resources.get::<T>() {
            Some(r) => r,
            None => panic!(
                "Requested resource {} does not exist in the `World`.
                Did you forget to add it using `world.insert_resource` / `world.init_resource`?",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Borrow a resource from the world.
    /// # Panics
    /// Panics if the resource does not exist in the store.
    #[track_caller]
    pub fn resource_mut<T: HasSchema>(&mut self) -> RefMut<T> {
        match self.resources.get_mut::<T>() {
            Some(r) => r,
            None => panic!(
                "Requested resource {} does not exist in the `World`.
                Did you forget to add it using `world.insert_resource` / `world.init_resource`?",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Borrow a resource from the world, if it exists.
    pub fn get_resource<T: HasSchema>(&self) -> Option<Ref<T>> {
        self.resources.get()
    }

    /// Borrow a resource from the world, if it exists.
    pub fn get_resource_mut<T: HasSchema>(&mut self) -> Option<RefMut<T>> {
        self.resources.get_mut()
    }
}

/// Creates an instance of the type this trait is implemented for
/// using data from the supplied [`World`].
///
/// This can be helpful for complex initialization or context-aware defaults.
pub trait FromWorld {
    /// Creates `Self` using data from the given [`World`].
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
        let mut world1 = World::new();
        world1.run_system(setup_world, ());

        // Snapshot world1
        let mut snap = world1.clone();

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
        send(World::new())
    }

    fn send<T: Send>(_: T) {}

    // ============
    //  From World
    // ============

    #[derive(Clone, HasSchema, Default)]
    struct TestResource(u32);

    #[derive(Clone, HasSchema)]
    #[schema(opaque, no_default)]
    struct TestFromWorld(u32);
    impl FromWorld for TestFromWorld {
        fn from_world(world: &mut World) -> Self {
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
