use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

//
// NOTE: You must run this example from within the `demos/assets_minimal` folder. Also, be sure to
// look at the `demo/assets_minimal/assets` folder to see the asset files for this example.
//

/// Create our "root" asset type.
#[derive(HasSchema, Clone, Default)]
#[repr(C)]
// We must mark this as a metadata asset, and we set the type to "game".
//
// This means that any files with names like `game.yaml`, `game.yml`, `game.json`, `name.game.yaml`,
// etc. will be loaded as a `GameMeta` asset.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    title: String,
}

fn main() {
    // First create bones game.
    let mut game = Game::new();

    game
        // We initialize the asset server.
        .init_shared_resource::<AssetServer>()
        // We must register all of our asset types before they can be loaded.
        .register_asset::<GameMeta>();

    // Create a new session for the game menu. Each session is it's own bones world with it's own
    // plugins, systems, and entities.
    let menu_session = game.sessions.create("menu");
    menu_session
        // Install the default bones_framework plugin for this session
        .install_plugin(DefaultPlugin)
        // Add our menu system to the update stage
        .add_system_to_stage(Update, menu_system);

    BonesBevyRenderer::new(game).app().run();
}

/// System to render the home menu.
fn menu_system(
    egui_ctx: Res<EguiCtx>,
    // We can access our root asset by using the Root parameter.
    meta: Root<GameMeta>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(&egui_ctx, |ui| {
            // Use the title that has been loaded from the asset
            ui.heading(&meta.title);
        });
}
