use std::path::PathBuf;

use bones_asset::prelude::*;
use glam::Vec2;

#[derive(HasSchema, Debug, Default, Clone)]
#[repr(C)]
struct GameMeta {
    pub gravity: f32,
    pub player: PlayerMeta,
}

#[derive(HasSchema, Debug, Default, Clone)]
#[repr(C)]
struct PlayerMeta {
    pub name: String,
    pub age: u8,
    // FIXME:!! Segfault when adding loading this field sometimes !!
    pub collision_size: Vec2,
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
    assert_eq!(data.player.collision_size, Vec2::new(1.2, 3.4));
    assert_eq!(data.player.name, "John");

    Ok(())
}
