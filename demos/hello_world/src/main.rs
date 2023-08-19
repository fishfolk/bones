use bones_bevy_renderer::BonesBevyRenderer;
use bones_framework::prelude::*;

fn main() {
    let mut game = Game::new();

    let menu_session = game.sessions.create("menu");

    menu_session
        .install_plugin(DefaultPlugin)
        .stages
        .add_system_to_stage(Update, menu_system);

    BonesBevyRenderer::new(game).app().run();
}

fn menu_system(egui_ctx: Res<EguiCtx>) {
    egui::CentralPanel::default().show(&egui_ctx, |ui| {
        ui.label("Hello World");
    });
}
