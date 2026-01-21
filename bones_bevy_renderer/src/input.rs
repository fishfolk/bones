use super::*;
use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    window::PrimaryWindow,
};
use bones::{MouseScreenPosition, MouseWorldPosition};
use bones_framework::input::gilrs::process_gamepad_events;

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
                    scan_code: bones::Set(event.scan_code),
                    key_code: event.key_code.map(|x| x.into_bones()).into(),
                    button_state: event.state.into_bones(),
                })
                .collect(),
        },
        process_gamepad_events(),
    )
}

pub fn insert_mouse_position(
    In((screen_pos, world_pos)): In<(Option<Vec2>, Option<Vec2>)>,
    mut game: ResMut<BonesGame>,
) {
    game.insert_shared_resource(MouseScreenPosition(screen_pos));
    game.insert_shared_resource(MouseWorldPosition(world_pos));
}

// Source: https://bevy-cheatbook.github.io/cookbook/cursor2world.html
pub fn get_mouse_position(
    mut q_primary_windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
) -> (Option<Vec2>, Option<Vec2>) {
    match q_primary_windows
        .get_single_mut()
        .ok()
        .and_then(Window::cursor_position)
    {
        None => (None, None),
        screen_pos @ Some(sp) => match q_camera.get_single() {
            Err(_) => (screen_pos, None),
            Ok((camera, camera_transform)) => {
                let world_pos = camera.viewport_to_world_2d(camera_transform, sp);
                (screen_pos, world_pos)
            }
        },
    }
}
