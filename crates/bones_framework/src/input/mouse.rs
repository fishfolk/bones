//! Mouse input resource.

use crate::prelude::*;

/// The mouse inputs made this frame.
#[derive(HasSchema, Clone, Debug, Default)]
#[schema(opaque)]
pub struct MouseInputs {
    /// The movement of the mouse this frame.
    pub movement: Vec2,
    /// The mouse wheel event sent this frame.
    pub wheel_events: Vec<MouseScrollInput>,
    /// The mouse button events sent this frame.
    pub button_events: Vec<MouseButtonInput>,
}

/// Mouse scroll-wheel input event.
#[derive(Debug, Clone, Copy)]
pub struct MouseScrollInput {
    /// The unit the mouse scroll is in.
    pub unit: MouseScrollUnit,
    /// the scroll movement.
    pub movement: Vec2,
}

/// The unit that a [`MouseWheelInput`] is in.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MouseScrollUnit {
    /// The number of lines scrolled.
    Lines,
    /// The number of pixels scrolled.
    Pixels,
}

/// A mouse button input event.
#[derive(Debug, Clone, Copy)]
pub struct MouseButtonInput {
    /// The button that the event refers to.
    pub button: MouseButton,
    /// Whether the button was pressed or released.
    pub state: ButtonState,
}

/// A button on the mouse.
#[derive(Debug, Default, Clone, Copy)]
#[repr(u8)]
pub enum MouseButton {
    #[default]
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
    /// The middle mouse button.
    Middle,
    /// Another mouse button with the associated number.
    Other(u16),
}
