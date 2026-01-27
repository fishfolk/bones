//! Input traits required by networking. These traits are networking specific, either only used in networking,
//! or extending other traits from [`crate::input`] for networking.

use crate::input::{DenseInput, DensePlayerControl, PlayerControls};

use super::NetworkInputStatus;

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
    Control: DensePlayerControl<Dense>,
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
