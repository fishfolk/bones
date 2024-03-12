//! Implements bones egui debug windows and tools.

use crate::prelude::*;

/// Track frame time state synced from bevy frame time diagnostics for bones app.
#[derive(HasSchema, Clone)]
#[allow(missing_docs)]
pub struct FrameDiagState {
    pub fps: f64,
    pub fps_avg: f64,
    pub min_fps: f64,
    pub max_fps: f64,
    pub frame_time: f64,
    pub frame_time_avg: f64,
    pub min_frame_time: f64,
    pub max_frame_time: f64,
}

impl Default for FrameDiagState {
    fn default() -> Self {
        Self {
            fps: 0.0,
            fps_avg: 0.0,
            min_fps: f64::MAX,
            max_fps: 0.0,
            frame_time: 0.0,
            frame_time_avg: 0.0,
            min_frame_time: f64::MAX,
            max_frame_time: 0.0,
        }
    }
}

impl FrameDiagState {
    /// Reset min/max values to default
    pub fn reset(&mut self) {
        self.min_fps = f64::MAX;
        self.max_fps = 0.0;
        self.min_frame_time = f64::MAX;
        self.max_frame_time = 0.0;
    }
}

/// State of frame time diagnostic window. Stored in [`EguiCtx`] state,
/// setting open = true will open window if plugin installed.
#[derive(Clone, Default)]
pub struct FrameTimeWindowState {
    /// Is window open?
    pub open: bool,
}

/// If installed, allows opening egui window with [`FrameTimeWindowState`] in [`EguiCtx`] state
/// to get frame time information.
pub fn frame_time_diagnostics_plugin(core: &mut Session) {
    core.stages
        .add_system_to_stage(CoreStage::Last, frame_diagnostic_window);
}

/// Renders frame time diagnostic window in Egui if window is set to open in [`FrameTimeWindowState`]
/// stored in [`EguiCtx`] state.
pub fn frame_diagnostic_window(
    mut state: ResMut<FrameDiagState>,
    // localization: Res<Localization>,
    egui_ctx: ResMut<EguiCtx>,
) {
    let mut window_state = egui_ctx.get_state::<FrameTimeWindowState>();
    let window_open = &mut window_state.open;

    if *window_open {
        // egui::Window::new(&localization.get("frame-diagnostics"))
        egui::Window::new("Frame Diagnostics")
            .id(egui::Id::new("frame_diagnostics"))
            .default_width(500.0)
            .open(window_open)
            .show(&egui_ctx, |ui| {
                // if ui.button(&localization.get("reset-min-max")).clicked() {
                if ui.button("Reset Min/Max").clicked() {
                    state.reset();
                }

                ui.monospace(&format!(
                    "{label:20}: {fps:4.0}{suffix:3} ( {min:4.0}{suffix:3}, {avg:4.0}{suffix:3}, {max:4.0}{suffix:3} )",
                    // label = localization.get("frames-per-second"),
                    label = "Frames Per Second",
                    fps = state.fps,
                    // suffix = fps.suffix,
                    suffix = "fps",
                    min = state.min_fps,
                    avg = state.fps_avg,
                    max = state.max_fps,
                ));
                ui.monospace(&format!(
                    "{label:20}: {fps:4.1}{suffix:3} ( {min:4.1}{suffix:3}, {avg:4.0}{suffix:3}, {max:4.1}{suffix:3} )",
                    // label = localization.get("frame-time"),
                    label = "Frame Time",
                    fps = state.frame_time,
                    suffix = "ms",
                    min = state.min_frame_time,
                    avg = state.frame_time_avg,
                    max = state.max_frame_time,
                ));
            });
    }

    egui_ctx.set_state(window_state);
}
