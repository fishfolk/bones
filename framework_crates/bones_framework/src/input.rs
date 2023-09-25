//! Input resources.

use bones_schema::HasSchema;

pub mod gamepad;
pub mod keyboard;
pub mod mouse;
pub mod window;

/// The state of a button, ether pressed or released.
#[derive(HasSchema, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ButtonState {
    /// The button is pressed.
    Pressed,
    #[default]
    /// The button is released.
    Released,
}

impl ButtonState {
    /// Get whether or not the button is pressed.
    pub fn pressed(&self) -> bool {
        matches!(self, ButtonState::Pressed)
    }
}

/// Module prelude.
pub mod prelude {
    pub use super::{gamepad::*, keyboard::*, mouse::*, window::*, ButtonState};
}
