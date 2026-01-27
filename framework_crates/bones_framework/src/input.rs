//! Input resources.

use bones_lib::ecs::World;
use bones_schema::HasSchema;

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
    pub use crate::input::{
        DenseInput, DenseInputCollector, DenseInputConfig, DensePlayerControl, InputCollector,
        PlayerControls,
    };
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
pub trait InputCollector<'a, Control>: Send + Sync {
    /// Update the internal state with new inputs. This must be called every render frame with the
    /// input events. This updates which buttons are pressed, but does not compute what buttons were "just_pressed".
    /// use [`InputCollector::update_just_pressed`] to do this.
    fn apply_inputs(&mut self, world: &World);

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
    fn get_control(&self) -> &Control;
}

/// Trait that tracks player control state. Provides associated types for other input trait implementations.
pub trait PlayerControls<'a, Control> {
    /// InputCollector used to update controls.
    type InputCollector: InputCollector<'a, Control>;

    /// Get control for player.
    fn get_control(&self, player_idx: usize) -> &Control;

    /// Get mutable control for player.
    fn get_control_mut(&mut self, player_idx: usize) -> &mut Control;
}

use std::fmt::Debug;

/// Dense input for network replication.
pub trait DenseInput:
    bytemuck::Pod + bytemuck::Zeroable + Copy + Clone + PartialEq + Eq + Send + Sync
{
}

/// Automatic implementation for `DenseInput`.
impl<T> DenseInput for T where
    T: bytemuck::Pod + bytemuck::Zeroable + Copy + Clone + PartialEq + Eq + Send + Sync
{
}

/// Define input types used by game for use in networking.
///
/// As long as types `PlayerControls` and `InputCollector` implement traits [`PlayerControls`] and [`InputCollector`],
/// trait bounds [`DensePlayerControl`] and [`DenseInputCollector`] are automatically implemented.
#[allow(missing_docs)]
pub trait DenseInputConfig<'a> {
    type Dense: DenseInput + Debug + Default;
    type Control: DensePlayerControl<Self::Dense>;

    // Must be HasSchema because expected to be retrieved from `World` as `Resource`.
    type PlayerControls: PlayerControls<'a, Self::Control> + HasSchema;

    // InputCollector type params must match that of PlayerControls, so using associated types.
    type InputCollector: InputCollector<'a, Self::Control> + Default;
}

///  Trait allowing for creating and applying [`DenseInput`] from control.
pub trait DensePlayerControl<Dense: DenseInput>: Send + Sync + Default {
    /// Get [`DenseInput`] for control.
    fn get_dense_input(&self) -> Dense;

    /// Update control from [`DenseInput`].
    fn update_from_dense(&mut self, new_control: &Dense);
}

/// Extension of [`InputCollector`] exposing dense control for networking.
///
/// This trait is automatically implemented for [`InputCollector`]'s such that `Control`
/// implements [`DensePlayerControl`] (i.e. implements dense input)
pub trait DenseInputCollector<'a, Dense, ControlMapping, ControlSource, Control>:
    InputCollector<'a, Control>
where
    Dense: DenseInput,
    ControlMapping: HasSchema,
    Control: DensePlayerControl<Dense>,
{
    /// Get dense control
    fn get_dense_control(&self) -> Dense;
}

/// Provide automatic [`DenseInputCollector`] for [`InputCollector`] when type parameters
/// meet required bounds for networking.
impl<'a, T, Dense, ControlMapping, ControlSource, Control>
    DenseInputCollector<'a, Dense, ControlMapping, ControlSource, Control> for T
where
    Dense: DenseInput,
    Control: DensePlayerControl<Dense>,
    ControlMapping: HasSchema,
    T: InputCollector<'a, Control>,
{
    fn get_dense_control(&self) -> Dense {
        self.get_control().get_dense_input()
    }
}
