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
    game.sessions.create_with("game", |builder| {
        builder.add_startup_system(|mut counter: ResMut<Counter>| {
            // Increment to 1
            counter.0 += 1;
        });
    });

    // Step twice, startup system should only run once
    game.step(Instant::now());
    game.step(Instant::now());

    // Verify startup system ran and incremented only once
    assert_eq!(game.shared_resource::<Counter>().unwrap().0, 1);

    // Add command that will trigger reset on next step
    {
        let game_session = game.sessions.get_mut("game").unwrap();
        game_session.world.init_resource::<CommandQueue>().add(
            |mut reset: ResMutInit<ResetWorld>| {
                reset.reset = true;
            },
        );
    }

    // step again, world should be reset. Startup doesn't run until next step though.
    game.step(Instant::now());

    // step again to trigger startup
    game.step(Instant::now());

    // Shared resource is not included in reset, should be incremented 2nd time
    assert_eq!(game.shared_resource::<Counter>().unwrap().0, 2);
}

/// Verify that single success systems run again (until success condition)
/// after a world reset
#[test]
pub fn single_success_system_reset() {
    let mut game = Game::new();

    // Session startup increments counter by 1
    game.sessions.create_with("game", |builder| {
        builder.init_resource::<Counter>();
        {
            let res = builder.resource_mut::<Counter>().unwrap();
            assert_eq!(res.0, 0);
        }
        // system
        builder.add_single_success_system(|mut counter: ResMut<Counter>| -> Option<()> {
            // Increment until 2
            counter.0 += 1;
            if counter.0 >= 2 {
                return Some(());
            }

            None
        });
    });

    // Step three times, single success should've incremented counter to 2 and completed.
    game.step(Instant::now());
    game.step(Instant::now());
    game.step(Instant::now());

    // Verify startup system ran and incremented only once
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);
    }

    // Add command that will trigger reset on next step
    {
        let game_session = game.sessions.get_mut("game").unwrap();
        game_session.world.init_resource::<CommandQueue>().add(
            |mut reset: ResMutInit<ResetWorld>| {
                reset.reset = true;
            },
        );
    }

    // step again, world should be reset after step. The Counter resource will not be present
    // until next step re-inits startup resources.
    // (at 0.unwrap())
    game.step(Instant::now());
    {
        let session = game.sessions.get("game").unwrap();
        assert!(session.world.get_resource::<Counter>().is_none());
    }

    // Startup resource should be re-initialized, and completion status of single single success system reset.
    // It will run incrementing to 1.
    game.step(Instant::now());
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 1);
    }

    // Run a few more times, single success system should stop at 2:
    game.step(Instant::now());
    game.step(Instant::now());
    game.step(Instant::now());
    {
        let session = game.sessions.get("game").unwrap();
        let counter = session.world.get_resource::<Counter>().unwrap();
        assert_eq!(counter.0, 2);
    }
}
