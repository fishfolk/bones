use bones_lib::prelude::*;

#[derive(HasSchema, Clone, Debug, Default)]
#[repr(C)]
struct GameMeta;

fn main() {
    let mut core = BonesCore::new();
    core.install_plugin(bones_lib::plugin);

    let asset_server = core.world.get_resource::<AssetServer>().unwrap();
    let mut _asset_server = asset_server.borrow_mut();
}
