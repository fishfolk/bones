use crate::texture::Texture;
use std::sync::Arc;

use crate::*;
use bones_framework::prelude::{self as bones, BitSet, ComponentIterBitset};

/// Functions used to load sprites, atlas sprites and tile sprites, and update them.

// Uniform data struct matching the WGSL `AtlasSpriteUniform` layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AtlasSpriteUniform {
    // Atlas parameters
    pub tile_size: [f32; 2],
    pub image_size: [f32; 2],
    pub padding: [f32; 2], //TODO Check if it really works
    pub offset: [f32; 2],  //TODO Check if it really works
    pub columns: u32,
    pub index: u32,

    // State flags
    pub use_atlas: u32,
    pub flip_x: u32,
    pub flip_y: u32,
    pub _pad: [u32; 3], // Explicit padding

    // Color tint
    pub color_tint: [f32; 4],
}

#[derive(bones_schema::HasSchema, Clone)]
#[repr(C)]
#[schema(opaque)]
#[schema(no_default)]
pub struct AtlasSpriteBuffer(Arc<wgpu::Buffer>);

impl AtlasSpriteUniform {
    pub fn from_atlas_sprite(atlas_sprite: &bones::AtlasSprite, atlas: &bones::Atlas) -> Self {
        let image_size = [
            atlas.offset.x + ((atlas.tile_size.x + atlas.padding.x) * atlas.columns as f32),
            atlas.offset.y + ((atlas.tile_size.y + atlas.padding.y) * atlas.rows as f32),
        ];

        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: atlas_sprite.index,
            image_size,
            use_atlas: 1,
            flip_x: atlas_sprite.flip_x as u32,
            flip_y: atlas_sprite.flip_y as u32,
            color_tint: atlas_sprite.color.as_rgba_f32(),
            ..Default::default()
        }
    }

    pub fn from_tile(tile: &bones::Tile, atlas: &bones::Atlas) -> Self {
        let image_size = [
            atlas.offset.x + ((atlas.tile_size.x + atlas.padding.x) * atlas.columns as f32),
            atlas.offset.y + ((atlas.tile_size.y + atlas.padding.y) * atlas.rows as f32),
        ];

        Self {
            tile_size: atlas.tile_size.into(),
            columns: atlas.columns,
            padding: atlas.padding.into(),
            offset: atlas.offset.into(),
            index: tile.idx,
            image_size,
            use_atlas: 1,
            flip_x: tile.flip_x as u32,
            flip_y: tile.flip_y as u32,
            color_tint: tile.color.as_rgba_f32(),
            ..Default::default()
        }
    }

    pub fn from_sprite(sprite: &bones::Sprite) -> Self {
        Self {
            color_tint: sprite.color.as_rgba_f32(),
            flip_x: sprite.flip_x as u32,
            flip_y: sprite.flip_y as u32,
            use_atlas: 0,
            ..Default::default()
        }
    }
}

pub fn load_sprite(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let texture_sender = game.shared_resource_cell::<TextureSender>().unwrap();
    let pixel_art = game.shared_resource_cell::<PixelArt>().unwrap();

    for (session_name, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let sprites = session.world.component::<bones::Sprite>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();
        let mut texture_loaded = session.world.component_mut::<TextureLoaded>();

        let mut not_loaded = texture_loaded.bitset().clone();
        not_loaded.bit_not();
        not_loaded.bit_and(sprites.bitset());

        for entity in entities.iter_with_bitset(&not_loaded) {
            let Some(sprite) = sprites.get(entity) else {
                unreachable!();
            };

            //Load and send texture
            let assets = assets.borrow().unwrap();
            let image = assets.get(sprite.image);
            if let bones::Image::Data(img) = &*image {
                let texture = Arc::new(
                    Texture::from_image(
                        device.borrow().unwrap().get(),
                        queue.borrow().unwrap().get(),
                        img,
                        None,
                        pixel_art.borrow().unwrap().0,
                    )
                    .unwrap(),
                );

                let atlas_uniform = AtlasSpriteUniform {
                    use_atlas: 0,
                    flip_x: sprite.flip_x as u32,
                    flip_y: sprite.flip_y as u32,
                    color_tint: sprite.color.as_rgba_f32(),
                    ..Default::default()
                };

                let atlas_sprite_buffer =
                    Arc::new(device.borrow().unwrap().get().create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Atlas Sprite Buffer"),
                            contents: bytemuck::cast_slice(&[atlas_uniform]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        },
                    ));

                //Add buffer to bones so we can update it
                buffers.insert(entity, AtlasSpriteBuffer(atlas_sprite_buffer.clone()));

                texture_sender
                    .borrow()
                    .unwrap()
                    .0
                    .send((texture, entity, atlas_sprite_buffer, *session_name))
                    .unwrap();
                texture_loaded.insert(entity, TextureLoaded);
            } else {
                unreachable!()
            };
        }
    }
}

pub fn load_atlas_sprite(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let texture_sender = game.shared_resource_cell::<TextureSender>().unwrap();
    let pixel_art = game.shared_resource_cell::<PixelArt>().unwrap();

    for (session_name, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let atlas_sprites = session.world.component::<bones::AtlasSprite>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();
        let mut texture_loaded = session.world.component_mut::<TextureLoaded>();

        let mut not_loaded = texture_loaded.bitset().clone();
        not_loaded.bit_not();
        not_loaded.bit_and(atlas_sprites.bitset());

        for entity in entities.iter_with_bitset(&not_loaded) {
            let Some(atlas_sprite) = atlas_sprites.get(entity) else {
                unreachable!();
            };

            //Load and send texture
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(atlas_sprite.atlas);
            let image = assets.get(atlas.image);
            if let bones::Image::Data(img) = &*image {
                let texture = Arc::new(
                    Texture::from_image(
                        device.borrow().unwrap().get(),
                        queue.borrow().unwrap().get(),
                        img,
                        None,
                        pixel_art.borrow().unwrap().0,
                    )
                    .unwrap(),
                );
                // create and send the atlas sprite uniform along with the texture and entity
                let uniform = AtlasSpriteUniform::from_atlas_sprite(
                    atlas_sprite,
                    &assets.get(atlas_sprite.atlas),
                );

                let atlas_sprite_buffer =
                    Arc::new(device.borrow().unwrap().get().create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Atlas Sprite Buffer"),
                            contents: bytemuck::cast_slice(&[uniform]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        },
                    ));

                //Add buffer to bones so we can update it
                buffers.insert(entity, AtlasSpriteBuffer(atlas_sprite_buffer.clone()));

                texture_sender
                    .borrow()
                    .unwrap()
                    .0
                    .send((texture, entity, atlas_sprite_buffer, *session_name))
                    .unwrap();
                texture_loaded.insert(entity, TextureLoaded);
            } else {
                unreachable!()
            };
        }
    }
}

pub fn load_tile_sprite(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let device = game.shared_resource_cell::<WgpuDevice>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();
    let texture_sender = game.shared_resource_cell::<TextureSender>().unwrap();
    let pixel_art = game.shared_resource_cell::<PixelArt>().unwrap();

    for (session_name, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();
        let mut texture_loaded = session.world.component_mut::<TextureLoaded>();

        let mut not_loaded = texture_loaded.bitset().clone();
        not_loaded.bit_not();
        not_loaded.bit_and(tile_layers.bitset());

        for layer_ent in entities.iter_with_bitset(&not_loaded) {
            let Some(tile_layer) = tile_layers.get(layer_ent) else {
                unreachable!();
            };

            //Load and send texture
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(tile_layer.atlas);
            let image = assets.get(atlas.image);
            if let bones::Image::Data(img) = &*image {
                let texture = Arc::new(
                    Texture::from_image(
                        device.borrow().unwrap().get(),
                        queue.borrow().unwrap().get(),
                        img,
                        None,
                        pixel_art.borrow().unwrap().0,
                    )
                    .unwrap(),
                );

                for (tile_pos_idx, tile) in tile_layer.tiles.iter().enumerate() {
                    let Some(tile_ent) = tile else {
                        continue;
                    };
                    let Some(tile) = tiles.get(*tile_ent) else {
                        panic!("Couldn't find tile entity!");
                    };
                    // create and send the atlas sprite uniform along with the texture and entity
                    let uniform =
                        AtlasSpriteUniform::from_tile(tile, &assets.get(tile_layer.atlas));

                    let atlas_sprite_buffer =
                        Arc::new(device.borrow().unwrap().get().create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some("Atlas Sprite Buffer"),
                                contents: bytemuck::cast_slice(&[uniform]),
                                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            },
                        ));

                    //Add buffer to bones so we can update it
                    buffers.insert(*tile_ent, AtlasSpriteBuffer(atlas_sprite_buffer.clone()));

                    texture_sender
                        .borrow()
                        .unwrap()
                        .0
                        .send((
                            texture.clone(),
                            *tile_ent,
                            atlas_sprite_buffer,
                            *session_name,
                        ))
                        .unwrap();
                }
            } else {
                unreachable!()
            };
            texture_loaded.insert(layer_ent, TextureLoaded);
        }
    }
}

// System for updating atlas uniforms
pub fn update_atlas_uniforms(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let atlases = session.world.component::<bones::AtlasSprite>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();

        for (_, (atlas_sprite, atlas_sprite_buffer)) in entities.iter_with((&atlases, &mut buffers))
        {
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(atlas_sprite.atlas).clone();
            let uniform = AtlasSpriteUniform::from_atlas_sprite(atlas_sprite, &atlas);
            queue.borrow().unwrap().get().write_buffer(
                &atlas_sprite_buffer.0,
                0,
                bytemuck::bytes_of(&uniform),
            );
        }
    }
}

// System for updating sprite uniforms
pub fn update_sprite_uniforms(game: &mut bones::Game) {
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let sprites = session.world.component::<bones::Sprite>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();

        for (_, (sprite, atlas_sprite_buffer)) in entities.iter_with((&sprites, &mut buffers)) {
            let uniform = AtlasSpriteUniform::from_sprite(sprite);
            queue.borrow().unwrap().get().write_buffer(
                &atlas_sprite_buffer.0,
                0,
                bytemuck::bytes_of(&uniform),
            );
        }
    }
}

// System for updating tiles uniforms
pub fn update_tiles_uniforms(game: &mut bones::Game) {
    let assets = game.shared_resource_cell::<bones::AssetServer>().unwrap();
    let queue = game.shared_resource_cell::<WgpuQueue>().unwrap();

    for (_, session) in game.sessions.iter_mut() {
        if !session.visible {
            continue;
        }

        let entities = session.world.resource::<bones::Entities>();
        let tile_layers = session.world.component::<bones::TileLayer>();
        let tiles = session.world.component::<bones::Tile>();
        let mut buffers = session.world.component_mut::<AtlasSpriteBuffer>();

        for (_, (tile_layer, atlas_sprite_buffer)) in
            entities.iter_with((&tile_layers, &mut buffers))
        {
            let assets = assets.borrow().unwrap();
            let atlas = assets.get(tile_layer.atlas).clone();
            for tile in &tile_layer.tiles {
                let Some(tile) = tile else {
                    continue;
                };
                let Some(tile) = tiles.get(*tile) else {
                    panic!("Couldn't find tile entity!");
                };

                let uniform = AtlasSpriteUniform::from_tile(tile, &atlas);
                queue.borrow().unwrap().get().write_buffer(
                    &atlas_sprite_buffer.0,
                    0,
                    bytemuck::bytes_of(&uniform),
                );
            }
        }
    }
}
