//! Bevy plugin for rendering Bones framework games.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::marker::PhantomData;

use bevy::{prelude::*, render::camera::ScalingMode};
use bones_lib::prelude::{self as bones, BitSet, IntoBevy};

/// The prelude
pub mod prelude {
    pub use crate::*;
}

/// This is a trait that must be implemented for your Bevy resource containing the bones
/// [`World`][bones::World].
///
/// It gives the [`BonesRendererPlugin`] a way to know how to read the bones world from your world
/// resource.
pub trait HasBonesWorld: Resource {
    /// Return a mutable reference to the bones world stored by the resource.
    fn world(&mut self) -> &mut bones::World;
}

/// The bones renderer plugin.
///
/// This will render the bones world stored in the resource of type `W`.
pub struct BonesRendererPlugin<W: HasBonesWorld> {
    _phantom: PhantomData<W>,
}

impl<W: HasBonesWorld> Default for BonesRendererPlugin<W> {
    fn default() -> Self {
        Self {
            _phantom: default(),
        }
    }
}

impl<W: HasBonesWorld> BonesRendererPlugin<W> {
    /// Create a new [`BonesRendererPlugin`] instance.
    pub fn new() -> Self {
        default()
    }
}

/// Marker component for entities that are rendered in Bevy for bones.
#[derive(Component)]
pub struct BevyBonesEntity;

impl<W: HasBonesWorld> Plugin for BonesRendererPlugin<W> {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::Last, render_world::<W>);
    }
}

/// The system that renders the bones world.
fn render_world<W: HasBonesWorld>(
    mut commands: Commands,
    world_resource: Option<ResMut<W>>,
    mut bevy_bones_sprites: Query<
        (Entity, &mut Handle<Image>, &mut Transform),
        (With<BevyBonesEntity>, Without<Camera>),
    >,
    mut bevy_bones_cameras: Query<
        (
            Entity,
            &mut Camera,
            &mut OrthographicProjection,
            &mut Transform,
        ),
        With<BevyBonesEntity>,
    >,
) {
    let Some(mut world_resource) = world_resource else {
        return;
    };

    let world = world_resource.world();

    let entities = world.resources.get::<bones::Entities>();
    let entities = entities.borrow();
    let sprites = world.components.get::<bones::Sprite>();
    let sprites = sprites.borrow();
    let transforms = world.components.get::<bones::Transform>();
    let transforms = transforms.borrow();
    let cameras = world.components.get::<bones::Camera>();
    let cameras = cameras.borrow();

    // Sync sprites
    let mut sprites_bitset = sprites.bitset().clone();
    sprites_bitset.bit_and(transforms.bitset());
    let mut bones_sprite_entity_iter = entities.iter_with_bitset(&sprites_bitset);
    for (bevy_ent, mut image, mut transform) in &mut bevy_bones_sprites {
        if let Some(bones_ent) = bones_sprite_entity_iter.next() {
            let bones_sprite = sprites.get(bones_ent).unwrap();
            let bones_transform = transforms.get(bones_ent).unwrap();

            *image = bones_sprite.image.get_bevy_handle_untyped().typed();
            *transform = bones_transform.into_bevy();
        } else {
            commands.entity(bevy_ent).despawn();
        }
    }
    for bones_ent in bones_sprite_entity_iter {
        let bones_sprite = sprites.get(bones_ent).unwrap();
        let bones_transform = transforms.get(bones_ent).unwrap();

        commands.spawn((
            SpriteBundle {
                texture: bones_sprite.image.get_bevy_handle_untyped().typed(),
                transform: bones_transform.into_bevy(),
                ..default()
            },
            BevyBonesEntity,
        ));
    }

    // Sync cameras
    let mut cameras_bitset = cameras.bitset().clone();
    cameras_bitset.bit_and(transforms.bitset());
    let mut bones_camera_entity_iter = entities.iter_with_bitset(&cameras_bitset);
    for (bevy_ent, mut camera, mut projection, mut transform) in &mut bevy_bones_cameras {
        if let Some(bones_ent) = bones_camera_entity_iter.next() {
            let bones_camera = cameras.get(bones_ent).unwrap();
            let bones_transform = transforms.get(bones_ent).unwrap();

            camera.is_active = bones_camera.active;
            match projection.scaling_mode {
                ScalingMode::FixedVertical(height) if height != bones_camera.height => {
                    projection.scaling_mode = ScalingMode::FixedVertical(bones_camera.height)
                }
                _ => (),
            }

            *transform = bones_transform.into_bevy();
        } else {
            commands.entity(bevy_ent).despawn();
        }
    }
    for bones_ent in bones_camera_entity_iter {
        let bones_camera = cameras.get(bones_ent).unwrap();
        let bones_transform = transforms.get(bones_ent).unwrap();

        commands.spawn((
            Camera2dBundle {
                camera: Camera {
                    is_active: bones_camera.active,
                    ..default()
                },
                projection: OrthographicProjection {
                    scaling_mode: ScalingMode::FixedVertical(bones_camera.height),
                    ..default()
                },
                transform: bones_transform.into_bevy(),
                ..default()
            },
            BevyBonesEntity,
        ));
    }
}
