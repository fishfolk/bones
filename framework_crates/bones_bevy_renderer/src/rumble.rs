// framework_crates/bones_bevy_renderer/src/rumble.rs
// use crate::bones::HasSchema;
use bevy::input::gamepad::{
    GamepadRumbleIntensity as BevyGamepadRumbleIntensity,
    GamepadRumbleRequest as BevyGamepadRumbleRequest,
};
use bevy::prelude::*;
use bevy::utils::Duration;
use bones::GamepadRumbleRequest;
use bones_framework::prelude as bones;

/// Struct that wraps a list of gamepad rumble requests as a resource
#[derive(Resource, Default, Clone)]
pub struct GamepadRumbleRequests(pub Vec<GamepadRumbleRequest>);

pub fn handle_bones_rumble(
    mut bones_rumble_requests: ResMut<GamepadRumbleRequests>,
    mut rumble_requests: EventWriter<BevyGamepadRumbleRequest>,
) {
    for request in bones_rumble_requests.0.drain(..) {
        match request {
            bones::GamepadRumbleRequest::Add {
                gamepad,
                intensity,
                duration,
            } => {
                let bevy_intensity = BevyGamepadRumbleIntensity {
                    strong_motor: intensity.strong_motor(),
                    weak_motor: intensity.weak_motor(),
                };

                let gamepad = Gamepad::new(gamepad as usize);

                rumble_requests.send(BevyGamepadRumbleRequest::Add {
                    gamepad,
                    intensity: bevy_intensity,
                    duration: Duration::from_secs_f32(duration),
                });
            }
            bones::GamepadRumbleRequest::Stop { gamepad } => {
                let gamepad = Gamepad::new(gamepad as usize);
                rumble_requests.send(BevyGamepadRumbleRequest::Stop { gamepad });
            }
        }
    }
}
