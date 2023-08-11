//! Bones lib asset integration.

pub use bones_asset::*;

use crate::prelude::*;

/// Install the bones_lib asset plugin.
pub fn plugin(core: &mut Session) {
    let io = get_io();
    let mut server = AssetServer::new(io, Version::parse(env!("CARGO_PKG_VERSION")).unwrap());
    server.load_assets().expect("Could not load assets");
    core.world.insert_resource(server);
}

#[cfg(not(target_arch = "wasm32"))]
fn get_io() -> impl AssetIo {
    use std::path::PathBuf;

    FileAssetIo {
        core_dir: PathBuf::from("assets"),
        packs_dir: PathBuf::from("packs"),
    }
}

#[cfg(target_arch = "wasm32")]
compile_error!("TODO: implement WASM asset IO");
