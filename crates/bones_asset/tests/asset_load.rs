use std::path::PathBuf;

use bones_asset::prelude::*;

#[derive(HasSchema, Debug, Default, Clone)]
#[repr(C)]
struct GameMeta {
    pub gravity: f32,
    pub player: PlayerMeta,
    // pub players: Vec<Handle<PlayerMeta>>,
}

#[derive(HasSchema, Debug, Default, Clone)]
#[repr(C)]
struct PlayerMeta {
    name: String,
    // atlas: Handle<AtlasMeta>,
    // favorite_things: Vec<String>,
    age: u8,
}

#[derive(HasSchema, Debug, Default, Clone)]
#[repr(C)]
struct AtlasMeta {
    tile_size: glam::UVec2,
    columns: u32,
    rows: u32,
}

#[test]
fn asset_load1() -> anyhow::Result<()> {
    // Locate our core asset dir and asset pack dir
    let core_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets");
    let packs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("packs");

    // Create a file asset IO to load assets from the filesystem.
    let io = FileAssetIo {
        core_dir,
        packs_dir,
    };

    // Create an asset server that we can load the assets with.
    let mut asset_server = AssetServer::new(io);

    // Register our GameMeta type as a core schema with the "game" file extension, so that all
    // `.game.yaml` files will be loaded with the GameMeta schema.
    asset_server.register_core_schema::<GameMeta>("game");
    // Do the same for our atlas and player metadata
    asset_server.register_core_schema::<PlayerMeta>("player");
    asset_server.register_core_schema::<AtlasMeta>("atlas");

    // Load all of the assets
    asset_server.load_assets()?;

    let data: &GameMeta = asset_server
        .store
        .assets
        .values()
        .next() // We only have one asset, so we can just get it
        .unwrap()
        .data
        .cast();

    dbg!(&data);
    assert_eq!(data.gravity, 9.8);
    assert_eq!(data.player.name, "John");

    Ok(())
}
