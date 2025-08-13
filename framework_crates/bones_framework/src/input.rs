//! Input resources.

use bones_schema::HasSchema;

use self::prelude::{GamepadInputs, KeyboardInputs};

pub mod gamepad;
pub mod gilrs;
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
    pub use crate::input::{InputCollector, PlayerControls};
}

/// Maps raw inputs to game controls and exposes controls for respective player and their control source.
///
/// [`InputCollector::apply_inputs`] maps raw input to game controls and updates them.
///
/// [`InputCollector::update_just_pressed`] computes any changes in pressed buttons that may be stored on control.
///
/// [`InputCollector::advance_frame`] is  used to mark that the input has been consumed, and update the prev frame inputs to current, to compute changes next frame.
///
/// Generic type param ControlMapping is HasSchema because it is expected to be a Resource retrievable on world.
pub trait InputCollector<'a, ControlMapping: HasSchema, ControlSource, Control>:
    Send + Sync
{
    /// Update the internal state with new inputs. This must be called every render frame with the
    /// input events. This updates which buttons are pressed, but does not compute what buttons were "just_pressed".
    /// use [`InputCollector::update_just_pressed`] to do this.
    fn apply_inputs(
        &mut self,
        mapping: &ControlMapping,
        keyboard: &KeyboardInputs,
        gamepad: &GamepadInputs,
    );

    /// Indicate input for this frame has been consumed. An implementation of [`InputCollector`] that is
    /// used with a fixed simulation step may track what keys are currently pressed, and what keys were "just pressed",
    /// (changing from previous state).
    ///
    /// This saves current inputs as previous frame's inputs, allowing for determining what is "just pressed" next frame.
    fn advance_frame(&mut self);

    /// Update which buttons have been "just pressed", when input has changed from last frame and current input state.
    ///
    /// This does not modify previous frame's input, to do this use [`InputCollector::advance_frame`].
    fn update_just_pressed(&mut self);

    /// Get control for player based on provided `ControlSource`.
    fn get_control(&self, control_source: ControlSource) -> &Control;
}

/// Trait that tracks player control state. Provides associated types for other input trait implementations.
pub trait PlayerControls<'a, Control> {
    /// InputCollector used to update controls.
    type InputCollector: InputCollector<'a, Self::ControlMapping, Self::ControlSource, Control>;

    /// Control mapping from raw input, expected to be able to be retrieved as `Resource` from world.
    type ControlMapping: HasSchema;

    /// Type used to map source of input to control.
    type ControlSource;

    /// Get `ControlSource` for player (only present for local player).
    fn get_control_source(&self, local_player_idx: usize) -> Option<Self::ControlSource>;

    /// Get control for player.
    fn get_control(&self, player_idx: usize) -> &Control;

    /// Get mutable control for player.
    fn get_control_mut(&mut self, player_idx: usize) -> &mut Control;
}
