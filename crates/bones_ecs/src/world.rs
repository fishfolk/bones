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

impl Default for World {
    fn default() -> Self {
        let mut resources = Resources::new();

        // Always initialize an Entities resource
        resources.init::<Entities>();

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
    pub fn run_system<R, S: IntoSystem<R>>(&mut self, system: S) -> SystemResult {
        let mut s = system.system();

        s.initialize(self);
        s.run(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

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
        let mut bitset = pos.bitset().clone();
        bitset.bit_and(vel.bitset());
        for entity in entities.iter_with_bitset(&bitset) {
            let pos = pos.get_mut(entity).unwrap();
            let vel = vel.get(entity).unwrap();

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
        let mut bitset = pos.bitset().clone();
        bitset.bit_and(vel.bitset());
        bitset.bit_or(marker.bitset());
        for entity in entities.iter_with_bitset(&bitset) {
            match (i, pos.get(entity), vel.get(entity), marker.get(entity)) {
                (0, Some(Pos(0, 100)), Some(Vel(0, -1)), None)
                | (1, Some(Pos(0, 0)), Some(Vel(1, 1)), Some(Marker)) => (),
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
        let mut bitset = pos.bitset().clone();
        bitset.bit_and(vel.bitset());
        bitset.bit_or(marker.bitset());
        for entity in entities.iter_with_bitset(&bitset) {
            match (i, pos.get(entity), vel.get(entity), marker.get(entity)) {
                (0, Some(Pos(0, 99)), Some(Vel(0, -1)), None)
                | (1, Some(Pos(1, 1)), Some(Vel(1, 1)), Some(Marker)) => (),
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
}
