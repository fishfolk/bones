use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

//
// NOTE: You must run this example from within the `demos/asset_packs` folder. Also, be sure to
// look at the `assets/` and `packs/` folders to see the asset files for this example.
//

/// Our "core" asset type.
#[derive(HasSchema, Clone, Default)]
#[repr(C)]
// We must mark this as a metadata asset, and we set the type to "game".
//
// This means that any files with names like `game.yaml`, `game.yml`, `game.json`, `name.game.yaml`,
// etc. will be loaded as a `GameMeta` asset.
#[type_data(metadata_asset("game"))]
struct GameMeta {
    title: String,
    core_items: SVec<i32>,
}

/// Our "supplementary" asset type.
#[derive(HasSchema, Clone, Default)]
#[repr(C)]
#[type_data(metadata_asset("data"))]
struct PackMeta {
    items: SVec<i32>,
}

fn main() {
    // Setup logging
    setup_logs!();

    // First create bones game.
    let mut game = Game::new();

    game
        // We initialize the asset server.
        .init_shared_resource::<AssetServer>();

    // We must register all of our asset types before they can be loaded by the asset server. This
    // may be done by calling schema() on each of our types, to register them with the schema
    // registry.
    GameMeta::register_schema();
    PackMeta::register_schema();

    // Create a new session for the game menu. Each session is it's own bones world with it's own
    // plugins, systems, and entities.
    game.sessions
        .create_with("menu", |builder: &mut SessionBuilder| {
            // Install the default bones_framework plugin for this session
            builder
                .install_plugin(DefaultSessionPlugin)
                // Add our menu system to the update stage
                .add_system_to_stage(Update, menu_system);
        });

    BonesBevyRenderer::new(game).app().run();
}

/// System to render the home menu.
fn menu_system(
    egui_ctx: Res<EguiCtx>,
    core_meta: Root<GameMeta>,
    all_packs: AllPacksData<GameMeta, PackMeta>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().outer_margin(egui::Margin::same(32.0)))
        .show(&egui_ctx, |ui| {
            // Use the title that has been loaded from the asset
            ui.heading(&core_meta.title);

            ui.separator();

            ui.label(egui::RichText::new("Items from all asset packs:"));

            // Show the numbers from all of the asset packs
            egui::Grid::new("pack-items").num_columns(1).show(ui, |ui| {
                for item in all_packs.iter_with(
                    |core| core.core_items.iter().copied(),
                    |pack| pack.items.iter().copied(),
                ) {
                    ui.label(item.to_string());
                    ui.end_row();
                }
            });
        });
}
