use super::*;
use bevy_egui::EguiContext;

/// Startup system to load egui fonts and textures.
pub fn setup_egui(world: &mut World) {
    world.resource_scope(|world: &mut World, mut game: Mut<BonesGame>| {
        let ctx = {
            let mut egui_query = world.query_filtered::<&mut EguiContext, With<Window>>();
            let mut egui_ctx = egui_query.get_single_mut(world).unwrap();
            egui_ctx.get_mut().clone()
        };

        // Insert the egui context as a shared resource
        // Broke when updating bones egui to 0.30
        //game.insert_shared_resource(bones::EguiCtx(ctx.clone()));

        if let Some(bones_assets) = &game.asset_server() {
            update_egui_fonts(&ctx, bones_assets);

            // Insert the bones egui textures
            ctx.data_mut(|map| {
                map.insert_temp(
                    bevy_egui::egui::Id::null(),
                    game.shared_resource_cell::<bones::EguiTextures>().unwrap(),
                );
            });
        }
    });
}

pub fn egui_input_hook(
    mut egui_query: Query<&mut bevy_egui::EguiInput, With<Window>>,
    mut game: ResMut<BonesGame>,
) {
    if let Some(hook) = game.shared_resource_cell::<bones::EguiInputHook>() {
        let hook = hook.borrow().unwrap();
        let mut egui_input = egui_query.get_single_mut().unwrap();
        // Broke when updating bones egui to 0.30
        //(hook.0)(&mut game, &mut egui_input);
    }
}

pub fn sync_egui_settings(
    game: Res<BonesGame>,
    mut bevy_egui_settings: ResMut<bevy_egui::EguiSettings>,
) {
    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        let world = &session.world;

        if let Some(settings) = world.get_resource::<bones::EguiSettings>() {
            bevy_egui_settings.scale_factor = settings.scale;
        }
    }
}

pub fn update_egui_fonts(ctx: &bevy_egui::egui::Context, bones_assets: &bones::AssetServer) {
    use bevy_egui::egui;
    let mut fonts = egui::FontDefinitions::default();

    for entry in bones_assets.store.assets.iter() {
        /*Broke when updating bones egui to 0.30
        let asset = entry.value();
        if let Ok(font) = asset.try_cast_ref::<bones::Font>() {
            let previous = fonts
                .font_data
                .insert(font.family_name.to_string(), font.data.clone());
            if previous.is_some() {
                warn!(
                    name=%font.family_name,
                    "Found two fonts with the same family name, using \
                    only the latest one"
                );
            }
            fonts
                .families
                .entry(egui::FontFamily::Name(font.family_name.clone()))
                .or_default()
                .push(font.family_name.to_string());
        }
        */
    }

    ctx.set_fonts(fonts);
}

pub fn default_load_progress(asset_server: &bones::AssetServer, ctx: &bevy_egui::egui::Context) {
    use bevy_egui::egui;
    let errored = asset_server.load_progress.errored();

    egui::CentralPanel::default().show(ctx, |ui| {
        let height = ui.available_height();
        let ctx = ui.ctx().clone();

        let space_size = 0.03;
        let spinner_size = 0.07;
        let text_size = 0.034;
        ui.vertical_centered(|ui| {
            ui.add_space(height * 0.3);

            if errored > 0 {
                ui.label(
                    egui::RichText::new("⚠")
                        .color(egui::Color32::RED)
                        .size(height * spinner_size),
                );
                ui.add_space(height * space_size);
                ui.label(
                    egui::RichText::new(format!(
                        "Error loading {errored} asset{}.",
                        if errored > 1 { "s" } else { "" }
                    ))
                    .color(egui::Color32::RED)
                    .size(height * text_size * 0.75),
                );
            } else {
                ui.add(egui::Spinner::new().size(height * spinner_size));
                ui.add_space(height * space_size);
                ui.label(egui::RichText::new("Loading").size(height * text_size));
            }
        });

        ctx.data_mut(|d| {
            d.insert_temp(ui.id(), (spinner_size, space_size, text_size));
        })
    });
}
