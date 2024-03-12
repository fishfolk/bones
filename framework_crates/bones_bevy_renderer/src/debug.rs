//! Debug module for data that hooks into Bevy. Bevy frame diagnostics are synchronized into [`bones_framework::debug::FrameDiagState`]
//! which may be displayed using [`bones_framework::debug`] module.
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bones_framework::debug::FrameDiagState;

use crate::BonesData;

/// Plugin for debug tools that hook into Bevy, such as [`bevy::diagnostic::FrameTimeDiagnosticsPlugin`].
pub struct BevyDebugPlugin;

impl Plugin for BevyDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin);
        app.add_systems(Last, sync_frame_time);
    }
}

fn sync_frame_time(mut bones_data: ResMut<BonesData>, diagnostics: Res<DiagnosticsStore>) {
    let game = &mut bones_data.game;
    let mut state = game.init_shared_resource::<FrameDiagState>();

    let fps = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).unwrap();
    state.fps = fps.value().unwrap_or(0.0);
    state.fps_avg = fps.average().unwrap_or(0.0);

    if state.fps < state.min_fps {
        state.min_fps = state.fps;
    }
    if state.fps > state.max_fps {
        state.max_fps = state.fps;
    }

    let frame_time = diagnostics
        .get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .unwrap();
    state.frame_time = frame_time.value().unwrap_or(0.0);
    state.frame_time_avg = frame_time.average().unwrap_or(0.0);

    if state.frame_time < state.min_frame_time {
        state.min_frame_time = state.frame_time;
    }
    if state.frame_time > state.max_frame_time {
        state.max_frame_time = state.frame_time;
    }
}
