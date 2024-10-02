#![doc = include_str!("./networking.md")]

use self::{
    input::{DenseInput, NetworkInputConfig, NetworkPlayerControl, NetworkPlayerControls},
    socket::Socket,
};
use crate::prelude::*;
use bones_matchmaker_proto::{MATCH_ALPN, PLAY_ALPN};
use desync::{DesyncDebugHistoryBuffer, DetectDesyncs};
use fxhash::FxHasher;
use ggrs::{DesyncDetection, P2PSession};
use instant::Duration;
use once_cell::sync::Lazy;
use std::{fmt::Debug, hash::Hasher, marker::PhantomData, sync::Arc};
use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "net-debug")]
use {
    self::debug::{NetworkDebugMessage, PlayerSyncState, NETWORK_DEBUG_CHANNEL},
    ggrs::{NetworkStats, PlayerHandle},
};

use crate::input::PlayerControls as PlayerControlsTrait;

pub mod desync;
pub mod input;
pub mod lan;
pub mod online;
pub mod proto;
pub mod socket;

#[cfg(feature = "net-debug")]
pub mod debug;

/// Runtime, needed to execute network related calls.
pub static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("unable to crate tokio runtime"));

/// Indicates if input from networking is confirmed, predicted, or if player is disconnected.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NetworkInputStatus {
    /// The input of this player for this frame is an actual received input.
    Confirmed,
    /// The input of this player for this frame is predicted.
    Predicted,
    /// The player has disconnected at or prior to this frame, so this input is a dummy.
    Disconnected,
}

impl From<ggrs::InputStatus> for NetworkInputStatus {
    fn from(value: ggrs::InputStatus) -> Self {
        match value {
            ggrs::InputStatus::Confirmed => NetworkInputStatus::Confirmed,
            ggrs::InputStatus::Predicted => NetworkInputStatus::Predicted,
            ggrs::InputStatus::Disconnected => NetworkInputStatus::Disconnected,
        }
    }
}

/// Module prelude.
pub mod prelude {
    pub use super::{
        desync::DetectDesyncs, input, lan, online, proto, DisconnectedPlayers, SyncingInfo, RUNTIME,
    };

    #[cfg(feature = "net-debug")]
    pub use super::debug::prelude::*;
}

/// Muliplier for framerate that will be used when playing an online match.
///
/// Lowering the frame rate a little for online matches reduces bandwidth and may help overall
/// gameplay. This may not be necessary once we improve network performance.
///
/// Note that FPS is provided as an integer to ggrs, so network modified fps is rounded to nearest int,
/// which is then used to compute timestep so ggrs and networking match.
pub const NETWORK_FRAME_RATE_FACTOR: f32 = 0.9;

/// Number of frames client may predict beyond confirmed frame before freezing and waiting
/// for inputs from other players. Default value if not specified in [`GgrsSessionRunnerInfo`].
pub const NETWORK_MAX_PREDICTION_WINDOW_DEFAULT: usize = 7;

// todo test as zero?

/// Amount of frames GGRS will delay local input.
pub const NETWORK_LOCAL_INPUT_DELAY_DEFAULT: usize = 2;

/// Possible errors returned by network loop.
pub enum NetworkError {
    /// The session was disconnected.
    Disconnected,
}

/// The [`ggrs::Config`] implementation used by Jumpy.
#[derive(Debug)]
pub struct GgrsConfig<T: DenseInput + Debug> {
    phantom: PhantomData<T>,
}

impl<T: DenseInput + Debug> ggrs::Config for GgrsConfig<T> {
    type Input = T;
    type State = World;
    /// Addresses are the same as the player handle for our custom socket.
    type Address = usize;
}

/// The network endpoint used for all network communications.
static NETWORK_ENDPOINT: tokio::sync::OnceCell<iroh_net::Endpoint> =
    tokio::sync::OnceCell::const_new();

/// Get the network endpoint used for all communications.
pub async fn get_network_endpoint() -> &'static iroh_net::Endpoint {
    NETWORK_ENDPOINT
        .get_or_init(|| async move {
            let secret_key = iroh_net::key::SecretKey::generate();
            iroh_net::Endpoint::builder()
                .alpns(vec![MATCH_ALPN.to_vec(), PLAY_ALPN.to_vec()])
                .discovery(Box::new(
                    iroh_net::discovery::ConcurrentDiscovery::from_services(vec![
                        Box::new(iroh_net::discovery::dns::DnsDiscovery::n0_dns()),
                        Box::new(iroh_net::discovery::pkarr::PkarrPublisher::n0_dns(
                            secret_key.clone(),
                        )),
                    ]),
                ))
                .secret_key(secret_key)
                .bind(0)
                .await
                .unwrap()
        })
        .await
}

/// Resource containing the [`NetworkSocket`] implementation while there is a connection to a
/// network game.
///
/// This is inserted into the world after a match has been established by a network matchmaker.
#[derive(Clone, HasSchema, Deref, DerefMut)]
#[schema(no_default)]
pub struct NetworkMatchSocket(Arc<dyn NetworkSocket>);

/// Wraps [`ggrs::Message`] with included `match_id`, used to determine if message received
/// from current match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMessage {
    /// Socket match id
    pub match_id: u8,
    /// Wrapped message
    pub message: ggrs::Message,
}

/// Automatically implemented for [`NetworkSocket`] + [`ggrs::NonBlockingSocket<usize>`].
pub trait GgrsSocket: NetworkSocket + ggrs::NonBlockingSocket<usize> {}
impl<T> GgrsSocket for T where T: NetworkSocket + ggrs::NonBlockingSocket<usize> {}

/// Trait that must be implemented by socket connections establish by matchmakers.
///
/// The [`NetworkMatchSocket`] resource will contain an instance of this trait and will be used by
/// the game to send network messages after a match has been established.
pub trait NetworkSocket: Sync + Send {
    /// Get a GGRS socket from this network socket.
    fn ggrs_socket(&self) -> Socket;
    /// Send a reliable message to the given [`SocketTarget`].
    fn send_reliable(&self, target: SocketTarget, message: &[u8]);
    /// Receive reliable messages from other players. The `usize` is the index of the player that
    /// sent the message.
    fn recv_reliable(&self) -> Vec<(u32, Vec<u8>)>;
    /// Close the connection.
    fn close(&self);
    /// Get the player index of the local player.
    fn player_idx(&self) -> u32;
    /// Get the player count for this network match.
    fn player_count(&self) -> u32;

    /// Increment match id so messages from previous match that are still in flight
    /// will be filtered out. Used when starting new session with existing socket.
    fn increment_match_id(&mut self);
}

/// The destination for a reliable network message.
pub enum SocketTarget {
    /// Send to a specific player.
    Player(u32),
    /// Broadcast to all players.
    All,
}

/// Resource updated each frame exposing syncing/networking information in the current session.
#[derive(HasSchema, Clone)]
#[schema(no_default)]
pub enum SyncingInfo {
    /// Holds data for an online session
    Online {
        /// Current frame of simulation step
        current_frame: i32,
        /// Last confirmed frame by all clients.
        /// Anything that occurred on this frame is agreed upon by all clients.
        last_confirmed_frame: i32,
        /// Socket
        socket: Socket,
        /// Networking stats for each connected player, stored at the \[player_idx\] index for each respective player.
        players_network_stats: SVec<PlayerNetworkStats>,
        /// The local player's index
        local_player_idx: usize,
        /// The local input delay set for this session
        local_frame_delay: usize,
        /// List of disconnected players (their idx)
        disconnected_players: SVec<usize>,
    },
    /// Holds data for an offline session
    Offline {
        /// Current frame of simulation step
        current_frame: i32,
    },
}

impl SyncingInfo {
    /// Checks if the session is online.
    pub fn is_online(&self) -> bool {
        matches!(self, SyncingInfo::Online { .. })
    }

    /// Checks if the session is offline.
    pub fn is_offline(&self) -> bool {
        matches!(self, SyncingInfo::Offline { .. })
    }

    /// Getter for the current frame (number).
    pub fn current_frame(&self) -> i32 {
        match self {
            SyncingInfo::Online { current_frame, .. } => *current_frame,
            SyncingInfo::Offline { current_frame } => *current_frame,
        }
    }

    /// Getter for the last confirmed frame (number).
    pub fn last_confirmed_frame(&self) -> i32 {
        match self {
            SyncingInfo::Online {
                last_confirmed_frame,
                ..
            } => *last_confirmed_frame,
            SyncingInfo::Offline { current_frame } => *current_frame,
        }
    }
    /// Getter for socket.
    pub fn socket(&self) -> Option<&Socket> {
        match self {
            SyncingInfo::Online { socket, .. } => Some(socket),
            SyncingInfo::Offline { .. } => None,
        }
    }

    /// Mutable getter for socket.
    pub fn socket_mut(&mut self) -> Option<&mut Socket> {
        match self {
            SyncingInfo::Online { socket, .. } => Some(socket),
            SyncingInfo::Offline { .. } => None,
        }
    }

    /// Getter for a single player's network stats using their player_idx
    pub fn player_network_stats(&self, player_idx: usize) -> Option<PlayerNetworkStats> {
        match self {
            SyncingInfo::Online {
                players_network_stats,
                ..
            } => players_network_stats.get(player_idx).cloned(),
            SyncingInfo::Offline { .. } => None,
        }
    }

    /// Getter for all players' network stats, including local player (set to default). This maintains index == player_idx.
    pub fn players_network_stats(&self) -> SVec<PlayerNetworkStats> {
        match self {
            SyncingInfo::Online {
                players_network_stats,
                ..
            } => players_network_stats.clone(),
            SyncingInfo::Offline { .. } => SVec::new(),
        }
    }

    /// Getter for remote player network stats (filtering out local player). This does not maintain index == player_idx.
    pub fn remote_players_network_stats(&self) -> SVec<PlayerNetworkStats> {
        match self {
            SyncingInfo::Online {
                players_network_stats,
                ..
            } => players_network_stats
                .iter()
                .filter(|&stats| stats.ping != 0 || stats.kbps_sent != 0)
                .cloned()
                .collect(),
            SyncingInfo::Offline { .. } => SVec::new(),
        }
    }

    /// Calculates the total kilobits per second sent across all remote players.
    pub fn total_kbps_sent(&self) -> usize {
        self.remote_players_network_stats()
            .iter()
            .map(|stats| stats.kbps_sent)
            .sum()
    }

    /// Calculates the average kilobits per second sent across all remote players.
    pub fn averaged_kbps_sent(&self) -> f32 {
        let remote_stats = self.remote_players_network_stats();
        if remote_stats.is_empty() {
            0.0
        } else {
            let total_kbps: usize = remote_stats.iter().map(|stats| stats.kbps_sent).sum();
            total_kbps as f32 / remote_stats.len() as f32
        }
    }

    /// Returns the highest number of local frames behind across all remote players.
    pub fn highest_local_frames_behind(&self) -> i32 {
        self.remote_players_network_stats()
            .iter()
            .map(|stats| stats.local_frames_behind)
            .max()
            .unwrap_or(0)
    }

    /// Returns the highest number of remote frames behind across all remote players.
    pub fn highest_remote_frames_behind(&self) -> i32 {
        self.remote_players_network_stats()
            .iter()
            .map(|stats| stats.remote_frames_behind)
            .max()
            .unwrap_or(0)
    }

    /// Calculates the average ping across all remote players.
    pub fn averaged_ping(&self) -> u128 {
        let remote_stats = self.remote_players_network_stats();
        if remote_stats.is_empty() {
            0
        } else {
            let total_ping: u128 = remote_stats.iter().map(|stats| stats.ping).sum();
            total_ping / remote_stats.len() as u128
        }
    }

    /// Returns the lowest ping across all remote players.
    pub fn lowest_ping(&self) -> u128 {
        self.remote_players_network_stats()
            .iter()
            .map(|stats| stats.ping)
            .min()
            .unwrap_or(0)
    }

    /// Returns the highest ping across all remote players.
    pub fn highest_ping(&self) -> u128 {
        self.remote_players_network_stats()
            .iter()
            .map(|stats| stats.ping)
            .max()
            .unwrap_or(0)
    }

    /// Getter for the local player index, if offline defaults to None.
    pub fn local_player_idx_checked(&self) -> Option<usize> {
        match self {
            SyncingInfo::Online {
                local_player_idx, ..
            } => Some(*local_player_idx),
            SyncingInfo::Offline { .. } => None,
        }
    }

    /// Getter for the local player index, if offline defaults to 0.
    pub fn local_player_idx(&self) -> usize {
        match self {
            SyncingInfo::Online {
                local_player_idx, ..
            } => *local_player_idx,
            SyncingInfo::Offline { .. } => 0,
        }
    }

    /// Getter for the local frame delay.
    pub fn local_frame_delay(&self) -> usize {
        match self {
            SyncingInfo::Online {
                local_frame_delay, ..
            } => *local_frame_delay,
            SyncingInfo::Offline { .. } => 0,
        }
    }

    /// Getter for the number of players, if offline defaults to 0.
    pub fn players_count(&self) -> usize {
        match self {
            SyncingInfo::Online {
                players_network_stats,
                ..
            } => players_network_stats.len(),
            SyncingInfo::Offline { .. } => 0,
        }
    }

    /// Getter for the number of players, if offline defaults to None.
    pub fn players_count_checked(&self) -> Option<usize> {
        match self {
            SyncingInfo::Online {
                players_network_stats,
                ..
            } => Some(players_network_stats.len()),
            SyncingInfo::Offline { .. } => None,
        }
    }

    /// Getter for the list of active players (idx) which are connected. Offline returns empty list.
    pub fn active_players(&self) -> SVec<usize> {
        match self {
            SyncingInfo::Online {
                players_network_stats,
                disconnected_players,
                ..
            } => {
                let total_players = players_network_stats.len();
                (0..total_players)
                    .filter(|&id| !disconnected_players.contains(&id))
                    .collect()
            }
            SyncingInfo::Offline { .. } => SVec::new(),
        }
    }

    /// Getter for the list of active players (idx) which are connected. Offline returns None.
    pub fn active_players_checked(&self) -> Option<SVec<usize>> {
        match self {
            SyncingInfo::Online {
                players_network_stats,
                disconnected_players,
                ..
            } => {
                let total_players = players_network_stats.len();
                let active = (0..total_players)
                    .filter(|&id| !disconnected_players.contains(&id))
                    .collect();
                Some(active)
            }
            SyncingInfo::Offline { .. } => None,
        }
    }

    /// Getter for the list of players which have been disconnected (their idx). Offline returns empty list.
    pub fn disconnected_players(&self) -> SVec<usize> {
        match self {
            SyncingInfo::Online {
                disconnected_players,
                ..
            } => disconnected_players.clone(),
            SyncingInfo::Offline { .. } => SVec::new(),
        }
    }

    /// Getter for the list of players which have been disconnected (their idx). Offline returns None.
    pub fn disconnected_players_checked(&self) -> Option<SVec<usize>> {
        match self {
            SyncingInfo::Online {
                disconnected_players,
                ..
            } => Some(disconnected_players.clone()),
            SyncingInfo::Offline { .. } => None,
        }
    }
}

/// Resource tracking which players have been disconnected.
/// May not be in world if no disconnects.
///
/// If rollback to frame before disconnect, player handle is still included here.
#[derive(HasSchema, Clone, Default)]
pub struct DisconnectedPlayers {
    /// Handles of players that have been disconnected.
    pub disconnected_players: Vec<usize>,
}

/// [`SessionRunner`] implementation that uses [`ggrs`] for network play.
///
/// This is where the whole `ggrs` integration is implemented.
pub struct GgrsSessionRunner<'a, InputTypes: NetworkInputConfig<'a>> {
    /// The last player input we detected.
    pub last_player_input: InputTypes::Dense,

    /// The GGRS peer-to-peer session.
    pub session: P2PSession<GgrsConfig<InputTypes::Dense>>,

    /// Local player idx.
    pub player_idx: u32,

    /// Index of local player, computed from player_is_local
    pub local_player_idx: u32,

    /// The frame time accumulator, used to produce a fixed refresh rate.
    pub accumulator: f64,

    /// Timestamp of last time session was run to compute delta time.
    pub last_run: Option<Instant>,

    /// FPS from game adjusted with constant network factor (may be slightly slower)
    pub network_fps: f64,

    /// FPS from game not adjusted with network factor.
    pub original_fps: f64,

    /// Session runner's input collector.
    pub input_collector: InputTypes::InputCollector,

    /// Is local input disabled? (No input will be used if set)
    pub local_input_disabled: bool,

    /// Players who have been reported disconnected by ggrs
    disconnected_players: Vec<usize>,

    /// Store copy of socket to be able to restart session runner with existing socket.
    socket: Socket,

    /// Local input delay ggrs session was initialized with
    local_input_delay: usize,

    /// When provided, desync detection is enabled. Contains settings for desync detection.
    detect_desyncs: Option<DetectDesyncs>,

    /// History buffer for desync debug data to fetch it upon detected desyncs.
    /// [`DefaultDesyncTree`] will be generated and saved here if feature `desync-debug` is enabled.
    pub desync_debug_history: Option<DesyncDebugHistoryBuffer<DefaultDesyncTree>>,
}

/// The info required to create a [`GgrsSessionRunner`].
#[derive(Clone)]
pub struct GgrsSessionRunnerInfo {
    /// The socket that will be converted into GGRS socket implementation.
    pub socket: Socket,
    /// The local player idx
    pub player_idx: u32,
    /// the player count.
    pub player_count: u32,

    /// Max prediction window (max number of frames client may predict ahead of last confirmed frame)
    /// `None` will use Bone's default.
    pub max_prediction_window: Option<usize>,

    /// Local input delay (local inputs + remote inputs will be buffered and sampled this many frames later)
    /// Increasing helps with mitigating pops when remote user changes input quickly, and reduces amount of frames
    /// client will end up predicted ahead from others, helps with high latency.
    ///
    /// `None` will use Bone's default.
    pub local_input_delay: Option<usize>,

    /// When provided, desync detection is enabled. Contains settings for desync detection.
    pub detect_desyncs: Option<DetectDesyncs>,
}

impl GgrsSessionRunnerInfo {
    /// See [`GgrsSessionRunnerInfo`] fields for info on arguments.
    pub fn new(
        socket: Socket,
        max_prediction_window: Option<usize>,
        local_input_delay: Option<usize>,
        detect_desyncs: Option<DetectDesyncs>,
    ) -> Self {
        let player_idx = socket.player_idx();
        let player_count = socket.player_count();
        Self {
            socket,
            player_idx,
            player_count,
            max_prediction_window,
            local_input_delay,
            detect_desyncs,
        }
    }
}

impl<'a, InputTypes> GgrsSessionRunner<'a, InputTypes>
where
    InputTypes: NetworkInputConfig<'a>,
{
    /// Create a new sessino runner.
    pub fn new(simulation_fps: f32, info: GgrsSessionRunnerInfo) -> Self
    where
        Self: Sized,
    {
        // Modified FPS may not be an integer, but ggrs requires integer fps, so we clamp and round
        // to integer so our computed timestep will match  that of ggrs.
        let network_fps = (simulation_fps * NETWORK_FRAME_RATE_FACTOR) as f64;
        let network_fps = network_fps
            .max(usize::MIN as f64)
            .min(usize::MAX as f64)
            .round() as usize;

        // There may be value in dynamically negotitaing these values based on client's pings
        // before starting the match.
        let max_prediction = info
            .max_prediction_window
            .unwrap_or(NETWORK_MAX_PREDICTION_WINDOW_DEFAULT);
        let local_input_delay = info
            .local_input_delay
            .unwrap_or(NETWORK_LOCAL_INPUT_DELAY_DEFAULT);

        // Notify debugger of setting
        #[cfg(feature = "net-debug")]
        NETWORK_DEBUG_CHANNEL
            .sender
            .try_send(NetworkDebugMessage::SetMaxPrediction(max_prediction))
            .unwrap();

        let desync_detection = match info.detect_desyncs.as_ref() {
            Some(config) => DesyncDetection::On {
                interval: config.detection_interval,
            },
            None => DesyncDetection::Off,
        };

        let mut builder = ggrs::SessionBuilder::new()
            .with_num_players(info.player_count as usize)
            .with_input_delay(local_input_delay)
            .with_fps(network_fps)
            .unwrap()
            .with_desync_detection_mode(desync_detection)
            .with_max_prediction_window(max_prediction)
            .unwrap();

        let local_player_idx = info.player_idx;
        for i in 0..info.player_count {
            if i == info.player_idx {
                builder = builder
                    .add_player(ggrs::PlayerType::Local, i as usize)
                    .unwrap();
            } else {
                builder = builder
                    .add_player(ggrs::PlayerType::Remote(i as usize), i as usize)
                    .unwrap();
            }
        }

        let session = builder.start_p2p_session(info.socket.clone()).unwrap();

        #[cfg(feature = "desync-debug")]
        let desync_debug_history = if let Some(detect_desync) = info.detect_desyncs.as_ref() {
            Some(DesyncDebugHistoryBuffer::<DefaultDesyncTree>::new(
                detect_desync.detection_interval,
            ))
        } else {
            None
        };

        #[cfg(not(feature = "desync-debug"))]
        let desync_debug_history = None;

        Self {
            last_player_input: InputTypes::Dense::default(),
            session,
            player_idx: info.player_idx,
            local_player_idx,
            accumulator: default(),
            last_run: None,
            network_fps: network_fps as f64,
            original_fps: simulation_fps as f64,
            disconnected_players: default(),
            input_collector: InputTypes::InputCollector::default(),
            socket: info.socket.clone(),
            local_input_delay,
            local_input_disabled: false,
            detect_desyncs: info.detect_desyncs,
            desync_debug_history,
        }
    }
}

/// Helper for accessing nested associated types on [`NetworkInputConfig`].
#[allow(type_alias_bounds)]
type ControlMapping<'a, C: NetworkInputConfig<'a>> =
    <C::PlayerControls as PlayerControls<'a, C::Control>>::ControlMapping;

impl<InputTypes> SessionRunner for GgrsSessionRunner<'static, InputTypes>
where
    InputTypes: NetworkInputConfig<'static> + 'static,
{
    fn step(&mut self, frame_start: Instant, world: &mut World, stages: &mut SystemStages) {
        let step: f64 = 1.0 / self.network_fps;

        let last_run = self.last_run.unwrap_or(frame_start);
        let delta = (frame_start - last_run).as_secs_f64();
        self.accumulator += delta;

        let mut skip_frames: u32 = 0;

        {
            let keyboard = world.resource::<KeyboardInputs>();
            let gamepad = world.resource::<GamepadInputs>();

            let player_inputs = world.resource::<InputTypes::PlayerControls>();

            // Collect inputs and update controls
            self.input_collector.apply_inputs(
                &world.resource::<ControlMapping<InputTypes>>(),
                &keyboard,
                &gamepad,
            );
            self.input_collector.update_just_pressed();

            // save local players dense input for use with ggrs
            match player_inputs.get_control_source(self.local_player_idx as usize) {
                Some(control_source) => {
                    let control = self
                        .input_collector
                        .get_control(self.local_player_idx as usize, control_source);

                    self.last_player_input = control.get_dense_input();
                },
                None => warn!("GgrsSessionRunner local_player_idx {} has no control source, no local input provided.",
                    self.local_player_idx)
            };
        }

        #[cfg(feature = "net-debug")]
        // Current frame before we start network update loop
        let current_frame_original = self.session.current_frame();

        for event in self.session.events() {
            match event {
                ggrs::GgrsEvent::Synchronizing { addr, total, count } => {
                    info!(player=%addr, %total, progress=%count, "Syncing network player");

                    #[cfg(feature = "net-debug")]
                    NETWORK_DEBUG_CHANNEL
                        .sender
                        .try_send(NetworkDebugMessage::PlayerSync((
                            PlayerSyncState::SyncInProgress,
                            addr,
                        )))
                        .unwrap();
                }
                ggrs::GgrsEvent::Synchronized { addr } => {
                    info!(player=%addr, "Syncrhonized network client");

                    #[cfg(feature = "net-debug")]
                    NETWORK_DEBUG_CHANNEL
                        .sender
                        .try_send(NetworkDebugMessage::PlayerSync((
                            PlayerSyncState::Sychronized,
                            addr,
                        )))
                        .unwrap();
                }
                ggrs::GgrsEvent::Disconnected { addr } => {
                    warn!(player=%addr, "Player Disconnected");
                    self.disconnected_players.push(addr);

                    #[cfg(feature = "net-debug")]
                    NETWORK_DEBUG_CHANNEL
                        .sender
                        .try_send(NetworkDebugMessage::DisconnectedPlayers(
                            self.disconnected_players.clone(),
                        ))
                        .unwrap();
                } //return Err(SessionError::Disconnected)},
                ggrs::GgrsEvent::NetworkInterrupted { addr, .. } => {
                    info!(player=%addr, "Network player interrupted");
                }
                ggrs::GgrsEvent::NetworkResumed { addr } => {
                    info!(player=%addr, "Network player re-connected");
                }
                ggrs::GgrsEvent::WaitRecommendation {
                    skip_frames: skip_count,
                } => {
                    info!(
                        "Skipping {skip_count} frames to give network players a chance to catch up"
                    );
                    skip_frames = skip_count;

                    #[cfg(feature = "net-debug")]
                    NETWORK_DEBUG_CHANNEL
                        .sender
                        .try_send(NetworkDebugMessage::SkipFrame {
                            frame: current_frame_original,
                            count: skip_count,
                        })
                        .unwrap();
                }
                ggrs::GgrsEvent::DesyncDetected {
                    frame,
                    local_checksum,
                    remote_checksum,
                    addr,
                } => {
                    error!(%frame, %local_checksum, %remote_checksum, player=%addr, "Network de-sync detected");

                    #[cfg(feature = "desync-debug")]
                    {
                        if let Some(desync_debug_history) = &self.desync_debug_history {
                            if let Some(desync_hash_tree) =
                                desync_debug_history.get_frame_data(frame as u32)
                            {
                                let string = serde_yaml::to_string(desync_hash_tree)
                                    .expect("Failed to serialize desync hash tree");
                                error!("Desync hash tree: frame: {frame}\n{}", string);
                            }
                        }
                    }
                }
            }
        }

        loop {
            if self.accumulator >= step {
                self.accumulator -= step;

                if !self.local_input_disabled {
                    self.session
                        .add_local_input(self.local_player_idx as usize, self.last_player_input)
                        .unwrap();
                } else {
                    // If local input is disabled, we still submit a default value representing no-inputs.
                    // This way if input is disabled current inputs will not be held down indefinitely.
                    self.session
                        .add_local_input(
                            self.local_player_idx as usize,
                            InputTypes::Dense::default(),
                        )
                        .unwrap();
                }

                let current_frame = self.session.current_frame();

                #[cfg(feature = "net-debug")]
                {
                    let confirmed_frame = self.session.confirmed_frame();

                    NETWORK_DEBUG_CHANNEL
                        .sender
                        .try_send(NetworkDebugMessage::FrameUpdate {
                            current: current_frame,
                            last_confirmed: confirmed_frame,
                        })
                        .unwrap();
                }

                if skip_frames > 0 {
                    skip_frames = skip_frames.saturating_sub(1);
                    continue;
                }

                match self.session.advance_frame() {
                    Ok(requests) => {
                        for request in requests {
                            match request {
                                ggrs::GgrsRequest::SaveGameState { cell, frame } => {
                                    // TODO: Do we only need to compute hash for desync interval frames?
                                    // GGRS should only use hashes from fixed interval.

                                    // If desync detection enabled, hash world.
                                    let checksum = if let Some(detect_desyncs) =
                                        self.detect_desyncs.as_ref()
                                    {
                                        #[cfg(feature = "desync-debug")]
                                        {
                                            if let Some(desync_debug_history) =
                                                &mut self.desync_debug_history
                                            {
                                                if desync_debug_history
                                                    .is_desync_detect_frame(frame as u32)
                                                {
                                                    let tree = DefaultDesyncTree::from(
                                                        world.desync_tree_node::<FxHasher>(
                                                            detect_desyncs.include_unhashable_nodes,
                                                        ),
                                                    );
                                                    desync_debug_history.record(frame as u32, tree);
                                                }
                                            }
                                        }

                                        if let Some(hash_func) = detect_desyncs.world_hash_func {
                                            Some(hash_func(world) as u128)
                                        } else {
                                            let mut hasher = FxHasher::default();
                                            world.hash(&mut hasher);
                                            Some(hasher.finish() as u128)
                                        }
                                    } else {
                                        None
                                    };

                                    cell.save(frame, Some(world.clone()), checksum);
                                }
                                ggrs::GgrsRequest::LoadGameState { cell, frame } => {
                                    // Swap out sessions to preserve them after world save.
                                    // Sessions clone makes empty copy, so saved snapshots do not include sessions.
                                    // Sessions are borrowed from Game for execution of this session,
                                    // they are not like other resources and should not be preserved.
                                    let mut sessions = Sessions::default();
                                    std::mem::swap(
                                        &mut sessions,
                                        &mut world.resource_mut::<Sessions>(),
                                    );
                                    *world = cell.load().unwrap_or_default();
                                    std::mem::swap(
                                        &mut sessions,
                                        &mut world.resource_mut::<Sessions>(),
                                    );

                                    trace!("Loading (rollback) frame: {frame}");
                                }
                                ggrs::GgrsRequest::AdvanceFrame {
                                    inputs: network_inputs,
                                } => {
                                    // Input has been consumed, signal that we are in new input frame
                                    self.input_collector.advance_frame();

                                    // Fetch the PlayerNetworkStats for each remote player, guaranteeing each one is inserted into the index matching its handle
                                    let mut players_network_stats: Vec<PlayerNetworkStats> = vec![
                                        PlayerNetworkStats::default();
                                        self.session.remote_player_handles().len() + 1 // + 1 for the local player to maintain correct length
                                    ];
                                    for handle in self.session.remote_player_handles().iter() {
                                        if let Ok(stats) = self.session.network_stats(*handle) {
                                            players_network_stats[*handle] =
                                                PlayerNetworkStats::from_ggrs_network_stats(
                                                    *handle, stats,
                                                );
                                        }
                                    }

                                    // TODO: Make sure NetworkInfo is initialized immediately when session is created,
                                    // even before a frame has advanced.
                                    //
                                    // The existance of this resource may be used to determine if in an online match, and there could
                                    // be race if expected it to exist but testing before first frame advance.
                                    world.insert_resource(SyncingInfo::Online {
                                        current_frame: self.session.current_frame(),
                                        last_confirmed_frame: self.session.confirmed_frame(),
                                        socket: self.socket.clone(),
                                        players_network_stats: players_network_stats.into(),
                                        local_player_idx: self.local_player_idx as usize,
                                        local_frame_delay: self.local_input_delay,
                                        disconnected_players: self
                                            .disconnected_players
                                            .clone()
                                            .into(),
                                    });

                                    // Disconnected players persisted on session runner, and updated each frame.
                                    // This avoids a rollback from changing resource state.
                                    world.insert_resource(DisconnectedPlayers {
                                        disconnected_players: self.disconnected_players.clone(),
                                    });

                                    {
                                        world
                                            .resource_mut::<Time>()
                                            .advance_exact(Duration::from_secs_f64(step));

                                        // update game controls from ggrs inputs
                                        let mut player_inputs =
                                            world.resource_mut::<InputTypes::PlayerControls>();
                                        for (player_idx, (input, status)) in
                                            network_inputs.into_iter().enumerate()
                                        {
                                            trace!(
                                                "Net player({player_idx}) local: {}, status: {status:?}, frame: {current_frame} input: {:?}",
                                                self.local_player_idx == player_idx as u32,
                                                input
                                            );
                                            player_inputs.network_update(
                                                player_idx,
                                                &input,
                                                status.into(),
                                            );
                                        }
                                    }

                                    // Run game session stages, advancing simulation
                                    stages.run(world);
                                }
                            }
                        }
                    }
                    Err(e) => match e {
                        ggrs::GgrsError::NotSynchronized => {
                            debug!("Waiting for network clients to sync")
                        }
                        ggrs::GgrsError::PredictionThreshold => {
                            warn!("Freezing game while waiting for network to catch-up.");

                            #[cfg(feature = "net-debug")]
                            NETWORK_DEBUG_CHANNEL
                                .sender
                                .try_send(NetworkDebugMessage::FrameFroze {
                                    frame: self.session.current_frame(),
                                })
                                .unwrap();
                        }
                        e => error!("Network protocol error: {e}"),
                    },
                }
            } else {
                break;
            }
        }

        self.last_run = Some(frame_start);

        // Fetch GGRS network stats of remote players and send to net debug tool
        #[cfg(feature = "net-debug")]
        {
            let mut network_stats: Vec<(PlayerHandle, NetworkStats)> = vec![];
            for handle in self.session.remote_player_handles().iter() {
                if let Ok(stats) = self.session.network_stats(*handle) {
                    network_stats.push((*handle, stats));
                }
            }
            if !network_stats.is_empty() {
                NETWORK_DEBUG_CHANNEL
                    .sender
                    .try_send(NetworkDebugMessage::NetworkStats { network_stats })
                    .unwrap();
            }
        }
    }

    fn restart_session(&mut self) {
        // Rebuild session info from runner + create new runner

        // Increment match id so messages from previous match that are still in flight
        // will be filtered out.
        self.socket.increment_match_id();

        let runner_info = GgrsSessionRunnerInfo {
            socket: self.socket.clone(),
            player_idx: self.player_idx,
            player_count: self.session.num_players().try_into().unwrap(),
            max_prediction_window: Some(self.session.max_prediction()),
            local_input_delay: Some(self.local_input_delay),
            detect_desyncs: self.detect_desyncs.clone(),
        };
        *self = GgrsSessionRunner::new(self.original_fps as f32, runner_info);
    }

    fn disable_local_input(&mut self, input_disabled: bool) {
        self.local_input_disabled = input_disabled;
    }
}

/// A schema-compatible wrapper for ggrs `NetworkStats` struct contains networking stats.
#[derive(Debug, Default, Clone, Copy, HasSchema)]
pub struct PlayerNetworkStats {
    /// The idx of the player these stats are for. Included here for self-attesting/ease-of-access.
    pub player_idx: usize,
    /// The length of the queue containing UDP packets which have not yet been acknowledged by the end client.
    /// The length of the send queue is a rough indication of the quality of the connection. The longer the send queue, the higher the round-trip time between the
    /// clients. The send queue will also be longer than usual during high packet loss situations.
    pub send_queue_len: usize,
    /// The roundtrip packet transmission time as calculated by GGRS.
    pub ping: u128,
    /// The estimated bandwidth used between the two clients, in kilobits per second.
    pub kbps_sent: usize,

    /// The number of frames GGRS calculates that the local client is behind the remote client at this instant in time.
    /// For example, if at this instant the current game client is running frame 1002 and the remote game client is running frame 1009,
    /// this value will mostly likely roughly equal 7.
    pub local_frames_behind: i32,
    /// The same as [`local_frames_behind`], but calculated from the perspective of the remote player.
    ///
    /// [`local_frames_behind`]: #structfield.local_frames_behind
    pub remote_frames_behind: i32,
}

impl PlayerNetworkStats {
    /// Creates a new PlayerNetworkStats from a player index and a ggrs NetworkStats.
    pub fn from_ggrs_network_stats(player_idx: usize, stats: ggrs::NetworkStats) -> Self {
        Self {
            player_idx,
            send_queue_len: stats.send_queue_len,
            ping: stats.ping,
            kbps_sent: stats.kbps_sent,
            local_frames_behind: stats.local_frames_behind,
            remote_frames_behind: stats.remote_frames_behind,
        }
    }
}
