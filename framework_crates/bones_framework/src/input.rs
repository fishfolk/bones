//! Input resources.

pub mod gamepad;
pub mod keyboard;
pub mod mouse;
pub mod time;
pub mod window;

/// The state of a button, ether pressed or released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ButtonState {
    /// The button is pressed.
    Pressed,
    /// The button is released.
    Released,
}

/// Module prelude.
pub mod prelude {
    pub use super::{keyboard::*, mouse::*, time::*, window::*, ButtonState};
}
