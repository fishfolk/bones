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
        .create("game")
        .add_system_to_stage(Startup, |mut counter: ResMut<Counter>| {
            // Increment to 1
            counter.0 += 1;
        });

    // Step twice, startup system should only run once
    game.step(Instant::now());
    game.step(Instant::now());

    // Verify startup system ran and incremented only once
    assert_eq!(game.shared_resource::<Counter>().0, 1);

    // Reset the world; Equivalent to reseting at the start or end of the frame.
    // `CoreStageStartupSystemsRan` is inserted into the world to indicate
    // completion, but is not kept so startup systems should run again.
    ResetWorld::standard().reset(&game.sessions.get_mut("game").unwrap().world);

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
        .create("game")
        // Counter should not be kept on reset.
        .add_system_to_stage(Startup, |world: &World| {
            world.insert_resource(Counter(0));
        })
        // Single success system increments counter by 1 until 2.
        .add_system_to_stage(
            First,
            single_success_system(|mut counter: ResMut<Counter>| -> Option<()> {
                // Increment until 2
                counter.0 += 1;
                if counter.0 >= 2 {
                    return Some(());
                }

                None
            }),
        );

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
    ResetWorld::standard().reset(&game.sessions.get_mut("game").unwrap().world);

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
pub fn reset_world_resource_keep_and_drop() {
    let mut game = Game::new();

    game.sessions
        .create("game")
        // This will add the counter resource to the startup stage.
        .add_startup_resource(Counter(2))
        // Same here only with the default bool; we won't keep this one.
        .init_startup_resource::<bool>();

    game.step(Instant::now());

    // assert that the resources were inserted on startup
    {
        let world = &game.sessions.get("game").unwrap().world;

        let counter = world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);

        let boolean = world.get_resource::<bool>().unwrap();
        assert!(boolean == false); // false is boolean default
    }

    // keep resources & reset world
    {
        let world = &game.sessions.get_mut("game").unwrap().world;

        // Mutate resources to check if replaced on startup later.
        world.resource_mut::<Counter>().0 = 3;
        *world.resource_mut::<bool>() = true;

        ResetWorld::standard()
            // Retain the counter resource, but not the boolean resource.
            .keep::<Counter>()
            // Reset the world; Equivalent to reseting at the start or end of the
            // frame here.
            .reset(world);
    }

    // verify Counter is 3 and boolean type is gone
    {
        let world = &game.sessions.get("game").unwrap().world;

        let counter = world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 3);

        let boolean = world.get_resource::<bool>();
        assert!(boolean.is_none());
    }

    // step the game and make sure the startup resource systems don't replace
    // Counter; we never want to replace kept resources on startup otherwise
    // they wouldn't be useful.
    game.step(Instant::now());

    {
        let world = &game.sessions.get("game").unwrap().world;

        let counter = world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 3);

        // boolean should be re-inserted on startup but at the default value
        let boolean = world.get_resource::<bool>().unwrap();
        assert!(boolean == false);
    }
}
