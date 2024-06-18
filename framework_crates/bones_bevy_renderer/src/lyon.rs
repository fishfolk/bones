use super::*;

use bevy_prototype_lyon::prelude as lyon;
use bones_framework::prelude::{BitSet, ComponentIterBitset};

pub use lyon::ShapePlugin;

pub fn sync_bones_path2ds(
    data: Res<BonesGame>,
    mut commands: Commands,
    mut bevy_bones_path2ds: Query<
        (Entity, &mut lyon::Path, &mut lyon::Stroke, &mut Transform),
        With<BevyBonesEntity>,
    >,
) {
    let game = &data;

    // Collect the bevy path2ds that we've created for the bones game
    let mut bevy_bones_path2ds = bevy_bones_path2ds.iter_mut();

    // Create a helper callback to add/update a bones path2d into the bevy world
    let mut add_bones_path2d = |bones_path2d: &bones::Path2d,
                                bones_transform: &bones::Transform| {
        // Get or create components for the entity
        let mut new_components = None;
        let mut existing_components;
        let (path, stroke, transform) = match bevy_bones_path2ds.next() {
            Some((_ent, path, stroke, transform)) => {
                existing_components = (path, stroke, transform);
                let (path, stroke, transform) = &mut existing_components;
                (&mut **path, &mut **stroke, &mut **transform)
            }
            None => {
                let bundle = lyon::ShapeBundle::default();
                new_components = Some((
                    bundle.path,
                    lyon::Stroke::new(Color::default(), 1.0),
                    Transform::default(),
                ));
                let (path, stroke, transform) = new_components.as_mut().unwrap();
                (path, stroke, transform)
            }
        };

        // Update the components
        *stroke = lyon::Stroke::new(bones_path2d.color.into_bevy(), bones_path2d.thickness);
        *path = bones_path2d
            .points
            .iter()
            .copied()
            .enumerate()
            .fold(lyon::PathBuilder::new(), |mut builder, (i, point)| {
                if i > 0 && !bones_path2d.line_breaks.contains(&i) {
                    builder.line_to(point);
                }
                builder.move_to(point);

                builder
            })
            .build();
        *transform = bones_transform.into_bevy();
        // Offset the path towards the camera slightly to make sure it renders on top of a
        // sprite/etc. if it is applied to an entity with both a sprite and a path.
        transform.translation.z += 0.0001;

        // Spawn the shape if it doesn't already exist
        if let Some((path, stroke, transform)) = new_components {
            commands
                .spawn(lyon::ShapeBundle { path, ..default() })
                .insert(transform)
                .insert(stroke)
                .insert(BevyBonesEntity);
        }
    };

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras renderable tile layers
        if !(world
            .components
            .get::<bones::Transform>()
            .borrow()
            .bitset()
            .bit_any()
            && world
                .components
                .get::<bones::Camera>()
                .borrow()
                .bitset()
                .bit_any()
            && world
                .components
                .get::<bones::Path2d>()
                .borrow()
                .bitset()
                .bit_any())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().borrow();
        let path2ds = world.components.get::<bones::Path2d>().borrow();

        // Extract tiles as sprites
        for (_, (path2d, transform)) in entities.iter_with((&path2ds, &transforms)) {
            add_bones_path2d(path2d, transform);
        }
    }

    // Despawn extra path 2ds
    for (ent, ..) in bevy_bones_path2ds {
        commands.entity(ent).despawn()
    }
}
