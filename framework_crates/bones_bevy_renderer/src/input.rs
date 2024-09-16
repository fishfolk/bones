use super::*;
use bevy::{
    input::{
        gamepad::GamepadEvent,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    window::PrimaryWindow,
};
use bones::MousePosition;

pub fn insert_bones_input(
    In((mouse_inputs, keyboard_inputs, gamepad_inputs)): In<(
        bones::MouseInputs,
        bones::KeyboardInputs,
        bones::GamepadInputs,
    )>,
    mut game: ResMut<BonesGame>,
) {
    // Add the game inputs
    game.insert_shared_resource(mouse_inputs);
    game.insert_shared_resource(keyboard_inputs);
    game.insert_shared_resource(gamepad_inputs);
}

pub fn get_bones_input(
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut keyboard_events: EventReader<KeyboardInput>,
    mut gamepad_events: EventReader<GamepadEvent>,
) -> (
    bones::MouseInputs,
    bones::KeyboardInputs,
    bones::GamepadInputs,
) {
    // TODO: investigate possible ways to avoid allocating vectors every frame for event lists.
    (
        bones::MouseInputs {
            movement: mouse_motion_events
                .iter()
                .last()
                .map(|x| x.delta)
                .unwrap_or_default(),
            wheel_events: mouse_wheel_events
                .iter()
                .map(|event| bones::MouseScrollEvent {
                    unit: event.unit.into_bones(),
                    movement: Vec2::new(event.x, event.y),
                })
                .collect(),
            button_events: mouse_button_input_events
                .iter()
                .map(|event| bones::MouseButtonEvent {
                    button: event.button.into_bones(),
                    state: event.state.into_bones(),
                })
                .collect(),
        },
        bones::KeyboardInputs {
            key_events: keyboard_events
                .iter()
                .map(|event| bones::KeyboardEvent {
                    scan_code: event.scan_code,
                    key_code: event.key_code.map(|x| x.into_bones()).into(),
                    button_state: event.state.into_bones(),
                })
                .collect(),
        },
        bones::GamepadInputs {
            gamepad_events: gamepad_events
                .iter()
                .map(|event| match event {
                    GamepadEvent::Connection(c) => {
                        bones::GamepadEvent::Connection(bones::GamepadConnectionEvent {
                            gamepad: c.gamepad.id as u32,
                            event: if c.connected() {
                                bones::GamepadConnectionEventKind::Connected
                            } else {
                                bones::GamepadConnectionEventKind::Disconnected
                            },
                        })
                    }
                    GamepadEvent::Button(b) => {
                        bones::GamepadEvent::Button(bones::GamepadButtonEvent {
                            gamepad: b.gamepad.id as u32,
                            button: b.button_type.into_bones(),
                            value: b.value,
                        })
                    }
                    GamepadEvent::Axis(a) => bones::GamepadEvent::Axis(bones::GamepadAxisEvent {
                        gamepad: a.gamepad.id as u32,
                        axis: a.axis_type.into_bones(),
                        value: a.value,
                    }),
                })
                .collect(),
        },
    )
}

pub fn insert_mouse_position(In(mouse_position): In<Option<Vec2>>, mut game: ResMut<BonesGame>) {
    game.insert_shared_resource(MousePosition(mouse_position));
}

// Source: https://bevy-cheatbook.github.io/cookbook/cursor2world.html
pub fn get_mouse_position(
    mut q_primary_windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = q_primary_windows.get_single_mut().ok()?;
    let viewport_position = window.cursor_position()?;

    let (camera, camera_transform) = q_camera.get_single().ok()?;
    camera.viewport_to_world_2d(camera_transform, viewport_position)
}
