use std::path::PathBuf;

use bones_asset::{AssetIo, AssetServer, FileAssetIo};

#[test]
fn smoke1() -> anyhow::Result<()> {
    let default_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets");
    let packs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("packs");
    let io = FileAssetIo {
        default_dir,
        packs_dir,
    };
    let mut asset_server = AssetServer::new();

    let loaded_assets = io.load_all(&mut asset_server)?;

    let default_handle = loaded_assets.default;
    let default_asset = asset_server.get_untyped(&default_handle).unwrap();
    dbg!(default_asset);

    let john_handle = default_asset
        .data
        .as_metadata()
        .unwrap()
        .get_key("players")
        .unwrap()
        .get(0)
        .unwrap()
        .as_asset()
        .unwrap();
    let john_asset = asset_server.get_untyped(john_handle).unwrap();
    dbg!(john_asset);
    assert_eq!(
        john_asset
            .data
            .as_metadata()
            .unwrap()
            .get_key("name")
            .unwrap()
            .as_string()
            .unwrap(),
        "John"
    );
    let jane_handle = default_asset
        .data
        .as_metadata()
        .unwrap()
        .get_key("players")
        .unwrap()
        .get(1)
        .unwrap()
        .as_asset()
        .unwrap();
    let jane_asset = asset_server.get_untyped(jane_handle).unwrap();
    dbg!(jane_asset);
    assert_eq!(
        jane_asset
            .data
            .as_metadata()
            .unwrap()
            .get_key("name")
            .unwrap()
            .as_string()
            .unwrap(),
        "Jane"
    );
    assert_eq!(
        *jane_asset
            .data
            .as_metadata()
            .unwrap()
            .get_key("age")
            .unwrap()
            .as_number()
            .unwrap(),
        25.0
    );
    assert_eq!(
        jane_asset
            .data
            .as_metadata()
            .unwrap()
            .get_key("favorite_things")
            .unwrap()
            .as_vec()
            .unwrap()
            .get(0)
            .unwrap()
            .as_string()
            .unwrap(),
        "cupcakes"
    );

    let jane_atlas_handle = jane_asset
        .data
        .as_metadata()
        .unwrap()
        .get_key("atlas")
        .unwrap()
        .as_asset()
        .unwrap();
    let jane_atlas_asset = asset_server.get_untyped(jane_atlas_handle).unwrap();
    dbg!(jane_atlas_asset);

    assert_eq!(
        jane_atlas_asset
            .data
            .as_metadata()
            .unwrap()
            .get_key("tile_size")
            .unwrap()
            .as_vec()
            .unwrap()
            .iter()
            .map(|x| *x.as_number().unwrap())
            .collect::<Vec<_>>(),
        [25., 30.]
    );

    let pack1_handle = loaded_assets.packs.get("pack1").unwrap();
    let pack1_asset = asset_server.get_untyped(pack1_handle).unwrap();
    dbg!(pack1_asset);

    let dummy_handle = pack1_asset
        .data
        .as_metadata()
        .unwrap()
        .get_key("weapons")
        .unwrap()
        .get(0)
        .unwrap()
        .as_asset()
        .unwrap();
    let dummy_asset = asset_server.get_untyped(dummy_handle).unwrap();
    assert_eq!(
        dummy_asset
            .data
            .as_metadata()
            .unwrap()
            .get_key("example")
            .unwrap()
            .as_string()
            .unwrap(),
        "dummy"
    );

    let pack2_handle = loaded_assets.packs.get("pack2").unwrap();
    let pack2_asset = asset_server.get_untyped(pack2_handle).unwrap();
    dbg!(pack2_asset);

    assert_eq!(
        pack2_asset
            .data
            .as_metadata()
            .unwrap()
            .get_key("name")
            .unwrap()
            .as_string()
            .unwrap(),
        "Pack 2"
    );
    // panic!();

    Ok(())
}
