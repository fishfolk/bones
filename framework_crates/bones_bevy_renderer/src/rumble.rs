use crate::bones::SVec;
use crate::BonesGame;
use bevy::input::gamepad::{
    GamepadRumbleIntensity as BevyGamepadRumbleIntensity,
    GamepadRumbleRequest as BevyGamepadRumbleRequest,
};
use bevy::prelude::*;
use bevy::utils::Duration;
use bones_framework::prelude as bones;

/// Handles rumble requests from the Bones framework and translates them to Bevy rumble requests
pub fn handle_bones_rumble(
    game: ResMut<BonesGame>,
    mut rumble_requests: EventWriter<BevyGamepadRumbleRequest>,
) {
    if let Some(mut bones_rumble_requests) = game.shared_resource_mut::<bones::GamepadsRumble>() {
        for request in &bones_rumble_requests.requests {
            match request {
                bones::GamepadRumbleRequest::AddRumble {
                    gamepad,
                    intensity,
                    duration,
                } => {
                    send_rumble_request(gamepad, intensity, duration, &mut rumble_requests);
                }
                bones::GamepadRumbleRequest::SetRumble {
                    gamepad,
                    intensity,
                    duration,
                } => {
                    // First, stop the current rumble
                    let stop_gamepad = Gamepad::new(*gamepad as usize);
                    rumble_requests.send(BevyGamepadRumbleRequest::Stop {
                        gamepad: stop_gamepad,
                    });

                    // Then, add the new rumble
                    send_rumble_request(gamepad, intensity, duration, &mut rumble_requests);
                }
                bones::GamepadRumbleRequest::Stop { gamepad } => {
                    let gamepad = Gamepad::new(*gamepad as usize);
                    rumble_requests.send(BevyGamepadRumbleRequest::Stop { gamepad });
                }
            }
        }
        bones_rumble_requests.requests = SVec::new();
    }
}

/// Helper function to send a rumble request to Bevy
fn send_rumble_request(
    gamepad: &u32,
    intensity: &bones::GamepadRumbleIntensity,
    duration: &f32,
    rumble_requests: &mut EventWriter<BevyGamepadRumbleRequest>,
) {
    let bevy_intensity = BevyGamepadRumbleIntensity {
        strong_motor: intensity.strong_motor(),
        weak_motor: intensity.weak_motor(),
    };

    let gamepad = Gamepad::new(*gamepad as usize);

    rumble_requests.send(BevyGamepadRumbleRequest::Add {
        gamepad,
        intensity: bevy_intensity,
        duration: Duration::from_secs_f32(*duration),
    });
}
