//! Gilrs integration.
use crate::prelude::*;
use gilrs::{ev::filter::axis_dpad_to_button, EventType, Filter, Gilrs as GilrsContext};
use once_cell::sync::Lazy;
use send_wrapper::SendWrapper;
use std::sync::{Arc, Mutex};

/// Lazy-initialized GilrsContext
static GILRS_CONTEXT: Lazy<Arc<Mutex<SendWrapper<GilrsContext>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(SendWrapper::new(
        GilrsContext::new().expect("Failed to initialize GilrsContext"),
    )))
});

/// Processes gilrs gamepad events into Bones-native GamepadInputs
pub fn process_gamepad_events() -> GamepadInputs {
    let mut gamepad_inputs = GamepadInputs::default();
    let mut gilrs = GILRS_CONTEXT.lock().unwrap();
    while let Some(gilrs_event) = gilrs
        .next_event()
        .filter_ev(&axis_dpad_to_button, &mut *gilrs)
    {
        gilrs.update(&gilrs_event);

        let gamepad = usize::from(gilrs_event.id) as u32;
        match gilrs_event.event {
            EventType::Connected => {
                let _pad = gilrs.gamepad(gilrs_event.id);
                gamepad_inputs.gamepad_events.push(GamepadEvent::Connection(
                    GamepadConnectionEvent {
                        gamepad,
                        event: GamepadConnectionEventKind::Connected,
                    },
                ));
            }
            EventType::Disconnected => {
                gamepad_inputs.gamepad_events.push(GamepadEvent::Connection(
                    GamepadConnectionEvent {
                        gamepad,
                        event: GamepadConnectionEventKind::Disconnected,
                    },
                ));
            }
            EventType::ButtonChanged(gilrs_button, value, _) => {
                if let Some(button) = convert_button(gilrs_button) {
                    gamepad_inputs
                        .gamepad_events
                        .push(GamepadEvent::Button(GamepadButtonEvent {
                            gamepad,
                            button,
                            value,
                        }));
                }
            }
            EventType::AxisChanged(gilrs_axis, value, _) => {
                if let Some(axis) = convert_axis(gilrs_axis) {
                    gamepad_inputs
                        .gamepad_events
                        .push(GamepadEvent::Axis(GamepadAxisEvent {
                            gamepad,
                            axis,
                            value,
                        }));
                }
            }
            _ => (),
        };
    }
    gamepad_inputs
}

/// Converts a gilrs button to a bones-native button
fn convert_button(button: gilrs::Button) -> Option<GamepadButton> {
    match button {
        gilrs::Button::South => Some(GamepadButton::South),
        gilrs::Button::East => Some(GamepadButton::East),
        gilrs::Button::North => Some(GamepadButton::North),
        gilrs::Button::West => Some(GamepadButton::West),
        gilrs::Button::C => Some(GamepadButton::C),
        gilrs::Button::Z => Some(GamepadButton::Z),
        gilrs::Button::LeftTrigger => Some(GamepadButton::LeftTrigger),
        gilrs::Button::LeftTrigger2 => Some(GamepadButton::LeftTrigger2),
        gilrs::Button::RightTrigger => Some(GamepadButton::RightTrigger),
        gilrs::Button::RightTrigger2 => Some(GamepadButton::RightTrigger2),
        gilrs::Button::Select => Some(GamepadButton::Select),
        gilrs::Button::Start => Some(GamepadButton::Start),
        gilrs::Button::Mode => Some(GamepadButton::Mode),
        gilrs::Button::LeftThumb => Some(GamepadButton::LeftThumb),
        gilrs::Button::RightThumb => Some(GamepadButton::RightThumb),
        gilrs::Button::DPadUp => Some(GamepadButton::DPadUp),
        gilrs::Button::DPadDown => Some(GamepadButton::DPadDown),
        gilrs::Button::DPadLeft => Some(GamepadButton::DPadLeft),
        gilrs::Button::DPadRight => Some(GamepadButton::DPadRight),
        gilrs::Button::Unknown => None,
    }
}

/// Converts a gilrs axis to a bones-native axis
fn convert_axis(axis: gilrs::Axis) -> Option<GamepadAxis> {
    match axis {
        gilrs::Axis::LeftStickX => Some(GamepadAxis::LeftStickX),
        gilrs::Axis::LeftStickY => Some(GamepadAxis::LeftStickY),
        gilrs::Axis::LeftZ => Some(GamepadAxis::LeftZ),
        gilrs::Axis::RightStickX => Some(GamepadAxis::RightStickX),
        gilrs::Axis::RightStickY => Some(GamepadAxis::RightStickY),
        gilrs::Axis::RightZ => Some(GamepadAxis::RightZ),
        gilrs::Axis::Unknown | gilrs::Axis::DPadX | gilrs::Axis::DPadY => None,
    }
}
