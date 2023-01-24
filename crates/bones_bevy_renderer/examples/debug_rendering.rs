use bevy::prelude::*;
use bones_bevy_renderer::*;
use bones_lib::prelude as bones;

#[derive(Deref, DerefMut, Resource)]
struct Session(pub bones_lib::prelude::World);

impl HasBonesWorld for Session {
    fn world(&mut self) -> &mut bones::World {
        &mut self.0
    }
}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(BonesRendererPlugin::<Session>::new())
        .add_startup_system(setup)
        .add_system(spin)
        .run();
}

/// Setup the game, loading the metadata and starting the game session.
fn setup(mut commands: Commands) {
    let mut world = bones::World::new();

    commands.spawn(Camera2dBundle::default());

    world
        .run_system(
            |mut entities: bones::ResMut<bones::Entities>,
             mut path2ds: bones::CompMut<bones::Path2d>,
             mut transforms: bones::CompMut<bones::Transform>| {
                const SIZE: f32 = 100.0;

                // Draw a red square
                let ent = entities.create();
                path2ds.insert(
                    ent,
                    bones::Path2d {
                        color: [1.0, 0.0, 0.0, 1.0],
                        points: vec![
                            glam::vec2(-SIZE, -SIZE),
                            glam::vec2(SIZE, -SIZE),
                            glam::vec2(SIZE, SIZE),
                            glam::vec2(-SIZE, SIZE),
                            glam::vec2(-SIZE, -SIZE),
                        ],
                        thickness: 2.0,
                        ..default()
                    },
                );
                transforms.insert(ent, default());

                const SIZE2: f32 = SIZE / 2.0;

                // Draw two blue lines
                let ent = entities.create();
                path2ds.insert(
                    ent,
                    bones::Path2d {
                        color: [0.0, 0.0, 1.0, 1.0],
                        points: vec![
                            // The first line
                            glam::vec2(-SIZE2, -SIZE2),
                            glam::vec2(SIZE2, -SIZE2),
                            // The second line
                            glam::vec2(SIZE2, SIZE2),
                            glam::vec2(-SIZE2, SIZE2),
                        ],
                        thickness: 4.0,
                        // This means that it won't connect points with indexes 2 and 3, so that
                        // they will be separate lines.
                        line_breaks: vec![2],
                    },
                );
                transforms.insert(ent, default());
            },
        )
        .unwrap();

    commands.insert_resource(Session(world));
}

fn spin(session: ResMut<Session>) {
    session
        .run_initialized_system(|mut transforms: bones::CompMut<bones::Transform>| {
            transforms.iter_mut().for_each(|trans| {
                trans.rotation *= Quat::from_axis_angle(Vec3::Z, f32::to_radians(0.1))
            });
        })
        .unwrap();
}
