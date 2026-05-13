use bones_framework::prelude::*;

#[derive(HasSchema, Default, Clone)]
struct Counter(u32);

/// Verify that startup systems run again after a world reset
#[test]
pub fn startup_system_reset() {
    let mut game = Game::new();
    // Shared resource, should survive reset
    game.init_shared_resource::<Counter>();

    // Session startup increments counter by 1
    game.sessions
        .create_with("game", |builder: &mut SessionBuilder| {
            builder.add_system_to_stage(Startup, |mut counter: ResMut<Counter>| {
                // Increment to 1
                counter.0 += 1;
            });
        });

    // Step twice, startup system should only run once
    game.step(Instant::now());
    game.step(Instant::now());

    // Verify startup system ran and incremented only once
    assert_eq!(game.shared_resource::<Counter>().0, 1);

    // Reset the world; Equivalent to reseting at the start or end of the frame.
    // `StartupSystemsRan` is inserted into the world to indicate completion,
    // but is not retained so startup systems should run again.
    {
        let world = &game.sessions.get_mut("game").unwrap().world;
        world.resource_mut::<ResetWorld>().reset_world(world);
    }

    // step again to trigger startup
    game.step(Instant::now());

    // Shared resource is not included in reset, should be incremented 2nd time
    assert_eq!(game.shared_resource::<Counter>().0, 2);
}

/// Verify that single success systems run again (until success condition)
/// after a world reset
#[test]
pub fn single_success_system_reset() {
    let mut game = Game::new();

    game.sessions
        .create_with("game", |builder: &mut SessionBuilder| {
            // Counter should not be retained on reset.
            builder.add_system_to_stage(Startup, |world: &World| {
                world.insert_resource(Counter(0));
            });
            // Single success system increments counter by 1 until 2.
            builder.add_system_to_stage(
                First,
                single_success_system(Ulid::new(), |mut counter: ResMut<Counter>| -> Option<()> {
                    // Increment until 2
                    counter.0 += 1;
                    if counter.0 >= 2 {
                        return Some(());
                    }

                    None
                }),
            );
        });

    // Step three times, single success should've incremented counter to 2 and completed.
    game.step(Instant::now());
    game.step(Instant::now());
    game.step(Instant::now());

    // Verify startup system ran and incremented only twice
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);
    }

    // Reset the world; Equivalent to reseting at the start or end of the frame.
    {
        let world = &game.sessions.get_mut("game").unwrap().world;
        world.resource_mut::<ResetWorld>().reset_world(world);
    }

    // Counter & SingleSuccessSystems should not exist now.
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>();
        assert!(counter.is_none());

        let sss = session.world.get_resource::<SingleSuccessSystems>();
        assert!(sss.is_none());
    }

    // Counter resource should be re-initialized on CoreStage::Startup.
    // SingleSuccessSystems should have no completions. The one "SSS" will run
    // incrementing to 1 without completing.
    game.step(Instant::now());
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 1);

        let sss = session
            .world
            .get_resource::<SingleSuccessSystems>()
            .unwrap();
        assert!(sss.is_empty());
    }

    // Run a few more times, single success system should stop at 2:
    game.step(Instant::now());
    game.step(Instant::now());
    game.step(Instant::now());
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);

        let sss = session
            .world
            .get_resource::<SingleSuccessSystems>()
            .unwrap();
        assert!(!sss.is_empty());
    }
}

#[test]
pub fn reset_world_resource_retain() {
    let mut game = Game::new();

    // insert counter resource
    game.sessions
        .create_with("game", |builder: &mut SessionBuilder| {
            // This will put the resource into `ResetWorld`'s retained resources
            // when built.
            builder.insert_resource(Counter(2));
            // Insert unit type on startup that won't be retained on reset.
            builder.add_system_to_stage(Startup, |world: &World| {
                world.insert_resource(());
            });
        });

    game.step(Instant::now());

    // assert Counter is still 2 & unit resource exists
    {
        let world = &game.sessions.get("game").unwrap().world;
        let counter = world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);
        let unit = world.get_resource::<()>();
        assert!(unit.is_some());
    }

    // Reset the world; Equivalent to reseting at the start or end of the frame.
    // Counter should be retained since `SessionBuilder` inserts "startup
    // resources" into `ResetWorld` when the session is built.
    // Unit type was inserted in the Startup stage so should not be retained.
    {
        let world = &game.sessions.get_mut("game").unwrap().world;
        world.resource_mut::<ResetWorld>().reset_world(world);
    }

    // Verify that Counter is still 2, instead of being removed.
    // Verify counter is removed.
    {
        let world = &game.sessions.get("game").unwrap().world;
        let counter = world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);
        let unit = world.get_resource::<()>();
        assert!(unit.is_none());
    }
}

#[test]
pub fn reset_world_resource_drop() {
    let mut game = Game::new();

    // insert counter resource
    game.sessions
        .create_with("game", |builder: &mut SessionBuilder| {
            // This will put the resource into `ResetWorld`'s retained resources
            // when built.
            builder.insert_resource(Counter(2));
        });

    game.step(Instant::now());

    // there are no systems but assert Counter is still 2
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);
    }
    // Reset the world; Equivalent to reseting at the start or end of the frame.
    // Remove Counter from `ResetWorld`; It should not exist after the call.
    {
        let world = &game.sessions.get_mut("game").unwrap().world;
        let mut reset = world.resource_mut::<ResetWorld>();
        reset.drop_resource::<Counter>();
        reset.reset_world(world);

        assert!(world.get_resource::<Counter>().is_none());
    }
}
