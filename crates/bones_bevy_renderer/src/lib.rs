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

mod asset;

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
        app
            // Install the asset loader for .atlas.yaml files.
            .add_asset_loader(asset::TextureAtlasLoader)
            // Add the world sync systems
            .add_system_to_stage(CoreStage::Last, sync_sprites::<W>)
            .add_system_to_stage(CoreStage::Last, sync_atlas_sprites::<W>)
            .add_system_to_stage(CoreStage::Last, sync_cameras::<W>);
    }
}

/// The system that renders the bones world.
fn sync_sprites<W: HasBonesWorld>(
    mut has_init: Local<bool>,
    mut commands: Commands,
    world_resource: Option<ResMut<W>>,
    mut bevy_bones_sprites: Query<
        (Entity, &mut Handle<Image>, &mut Sprite, &mut Transform),
        With<BevyBonesEntity>,
    >,
) {
    let Some(mut world_resource) = world_resource else {
        return;
    };

    let world = world_resource.world();

    if !*has_init {
        world.components.init::<bones::Sprite>();
        world.components.init::<bones::Transform>();
        *has_init = true;
    }

    let entities = world.resources.get::<bones::Entities>();
    let entities = entities.borrow();
    let sprites = world.components.get::<bones::Sprite>();
    let sprites = sprites.borrow();
    let transforms = world.components.get::<bones::Transform>();
    let transforms = transforms.borrow();

    // Sync sprites
    let mut sprites_bitset = sprites.bitset().clone();
    sprites_bitset.bit_and(transforms.bitset());
    let mut bones_sprite_entity_iter = entities.iter_with_bitset(&sprites_bitset);
    for (bevy_ent, mut image, mut sprite, mut transform) in &mut bevy_bones_sprites {
        if let Some(bones_ent) = bones_sprite_entity_iter.next() {
            let bones_sprite = sprites.get(bones_ent).unwrap();
            let bones_transform = transforms.get(bones_ent).unwrap();

            sprite.flip_x = bones_sprite.flip_x;
            sprite.flip_y = bones_sprite.flip_y;
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
}

/// The system that renders the bones world.
fn sync_atlas_sprites<W: HasBonesWorld>(
    mut has_init: Local<bool>,
    mut commands: Commands,
    world_resource: Option<ResMut<W>>,
    mut bevy_bones_atlases: Query<
        (
            Entity,
            &mut Handle<TextureAtlas>,
            &mut TextureAtlasSprite,
            &mut Transform,
        ),
        With<BevyBonesEntity>,
    >,
) {
    let Some(mut world_resource) = world_resource else {
        return;
    };

    let world = world_resource.world();

    if !*has_init {
        world.components.init::<bones::AtlasSprite>();
        world.components.init::<bones::Transform>();
        *has_init = true;
    }

    let entities = world.resources.get::<bones::Entities>();
    let entities = entities.borrow();
    let atlas_sprites = world.components.get::<bones::AtlasSprite>();
    let atlas_sprites = atlas_sprites.borrow();
    let transforms = world.components.get::<bones::Transform>();
    let transforms = transforms.borrow();

    // Sync atlas sprites
    let mut atlas_bitset = atlas_sprites.bitset().clone();
    atlas_bitset.bit_and(transforms.bitset());
    let mut bones_atlas_sprite_entity_iter = entities.iter_with_bitset(&atlas_bitset);
    for (bevy_ent, mut image, mut atlas_sprite, mut transform) in &mut bevy_bones_atlases {
        if let Some(bones_ent) = bones_atlas_sprite_entity_iter.next() {
            let bones_atlas = atlas_sprites.get(bones_ent).unwrap();
            let bones_transform = transforms.get(bones_ent).unwrap();

            *image = bones_atlas.atlas.get_bevy_handle_untyped().typed();
            *transform = bones_transform.into_bevy();

            atlas_sprite.index = bones_atlas.index;
            atlas_sprite.flip_x = bones_atlas.flip_x;
            atlas_sprite.flip_y = bones_atlas.flip_y;
        } else {
            commands.entity(bevy_ent).despawn();
        }
    }
    for bones_ent in bones_atlas_sprite_entity_iter {
        let bones_atlas = atlas_sprites.get(bones_ent).unwrap();
        let bones_transform = transforms.get(bones_ent).unwrap();

        commands.spawn((
            SpriteSheetBundle {
                texture_atlas: bones_atlas.atlas.get_bevy_handle_untyped().typed(),
                transform: bones_transform.into_bevy(),
                ..default()
            },
            BevyBonesEntity,
        ));
    }
}

/// The system that renders the bones world.
fn sync_cameras<W: HasBonesWorld>(
    mut has_init: Local<bool>,
    mut commands: Commands,
    world_resource: Option<ResMut<W>>,
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

    if !*has_init {
        world.components.init::<bones::Transform>();
        world.components.init::<bones::Camera>();
        *has_init = true;
    }

    let entities = world.resources.get::<bones::Entities>();
    let entities = entities.borrow();
    let transforms = world.components.get::<bones::Transform>();
    let transforms = transforms.borrow();
    let cameras = world.components.get::<bones::Camera>();
    let cameras = cameras.borrow();

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
