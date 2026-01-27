//! Input traits required by networking. These traits are networking specific, either only used in networking,
//! or extending other traits from [`crate::input`] for networking.

use std::fmt::Debug;

use bones_schema::HasSchema;

use crate::input::{InputCollector, PlayerControls};

use super::NetworkInputStatus;

/// Define input types used by game for use in networking.
///
/// As long as types `PlayerControls` and `InputCollector` implement traits [`PlayerControls`] and [`InputCollector`],
/// trait bounds [`NetworkPlayerControl`] and [`NetworkInputCollector`] are automatically implemented.
#[allow(missing_docs)]
pub trait NetworkInputConfig<'a> {
    type Dense: DenseInput + Debug + Default;
    type Control: NetworkPlayerControl<Self::Dense>;

    // Must be HasSchema because expected to be retrieved from `World` as `Resource`.
    type PlayerControls: PlayerControls<'a, Self::Control> + HasSchema;

    // InputCollector type params must match that of PlayerControls, so using associated types.
    type InputCollector: InputCollector<'a, Self::Control> + Default;
}

/// Required for use of [`PlayerControls`] in networking.
pub trait NetworkPlayerControls<'a, Dense: DenseInput, Control>:
    PlayerControls<'a, Control>
{
    /// Update control of player from dense input.
    ///
    /// [`NetworkInputStatus`] communicates if input is confirmed, predicted, or from disconnected player.
    fn network_update(
        &mut self,
        player_idx: usize,
        dense_input: &Dense,
        status: NetworkInputStatus,
    );

    /// Get dense control for player.
    fn get_dense_control(&self, player_idx: usize) -> Dense;
}

impl<'a, T, Dense, Control> NetworkPlayerControls<'a, Dense, Control> for T
where
    Dense: DenseInput,
    Control: NetworkPlayerControl<Dense>,
    T: PlayerControls<'a, Control>,
{
    // type NetworkControl = PlayerControl;
    fn network_update(
        &mut self,
        player_idx: usize,
        dense_input: &Dense,
        _status: NetworkInputStatus,
    ) {
        self.get_control_mut(player_idx)
            .update_from_dense(dense_input);
    }

    fn get_dense_control(&self, player_idx: usize) -> Dense {
        self.get_control(player_idx).get_dense_input()
    }
}

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

///  Trait allowing for creating and applying [`DenseInput`] from control.
pub trait NetworkPlayerControl<Dense: DenseInput>: Send + Sync + Default {
    /// Get [`DenseInput`] for control.
    fn get_dense_input(&self) -> Dense;

    /// Update control from [`DenseInput`].
    fn update_from_dense(&mut self, new_control: &Dense);
}

/// Extension of [`InputCollector`] exposing dense control for networking.
///
/// This trait is automatically implemented for [`InputCollector`]'s such that `Control`
/// implements [`NetworkPlayerControl`] (i.e. implements dense input)
pub trait NetworkInputCollector<'a, Dense, ControlMapping, ControlSource, Control>:
    InputCollector<'a, Control>
where
    Dense: DenseInput,
    ControlMapping: HasSchema,
    Control: NetworkPlayerControl<Dense>,
{
    /// Get dense control
    fn get_dense_control(&self) -> Dense;
}

/// Provide automatic [`NetworkInputCollector`] for [`InputCollector`] when type parameters
/// meet required bounds for networking.
impl<'a, T, Dense, ControlMapping, ControlSource, Control>
    NetworkInputCollector<'a, Dense, ControlMapping, ControlSource, Control> for T
where
    Dense: DenseInput,
    Control: NetworkPlayerControl<Dense>,
    ControlMapping: HasSchema,
    T: InputCollector<'a, Control>,
{
    fn get_dense_control(&self) -> Dense {
        self.get_control().get_dense_input()
    }
}
