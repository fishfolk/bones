use bevy_tasks::{IoTaskPool, TaskPool};
use bones_asset::prelude::*;
use glam::{UVec2, Vec2};

use std::path::PathBuf;

/// We will use this as our root asset data, as we will see below.
#[derive(HasSchema, Debug, Default, Clone)]
// Every asset type that will be loaded from a file needs either the `metadata_asset` annotation, or
// the `asset_loader` annotation.
//
// The `metadata_asset` annotation tells the asset loader to load the file from a YAML or JSON file
// with an extension like `.game.yaml`, or `.game.json`.
#[type_data(metadata_asset("game"))]
#[repr(C)]
struct GameMeta {
    // We can add global game settings, for example.
    pub gravity: f32,
    /// Handles allow one asset to reference another asset in the asset files.
    pub players: SVec<Handle<PlayerMeta>>,
}

/// We will use this as our player meta format.
#[derive(HasSchema, Debug, Default, Clone)]
// We want to load players from `.player.yaml` files.
#[type_data(metadata_asset("player"))]
#[repr(C)]
struct PlayerMeta {
    /// We include basic info.
    pub name: String,
    /// This will fail to deserialize if the asset file has a value outside of the u8 range.
    pub age: u8,
    /// And we also reference a separate AtlasMeta asset, which will be loaded from a separate file.
    pub atlas: Handle<AtlasMeta>,
    /// We can include nested structs of our own if they implement HasSchema. This will not be
    /// loaded from a separate file because it is not in a Handle.
    pub stats: PlayerMetaStats,
    /// We can also load key-value data using an SMap
    pub animations: SMap<String, AnimMeta>,
    /// The player's avatar. This is a custom asset type, which we will implement below.
    pub avatar: Handle<Image>,
}

/// Player animation metadata
#[derive(HasSchema, Debug, Default, Clone)]
#[repr(C)]
pub struct AnimMeta {
    fps: f32,
    frames: SVec<u32>,
}

/// Stats used in [`PlayerMeta`].
#[derive(HasSchema, Debug, Clone)]
#[repr(C)]
struct PlayerMetaStats {
    pub speed: f32,
    pub intelligence: f32,
}

// We can also implement custom defaults, so that un-specified fields will be set to the default
// value.
impl Default for PlayerMetaStats {
    fn default() -> Self {
        Self {
            speed: 30.0,
            intelligence: 20.0,
        }
    }
}

/// The atlas metadata referenced by [`PlayerMeta`].
#[derive(HasSchema, Debug, Default, Clone)]
#[type_data(metadata_asset("atlas"))]
#[repr(C)]
struct AtlasMeta {
    /// We can include glam types!
    pub tile_size: Vec2,
    pub grid_size: UVec2,
}

/// We also want to support loading asset packs, so we create a plugin metdata type that will be
/// used for plugin assets.
#[derive(HasSchema, Debug, Default, Clone)]
#[type_data(metadata_asset("plugin"))]
#[repr(C)]
struct PluginMeta {
    /// We'll keep this one simple for now.
    pub description: String,
}

/// We can also make asset types that use a custom asset loader, for example, for images.
#[derive(HasSchema, Debug, Clone, Default)]
// We specify the file extensions and the asset loader to use to load the asset.
#[type_data(asset_loader(["png", "jpg"], ImageAssetLoader))]
#[allow(dead_code)]
struct Image {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

/// Our custom loader for image assets.
struct ImageAssetLoader;
impl AssetLoader for ImageAssetLoader {
    fn load(&self, _ctx: AssetLoadCtx, bytes: &[u8]) -> BoxedFuture<anyhow::Result<SchemaBox>> {
        // We're not going to bother actually loading the image.
        let data = bytes.to_vec();
        Box::pin(async move {
            Ok(SchemaBox::new(Image {
                data,
                width: 0,
                height: 0,
            }))
        })
    }
}

fn main() -> anyhow::Result<()> {
    // Locate the dir that our core asset pack will be loaded from.
    let core_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("assets");
    // Locate the dir that our other asset packs will be loaded from. These are presumably able to
    // be installed by the user for modding, etc.
    let packs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("packs");

    // Create a FileAssetIo to load assets from the filesystem.
    //
    // We can implement different AssetIo implementations for things web builds or other use-cases.
    let io = FileAssetIo::new(&core_dir, &packs_dir);

    // Create an asset server that we can load the assets with. We must provide our AssetIo
    // implementation, and the version of our game, which is used to determine if asset packs are
    // compatible with our game version.
    let asset_server = AssetServer::new(io, Version::new(0, 1, 3));

    // Each asset type needs to have it's schema registered to be loaded from an asset file.
    GameMeta::register_schema();
    PlayerMeta::register_schema();
    AtlasMeta::register_schema();
    PluginMeta::register_schema();

    // Load assets
    let s = asset_server.clone();
    IoTaskPool::init(TaskPool::default);
    println!("Loading Assets...");

    // Spawn a task to load the assets
    IoTaskPool::get()
        .spawn(async move {
            s.load_assets().await.unwrap();
        })
        .detach();

    // Use the load progress listener to wait for assets to load
    loop {
        let mut listener = asset_server.load_progress.listen();
        println!(
            "    Pending: {}\t\tDownloaded: {}\t\tLoaded: {}",
            asset_server.load_progress.to_load(),
            asset_server.load_progress.downloaded(),
            asset_server.load_progress.loaded()
        );
        if asset_server.load_progress.is_finished() {
            break;
        }
        listener.as_mut().wait();
    }
    println!(
        "Done loading {} assets.",
        asset_server.load_progress.loaded()
    );

    // No we can load the root asset handle of the core asset pack. We cast it to the expected type,
    // GameMeta.
    let root_handle = asset_server.core().root.typed::<GameMeta>();

    // We use the handle to get a reference to the `GameMeta`. This would panic if the actual asset
    // type was not `GameMeta`.
    let game_meta = asset_server.get(root_handle);

    dbg!(&*game_meta);
    assert_eq!(game_meta.gravity, 9.8);

    // The GameMeta contains a handle to the player asset, which we can get here.
    for (i, player_handle) in game_meta.players.iter().enumerate() {
        // And we can load the `PlayerMeta` using the handle.
        let player_meta = &asset_server.get(*player_handle);

        dbg!(&**player_meta);

        // And we can load the player's atlas metadata in the same way.
        let atlas_handle = player_meta.atlas;
        let atlas_meta = &asset_server.get(atlas_handle);
        dbg!(&**atlas_meta);

        // let avatar = asset_server.get(player_meta.avatar);
        // dbg!(avatar.data.len(), avatar.width, avatar.height);

        if i == 0 {
            assert_eq!(player_meta.name, "Jane");
            // This should be the default value because it was left unspecified in the asset file.
            assert_eq!(player_meta.stats.intelligence, 20.);
            assert_eq!(atlas_meta.tile_size, Vec2::new(25.5, 30.));
            assert_eq!(atlas_meta.grid_size, UVec2::new(2, 4));
        }
    }

    // We can also check out our loaded asset packs.
    println!("\n===== Asset Packs =====\n");
    for entry in asset_server.packs().iter() {
        let (pack_spec, asset_pack) = (entry.key(), entry.value());
        // Let's load the plugin metadata from the pack.
        let plugin_handle = asset_pack.root.typed::<PluginMeta>();
        let plugin_meta = asset_server.get(plugin_handle);

        // Print the pack name and version and it's description
        println!("{pack_spec}: {}", plugin_meta.description);
    }

    // Finally, there may be some asset packs that are installed, but not compatible with our game
    // version. Let's check for those.
    println!("\n===== Incompatible Asset Packs ====\n");

    // We can iterate over the incompatible packs, and print a message describing the mismatch.
    for entry in asset_server.store.incompabile_packs.iter() {
        let (folder_name, pack_meta) = (entry.key(), entry.value());
        let id = pack_meta.id;
        let version = &pack_meta.version;
        let actual_game_version = &asset_server.game_version();
        let compatible_game_version = &pack_meta.game_version;
        println!(
            "{id}@{version} in folder `{folder_name}` is not compatible with game version \
            {actual_game_version} - pack is compatible with game version {compatible_game_version}"
        );
    }

    Ok(())
}
