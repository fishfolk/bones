//! Gamepad input resource.

use crate::prelude::*;

/// Resource containing the gamepad input events detected this frame.
#[derive(HasSchema, Clone, Default, Debug)]
pub struct GamepadInputs {
    /// The gampad events.
    pub gamepad_events: SVec<GamepadEvent>,
}

/// A gamepad event.
#[derive(HasSchema, Clone, Debug)]
#[repr(C, u8)]
pub enum GamepadEvent {
    /// A connection event.
    Connection(GamepadConnectionEvent),
    /// A button event.
    Button(GamepadButtonEvent),
    /// An axis event.
    Axis(GamepadAxisEvent),
}

impl Default for GamepadEvent {
    fn default() -> Self {
        Self::Connection(default())
    }
}

/// A gamepad connection event.
#[derive(HasSchema, Clone, Debug, Default)]
#[repr(C)]
pub struct GamepadConnectionEvent {
    /// The ID of the gamepad.
    pub gamepad: u32,
    /// The type of connection event.
    pub event: GamepadConnectionEventKind,
}

/// The kind of gamepad connection event.
#[derive(HasSchema, Clone, Debug, Default)]
#[repr(u8)]
pub enum GamepadConnectionEventKind {
    #[default]
    /// The gamepad was connected.
    Connected,
    /// The gamepad was disconnected.
    Disconnected,
}

/// A gamepad button event.
#[derive(HasSchema, Clone, Debug, Default)]
#[repr(C)]
pub struct GamepadButtonEvent {
    /// The ID of the gamepad.
    pub gamepad: u32,
    /// The gamepad button.
    pub button: GamepadButton,
    /// The value of the button, for example, this will be `1.0` when presssed and `0.0` when
    /// released if this is a normal button.
    pub value: f32,
}

/// A specific button on a gamepad.
#[allow(missing_docs)]
#[derive(HasSchema, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C, u8)]
pub enum GamepadButton {
    #[default]
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Other(u8),
}

/// A gamepad axis event.
#[derive(HasSchema, Clone, Debug)]
#[schema(no_default)]
#[repr(C)]
pub struct GamepadAxisEvent {
    /// The ID of the gamepad.
    pub gamepad: u32,
    /// The axis that has changed.
    pub axis: GamepadAxis,
    /// The value of the axis.
    pub value: f32,
}

/// A specific gamepad axis that may have changed.
#[derive(HasSchema, Clone, Debug, PartialEq, Eq, Hash)]
#[schema(no_default)]
#[allow(missing_docs)]
#[repr(C, u8)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    Other(u8),
}
