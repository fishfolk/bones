use bones_wgpu_renderer::BonesWgpuRenderer;
use bones_framework::prelude::*;

fn main() {
    // Setup logging
    setup_logs!();

    // First create bones game.
    let mut game = Game::new();

    // Create a new session for the game menu. Each session is it's own bones world with it's own
    // plugins, systems, and entities.
    let menu_session = game.sessions.create("menu");
    menu_session
        // Install the default bones_framework plugin for this session
        .install_plugin(DefaultSessionPlugin)
        // Add our menu system to the update stage
        .add_system_to_stage(Update, menu_system);

    BonesWgpuRenderer::new(game).run();
}

/// System to render the home menu.
fn menu_system(ctx: Res<EguiCtx>) {
    egui::CentralPanel::default().show(&ctx, |ui| {
        ui.label("Hello World");
    });
}
