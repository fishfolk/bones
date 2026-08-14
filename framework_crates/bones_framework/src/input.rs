//! Input resources.

use bones_lib::ecs::World;
use bones_schema::HasSchema;
use std::fmt::Debug;

pub mod gamepad;
pub mod gilrs;
pub mod keyboard;
pub mod mouse;
pub mod proto;
pub mod window;

/// The state of a button, either pressed or released.
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
    pub use super::{gamepad::*, keyboard::*, mouse::*, proto, window::*, ButtonState};
    pub use crate::input::{Controls, DenseControl, DenseInput, DenseInputConfig, InputCollector};
}

/// Dense input super trait for network replication.
pub trait DenseInput:
    bytemuck::Pod + bytemuck::Zeroable + Copy + Clone + PartialEq + Eq + Send + Sync
{
}
// Automatic implementation for `DenseInput`.
impl<T> DenseInput for T where
    T: bytemuck::Pod + bytemuck::Zeroable + Copy + Clone + PartialEq + Eq + Send + Sync
{
}

/// Collects local input state for the network client from the [`World`].
pub trait InputCollector<'a, State>: Send + Sync {
    /// Update the internal state with the [`World`].
    ///
    /// This should be called every render frame / cpu cycle after input events.
    fn apply_inputs(&mut self, world: &World);

    /// Indicates to the collector that its input has been consumed.
    ///
    /// This should be called on frame update when input for this frame has been
    /// consumed as opposed to the cpu cycle. In a network session, by the time
    /// this is called, the dense input should be sent across the network.
    fn advance_frame(&mut self);

    /// Get the internally stored input state.
    fn get_state(&self) -> State;
}

/// A trait representing a control type capable of being updated by a [`DenseInput`].
pub trait DenseControl<Dense: DenseInput>: Send + Sync + Default {
    /// Update this control with a [`DenseInput`].
    fn update_from_dense(&mut self, new_control: &Dense);
}

/// Trait for getting every player control via index.
pub trait Controls<'a, Control> {
    /// Get control for player.
    fn get_control(&self, player_idx: usize) -> &Control;

    /// Get mutable control for player.
    fn get_control_mut(&mut self, player_idx: usize) -> &mut Control;
}

/// Defines input types for use in networking.
#[allow(missing_docs)]
pub trait DenseInputConfig<'a> {
    /// Current input state for a client. This is what is sent across the
    /// network. All input history and other data derived from the state should
    /// be stored by the [`Control`](DenseInputConfig::Control).
    type Dense: DenseInput + Debug + Default;

    /// Local configuration for getting [`Dense`](DenseInputConfig::Dense) from
    /// the [`World`] to be sent across the network.
    type InputCollector: InputCollector<'a, Self::Dense> + Default;

    /// The complete input data of a client. Updated with
    /// [`Dense`](DenseInputConfig::Dense) when recieved from the network. The
    /// [`Controls`](DenseInputConfig::Controls) resource in the [`World`] will
    /// contain every client's control after update.
    type Control: DenseControl<Self::Dense>;

    /// The resource that will contain a [`Control`](DenseInputConfig::Control)
    /// for every client in a network session.
    type Controls: Controls<'a, Self::Control>
        // HasSchema because expected to be retrieved from `World` as
        // `Resource`.
        + HasSchema
        // Default because not expected to be initialized when retrieved from
        // `World`.
        + Default;
}
