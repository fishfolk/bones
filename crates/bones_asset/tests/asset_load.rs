use bones_asset::prelude::*;
use glam::{UVec2, Vec2};

mod dummy_io;
use dummy_io::DummyIo;

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
    pub tile_size: UVec2,
}

#[test]
fn asset_load1() -> anyhow::Result<()> {
    // Create a file asset IO to load assets from the filesystem.
    let io = DummyIo::new([
        ("pack.yaml", include_bytes!("./assets/pack.yaml").to_vec()),
        (
            "root.game.yaml",
            include_bytes!("./assets/root.game.yaml").to_vec(),
        ),
    ]);

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
    assert_eq!(data.player.tile_size, UVec2::new(5, 6));
    assert_eq!(data.player.name, "John");

    Ok(())
}
