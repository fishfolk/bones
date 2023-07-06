use std::path::PathBuf;

use bones_asset::{AssetPack, AssetServer, FileAssetIo};

#[test]
fn smoke1() -> anyhow::Result<()> {
    let default_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets");
    let packs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("packs");
    let pack1_file = std::fs::read_to_string(packs_dir.join("pack1").join("pack.yaml"))?;
    let io = FileAssetIo {
        default_dir,
        packs_dir,
    };
    let mut asset_server = AssetServer::new();

    let pack: AssetPack = serde_yaml::from_str(&pack1_file)?;

    assert_eq!(pack.name, "Pack 1");
    assert_eq!(pack.id.prefix(), "pack-1");

    Ok(())
}
