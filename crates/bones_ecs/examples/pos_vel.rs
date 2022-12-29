use bones_ecs::prelude::*;
use glam::Vec2;

// Define our component types.
//
// Each component must be Clone + Sync + Send + TypeUlid.

#[derive(Clone, Debug, Deref, DerefMut, TypeUlid)]
#[ulid = "01GNDP2WAAN8C6C8XA5ZBXGHFR"]
pub struct Vel(pub Vec2);

#[derive(Clone, Debug, Deref, DerefMut, TypeUlid)]
#[ulid = "01GNDP34A7TMS8PFZAGSQJ5DDX"]
pub struct Pos(pub Vec2);

fn main() {
    // Initialize an empty world
    let mut world = World::new();

    // Run our setup system once to spawn our entities
    world.run_system(setup_system).ok();

    // Create a dispatcher to run our systems in our game loop
    let mut dispatcher = Dispatcher::builder()
        // Add our systems to the dispatcher
        .add(pos_vel_system)
        .add(print_system)
        .build(&mut world);

    // Run our game loop for 10 frames
    for _ in 0..10 {
        dispatcher.run_seq(&world).unwrap();
    }
}

/// Setup system that spawns two entities with a Pos and a Vel component.
fn setup_system(
    mut entities: ResMut<Entities>,
    mut pos_comps: CompMut<Pos>,
    mut vel_comps: CompMut<Vel>,
) {
    let ent1 = entities.create();
    pos_comps.insert(ent1, Pos(Vec2::new(0., 0.)));
    vel_comps.insert(ent1, Vel(Vec2::new(3.0, 1.0)));

    let ent2 = entities.create();
    pos_comps.insert(ent2, Pos(Vec2::new(0., 100.)));
    vel_comps.insert(ent2, Vel(Vec2::new(0.0, -1.0)));
}

/// Update the Pos of all entities with both a Pos and a Vel
fn pos_vel_system(mut pos: CompMut<Pos>, vel: Comp<Vel>) {
    for pos_vel in join!(&mut pos && &vel) {
        let (Some(pos), Some(vel)) = pos_vel else {
            continue;
        };
        **pos += **vel;
    }
}

/// Print the Pos and Vel of every entity
fn print_system(pos: Comp<Pos>, vel: Comp<Vel>) {
    println!("=====");
    for pos_vel in join!(&pos && &vel) {
        let (Some(pos), Some(vel)) = pos_vel else {
            continue;
        };
        println!("{pos:?} \t- {vel:?}");
    }
}
