use crate::bones::SVec;
use crate::BonesGame;
use bevy::input::gamepad::{
    GamepadRumbleIntensity as BevyGamepadRumbleIntensity,
    GamepadRumbleRequest as BevyGamepadRumbleRequest,
};
use bevy::prelude::*;
use bevy::utils::Duration;
use bones_framework::prelude as bones;

pub fn handle_bones_rumble(
    game: ResMut<BonesGame>,
    mut rumble_requests: EventWriter<BevyGamepadRumbleRequest>,
) {
    if let Some(mut bones_rumble_requests) = game.shared_resource_mut::<bones::GamepadsRumble>() {
        for request in &bones_rumble_requests.requests {
            match request {
                bones::GamepadRumbleRequest::Trigger {
                    gamepad,
                    intensity,
                    duration,
                } => {
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
                bones::GamepadRumbleRequest::Stop { gamepad } => {
                    let gamepad = Gamepad::new(*gamepad as usize);
                    rumble_requests.send(BevyGamepadRumbleRequest::Stop { gamepad });
                }
            }
        }
        bones_rumble_requests.requests = SVec::new();
    }
}
