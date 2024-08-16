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

impl std::fmt::Display for GamepadButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GamepadButton::South => "South",
                GamepadButton::East => "East",
                GamepadButton::North => "North",
                GamepadButton::West => "West",
                GamepadButton::C => "C",
                GamepadButton::Z => "Z",
                GamepadButton::LeftTrigger => "Left Trigger",
                GamepadButton::LeftTrigger2 => "Left Trigger 2",
                GamepadButton::RightTrigger => "Right Trigger",
                GamepadButton::RightTrigger2 => "Right Trigger 2",
                GamepadButton::Select => "Select",
                GamepadButton::Start => "Start",
                GamepadButton::Mode => "Mode",
                GamepadButton::LeftThumb => "Left Thumb",
                GamepadButton::RightThumb => "Right Thumb",
                GamepadButton::DPadUp => "DPad Up",
                GamepadButton::DPadDown => "DPad Down",
                GamepadButton::DPadLeft => "DPad Left",
                GamepadButton::DPadRight => "DPad Right",
                GamepadButton::Other(n) => return write!(f, "Button {n}"),
            }
        )
    }
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

impl std::fmt::Display for GamepadAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GamepadAxis::LeftStickX => "Left Stick X",
                GamepadAxis::LeftStickY => "Left Stick Y",
                GamepadAxis::LeftZ => "Left Z",
                GamepadAxis::RightStickX => "Right Stick X",
                GamepadAxis::RightStickY => "Right Stick Y",
                GamepadAxis::RightZ => "Right Z",
                GamepadAxis::Other(n) => return write!(f, "Axis {n}"),
            }
        )
    }
}

/// Struct that represents intensity of a rumble
#[derive(HasSchema, Default, Clone, Debug, Copy)]
pub struct GamepadRumbleIntensity {
    /// The intensity of the strong motor, between 0.0 - 1.0.
    strong_motor: f32,
    /// The intensity of the weak motor, between 0.0 - 1.0.
    weak_motor: f32,
}

impl GamepadRumbleIntensity {
    pub const ZERO: Self = Self {
        strong_motor: 0.0,
        weak_motor: 0.0,
    };
    pub const MAX_BOTH: Self = Self {
        strong_motor: 1.0,
        weak_motor: 1.0,
    };
    pub const MAX_STRONG: Self = Self {
        strong_motor: 1.0,
        weak_motor: 0.0,
    };
    pub const MAX_WEAK: Self = Self {
        strong_motor: 0.0,
        weak_motor: 1.0,
    };
    pub const MEDIUM_BOTH: Self = Self {
        strong_motor: 0.5,
        weak_motor: 0.5,
    };
    pub const MEDIUM_STRONG: Self = Self {
        strong_motor: 0.5,
        weak_motor: 0.0,
    };
    pub const MEDIUM_WEAK: Self = Self {
        strong_motor: 0.0,
        weak_motor: 0.5,
    };
    pub const LIGHT_BOTH: Self = Self {
        strong_motor: 0.25,
        weak_motor: 0.25,
    };
    pub const LIGHT_STRONG: Self = Self {
        strong_motor: 0.25,
        weak_motor: 0.0,
    };
    pub const LIGHT_WEAK: Self = Self {
        strong_motor: 0.0,
        weak_motor: 0.25,
    };
    pub const VERY_LIGHT_BOTH: Self = Self {
        strong_motor: 0.1,
        weak_motor: 0.1,
    };
    pub const VERY_LIGHT_STRONG: Self = Self {
        strong_motor: 0.1,
        weak_motor: 0.0,
    };
    pub const VERY_LIGHT_WEAK: Self = Self {
        strong_motor: 0.0,
        weak_motor: 0.1,
    };

    /// Get the intensity of the strong motor.
    pub fn strong_motor(&self) -> f32 {
        self.strong_motor
    }

    /// Set the intensity of the strong motor, clamping it between 0.0 and 1.0.
    pub fn set_strong_motor(&mut self, value: f32) {
        self.strong_motor = value.clamp(0.0, 1.0);
    }

    /// Get the intensity of the weak motor.
    pub fn weak_motor(&self) -> f32 {
        self.weak_motor
    }

    /// Set the intensity of the weak motor, clamping it between 0.0 and 1.0.
    pub fn set_weak_motor(&mut self, value: f32) {
        self.weak_motor = value.clamp(0.0, 1.0);
    }
}

/// Represents a request to either add or stop rumble on a specific gamepad
#[derive(HasSchema, Clone, Debug)]
pub enum GamepadRumbleRequest {
    /// Request to add rumble to a gamepad.
    Add {
        /// The ID of the gamepad to rumble.
        gamepad: u32,
        /// The intensity of the rumble.
        intensity: GamepadRumbleIntensity,
        /// The duration of the rumble in seconds.
        duration: f32,
    },
    /// Request to stop rumble on a gamepad.
    Stop {
        /// The ID of the gamepad to stop rumbling.
        gamepad: u32,
    },
}

impl Default for GamepadRumbleRequest {
    fn default() -> Self {
        GamepadRumbleRequest::Stop { gamepad: 0 }
    }
}
