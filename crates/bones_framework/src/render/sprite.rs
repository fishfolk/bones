//! Sprite rendering components.

use crate::prelude::*;

/// Image component.
#[derive(Clone, HasSchema, Debug)]
#[schema(opaque, no_default)]
#[type_data(asset_loader(["png", "jpg", "jpeg"], ImageAssetLoader))]
pub enum Image {
    /// Loaded image data
    Data(image::DynamicImage),
    /// A reference to image data stored in the external bones renderer.
    External(u32),
}

struct ImageAssetLoader;
impl AssetLoader for ImageAssetLoader {
    fn load(&self, bytes: Vec<u8>) -> anyhow::Result<SchemaBox> {
        Ok(SchemaBox::new(Image::Data(image::load_from_memory(
            &bytes,
        )?)))
    }
}

/// An atlas image asset type, contains no data, but [`Handle<Atlas>`] is still useful becaause it
/// uniquely represents an atlas asset that may be rendered outside of the core.
#[derive(Copy, Clone, HasSchema, Debug, Default)]
#[repr(C)]
pub struct Atlas;

/// A 2D sprite component
#[derive(Clone, HasSchema, Debug, Default)]
#[repr(C)]
pub struct Sprite {
    /// The sprite's color tint
    pub color: Color,
    /// The sprite image handle.
    pub image: Handle<Image>,
    /// Whether or not the flip the sprite horizontally.
    pub flip_x: bool,
    /// Whether or not the flip the sprite vertically.
    pub flip_y: bool,
}

/// An animated sprite component.
///
/// Represents one or more [`Atlas`]s stacked on top of each other, and possibly animated through a
/// range of frames out of the atlas.
#[derive(Debug, Default, Clone, HasSchema)]
#[repr(C)]
pub struct AtlasSprite {
    /// The sprite's color tint
    pub color: Color,
    /// This is the current index in the animation, with an `idx` of `0` meaning that the index in
    /// the sprite sheet will be `start`.
    ///
    /// If the idx is greater than `end - start`, then the animation will loop around.
    pub index: usize,
    /// The atlas handle.
    pub atlas: Handle<Atlas>,
    /// Whether or not the flip the sprite horizontally.
    pub flip_x: bool,
    /// Whether or not the flip the sprite vertically.
    pub flip_y: bool,
}

impl AtlasSprite {
    /// Create a new [`AtlasSprite`] from the given atlas handle.
    pub fn new(atlas: Handle<Atlas>) -> Self {
        Self {
            atlas,
            color: Color::WHITE,
            ..default()
        }
    }
}
