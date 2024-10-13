#![allow(dead_code)]

use std::path::PathBuf;

use bones_framework::prelude::*;
use futures_lite::{
    future::{block_on, yield_now},
    FutureExt,
};

fn create_world() -> World {
    let mut world = World::default();

    let core_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets");
    let packs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("packs");
    let io = FileAssetIo::new(&core_dir, &packs_dir);

    let mut asset_server = world.init_resource::<AssetServer>();
    asset_server.set_io(io);

    let scope = async move {
        asset_server.load_assets().await.expect("load test assets");
        while !asset_server.load_progress.is_finished() {
            yield_now().await;
        }
    };
    block_on(scope.boxed());

    world
}

#[derive(Clone, Default, HasSchema)]
#[type_data(metadata_asset("game"))]
#[repr(C)]
struct GameMeta {
    data: String,
    important_numbers: SVec<i32>,
}

#[derive(Clone, Default, HasSchema)]
#[type_data(metadata_asset("root"))]
#[repr(C)]
struct PackMeta {
    label: String,
    other_numbers: SVec<i32>,
}

fn init() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        GameMeta::register_schema();
        PackMeta::register_schema();

        bevy_tasks::IoTaskPool::init(|| bevy_tasks::TaskPoolBuilder::new().num_threads(1).build());
        setup_logs!();
    });
}

#[test]
#[cfg(not(miri))]
fn core_root_data() {
    init();
    let world = create_world();
    let actual = world.run_system(|root: Root<GameMeta>| root.data.clone(), ());
    assert_eq!(actual, "abc".to_string());
}

#[test]
#[cfg(not(miri))]
fn supplementary_packs_root_data() {
    init();

    let world = create_world();

    let actual = world.run_system(
        |packs: Packs<PackMeta>| {
            let mut data = packs
                .iter()
                .flat_map(|p| p.other_numbers.clone().into_iter())
                .collect::<Vec<_>>();
            data.sort();
            data
        },
        (),
    );

    assert_eq!(actual, vec![10, 20, 30, 100, 200, 300]);
}

#[test]
#[cfg(not(miri))]
fn all_packs_root_data() {
    init();

    let world = create_world();

    let actual = world.run_system(
        |packs: AllPacksData<GameMeta, PackMeta>| {
            let mut data = packs
                .iter_with(
                    |core| core.important_numbers.iter().copied(),
                    |pack| pack.other_numbers.iter().copied(),
                )
                .collect::<Vec<_>>();
            data.sort();
            data
        },
        (),
    );

    assert_eq!(actual, vec![1, 2, 3, 10, 20, 30, 100, 200, 300]);
}
