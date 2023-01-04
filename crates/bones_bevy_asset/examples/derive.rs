use bevy::prelude::*;
use bones_bevy_asset::prelude::*;
use bones_lib::prelude as bones;
use serde::Deserialize;

/// Example of an airplane asset.
#[derive(BonesBevyAsset, TypeUlid, Deserialize, Debug)]
#[ulid = "01GNT26ATV1QWAAYP2PA3M5EFT"]
// This allows us to load the asset from files with `.meta.json` and `.meta.yaml` extensions.
#[asset_id = "meta"]
pub struct GameMeta {
    pub title: String,
    #[asset(deserialize_only)]
    pub info: GameInfo,
    pub players: Vec<bones::Handle<PlayerMeta>>,
}

#[derive(serde::Deserialize, Debug)]
pub struct GameInfo {
    pub description: String,
    pub authors: Vec<String>,
}

#[derive(BonesBevyAsset, TypeUlid, Deserialize, Debug)]
#[ulid = "01GNT6APZEBGYFJ8SAKX2Q7TX2"]
#[asset_id = "player"]
pub struct PlayerMeta {
    pub name: String,
}

#[derive(Resource)]
pub struct GameMetaHandle(pub Handle<GameMeta>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_bones_asset::<GameMeta>()
        .add_bones_asset::<PlayerMeta>()
        .add_startup_system(|mut commands: Commands, asset_server: Res<AssetServer>| {
            let handle = asset_server.load("game.meta.yaml");
            commands.insert_resource(GameMetaHandle(handle));
        })
        .add_system(
            |mut done: Local<bool>,
             game_meta_assets: Res<Assets<GameMeta>>,
             player_assets: Res<Assets<PlayerMeta>>,
             game_meta_handle: Option<Res<GameMetaHandle>>| {
                if *done {
                    return;
                }
                let Some(game_meta_handle) = game_meta_handle else {
                    return;
                };
                let Some(game_meta) = game_meta_assets.get(&game_meta_handle.0) else {
                    return;
                };

                let player_bevy_handles = game_meta
                    .players
                    .iter()
                    .map(|x| x.get_bevy_handle())
                    .collect::<Vec<_>>();

                if player_bevy_handles
                    .iter()
                    .all(|x| player_assets.get(x).is_some())
                {
                    *done = true;
                    dbg!(&game_meta);
                    for player_handle in &game_meta.players {
                        let handle = player_handle.get_bevy_handle();

                        let player_meta = player_assets.get(&handle);

                        dbg!(&player_meta);
                    }
                }
            },
        )
        .run();
}
