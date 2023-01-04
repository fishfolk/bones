use std::ffi::OsStr;

use bevy::{asset::LoadedAsset, sprite::TextureAtlas};
use bones_bevy_asset::BonesBevyAssetLoad;
use glam::Vec2;

/// The YAML/JSON metadata format for texture atlases
#[derive(serde::Deserialize)]
pub struct AtlasMeta {
    pub image: bones_lib::asset::Handle<bones_lib::render::sprite::Image>,
    pub tile_size: Vec2,
    pub columns: usize,
    pub rows: usize,
    #[serde(default)]
    pub padding: Option<Vec2>,
    #[serde(default)]
    pub offset: Option<Vec2>,
}

/// An asset loader for [`TextureAtlas`]s from JSON or YAML.
pub struct TextureAtlasLoader;

impl bevy::asset::AssetLoader for TextureAtlasLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let self_path = &load_context.path().to_owned();
            let mut dependencies = Vec::with_capacity(1);

            let mut meta: AtlasMeta = if self_path.extension() == Some(OsStr::new("json")) {
                serde_json::from_slice(bytes)?
            } else {
                serde_yaml::from_slice(bytes)?
            };

            meta.image.load(load_context, &mut dependencies);

            load_context.set_default_asset(
                LoadedAsset::new(TextureAtlas::from_grid(
                    meta.image.get_bevy_handle_untyped().typed(),
                    meta.tile_size,
                    meta.columns,
                    meta.rows,
                    meta.padding,
                    meta.offset,
                ))
                .with_dependencies(dependencies),
            );

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["atlas.json", "atlas.yml", "atlas.yaml"]
    }
}
