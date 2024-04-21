#![doc = include_str!("./networking.md")]

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use ggrs::{NetworkStats, P2PSession, PlayerHandle};
use instant::Duration;
use once_cell::sync::Lazy;
use tracing::{debug, error, info, trace, warn};

use crate::prelude::*;

use self::{
    debug::{NetworkDebugMessage, NETWORK_DEBUG_CHANNEL},
    input::{DenseInput, NetworkInputConfig, NetworkPlayerControl, NetworkPlayerControls},
};
use crate::input::PlayerControls as PlayerControlsTrait;

pub mod certs;
pub mod debug;
pub mod input;
pub mod lan;
pub mod online;
pub mod proto;

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
    pub use super::{certs, debug::prelude::*, input, lan, online, proto, NetworkInfo};
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

// TODO: Remove this limitation on max players, a variety of types use this for static arrays,
// should either figure out how to make this a compile-time const value specified by game, or
// use dynamic arrays.
//
/// Max players in networked game
pub const MAX_PLAYERS: usize = 4;

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

/// The network endpoint used for all QUIC network communications.
pub static NETWORK_ENDPOINT: Lazy<quinn::Endpoint> = Lazy::new(|| {
    // Generate certificate
    let (cert, key) = certs::generate_self_signed_cert().unwrap();

    let mut transport_config = quinn::TransportConfig::default();
    transport_config.keep_alive_interval(Some(std::time::Duration::from_secs(5)));

    let mut server_config = quinn::ServerConfig::with_single_cert([cert].to_vec(), key).unwrap();
    server_config.transport = Arc::new(transport_config);

    // Open Socket and create endpoint
    let port = THREAD_RNG.with(|rng| rng.u16(10000..=11000));
    info!(port, "Started network endpoint");
    let socket = std::net::UdpSocket::bind(("0.0.0.0", port)).unwrap();

    let client_config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(certs::SkipServerVerification::new())
        .with_no_client_auth();
    let client_config = quinn::ClientConfig::new(Arc::new(client_config));

    let mut endpoint = quinn::Endpoint::new(
        quinn::EndpointConfig::default(),
        Some(server_config),
        socket,
        Arc::new(quinn_runtime_bevy::BevyIoTaskPoolExecutor),
    )
    .unwrap();

    endpoint.set_default_client_config(client_config);

    endpoint
});

/// Resource containing the [`NetworkSocket`] implementation while there is a connection to a
/// network game.
///
/// This is inserted into the world after a match has been established by a network matchmaker.
#[derive(Clone, HasSchema, Deref, DerefMut)]
#[schema(no_default)]
pub struct NetworkMatchSocket(Arc<dyn NetworkSocket>);

/// A type-erased [`ggrs::NonBlockingSocket`]
/// implementation.
#[derive(Deref, DerefMut)]
pub struct BoxedNonBlockingSocket(Box<dyn GgrsSocket>);

impl Clone for BoxedNonBlockingSocket {
    fn clone(&self) -> Self {
        self.ggrs_socket()
    }
}

/// Wraps [`ggrs::Message`] with included `match_id`, used to determine if message received
/// from current match.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameMessage {
    /// Socket match id
    pub match_id: u8,
    /// Wrapped message
    pub message: ggrs::Message,
}

/// Automatically implemented for [`NetworkSocket`] + [`ggrs::NonBlockingSocket<usize>`].
pub trait GgrsSocket: NetworkSocket + ggrs::NonBlockingSocket<usize> {}
impl<T> GgrsSocket for T where T: NetworkSocket + ggrs::NonBlockingSocket<usize> {}

impl ggrs::NonBlockingSocket<usize> for BoxedNonBlockingSocket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &usize) {
        self.0.send_to(msg, addr)
    }

    fn receive_all_messages(&mut self) -> Vec<(usize, ggrs::Message)> {
        self.0.receive_all_messages()
    }
}

/// Trait that must be implemented by socket connections establish by matchmakers.
///
/// The [`NetworkMatchSocket`] resource will contain an instance of this trait and will be used by
/// the game to send network messages after a match has been established.
pub trait NetworkSocket: Sync + Send {
    /// Get a GGRS socket from this network socket.
    fn ggrs_socket(&self) -> BoxedNonBlockingSocket;
    /// Send a reliable message to the given [`SocketTarget`].
    fn send_reliable(&self, target: SocketTarget, message: &[u8]);
    /// Receive reliable messages from other players. The `usize` is the index of the player that
    /// sent the message.
    fn recv_reliable(&self) -> Vec<(usize, Vec<u8>)>;
    /// Close the connection.
    fn close(&self);
    /// Get the player index of the local player.
    fn player_idx(&self) -> usize;
    /// Return, for every player index, whether the player is a local player.
    fn player_is_local(&self) -> [bool; MAX_PLAYERS];
    /// Get the player count for this network match.
    fn player_count(&self) -> usize;

    /// Increment match id so messages from previous match that are still in flight
    /// will be filtered out. Used when starting new session with existing socket.
    fn increment_match_id(&mut self);
}

/// The destination for a reliable network message.
pub enum SocketTarget {
    /// Send to a specific player.
    Player(usize),
    /// Broadcast to all players.
    All,
}

/// Resource updated each frame exposing current frame and last confirmed of online session.
#[derive(HasSchema, Clone)]
#[schema(no_default)]
pub struct NetworkInfo {
    /// Current frame of simulation step
    pub current_frame: i32,

    /// Last confirmed frame by all clients.
    /// Anything that occurred on this frame is agreed upon by all clients.
    pub last_confirmed_frame: i32,

    /// Socket
    pub socket: BoxedNonBlockingSocket,
}

/// [`SessionRunner`] implementation that uses [`ggrs`] for network play.
///
/// This is where the whole `ggrs` integration is implemented.
pub struct GgrsSessionRunner<'a, InputTypes: NetworkInputConfig<'a>> {
    /// The last player input we detected.
    pub last_player_input: InputTypes::Dense,

    /// The GGRS peer-to-peer session.
    pub session: P2PSession<GgrsConfig<InputTypes::Dense>>,

    /// Array containing a flag indicating, for each player, whether they are a local player.
    pub player_is_local: [bool; MAX_PLAYERS],

    /// Index of local player, computed from player_is_local
    pub local_player_idx: usize,

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

    /// Store copy of socket to be able to restart session runner with existing socket.
    socket: BoxedNonBlockingSocket,

    /// Local input delay ggrs session was initialized with
    local_input_delay: usize,
}

/// The info required to create a [`GgrsSessionRunner`].
#[derive(Clone)]
pub struct GgrsSessionRunnerInfo {
    /// The socket that will be converted into GGRS socket implementation.
    pub socket: BoxedNonBlockingSocket,
    /// The list of local players.
    pub player_is_local: [bool; MAX_PLAYERS],
    /// the player count.
    pub player_count: usize,

    /// Max prediction window (max number of frames client may predict ahead of last confirmed frame)
    /// `None` will use Bone's default.
    pub max_prediction_window: Option<usize>,

    /// Local input delay (local inputs + remote inputs will be buffered and sampled this many frames later)
    /// Increasing helps with mitigating pops when remote user changes input quickly, and reduces amount of frames
    /// client will end up predicted ahead from others, helps with high latency.
    ///
    /// `None` will use Bone's default.
    pub local_input_delay: Option<usize>,
}

impl GgrsSessionRunnerInfo {
    /// See [`GgrsSessionRunnerInfo`] fields for info on arguments.
    pub fn new(
        socket: BoxedNonBlockingSocket,
        max_prediction_window: Option<usize>,
        local_input_delay: Option<usize>,
    ) -> Self {
        let player_is_local = socket.0.player_is_local();
        let player_count = socket.0.player_count();
        Self {
            socket,
            player_is_local,
            player_count,
            max_prediction_window,
            local_input_delay,
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
            .max(std::usize::MIN as f64)
            .min(std::usize::MAX as f64)
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
        NETWORK_DEBUG_CHANNEL
            .sender
            .try_send(NetworkDebugMessage::SetMaxPrediction(max_prediction))
            .unwrap();

        let mut builder = ggrs::SessionBuilder::new()
            .with_num_players(info.player_count)
            .with_input_delay(local_input_delay)
            .with_fps(network_fps)
            .unwrap()
            .with_max_prediction_window(max_prediction)
            .unwrap();

        let mut local_player_idx: Option<usize> = None;
        for i in 0..info.player_count {
            if info.player_is_local[i] {
                builder = builder.add_player(ggrs::PlayerType::Local, i).unwrap();
                local_player_idx = Some(i);
            } else {
                builder = builder.add_player(ggrs::PlayerType::Remote(i), i).unwrap();
            }
        }
        let local_player_idx =
            local_player_idx.expect("Networking player_is_local array has no local players.");

        let session = builder.start_p2p_session(info.socket.clone()).unwrap();

        Self {
            last_player_input: InputTypes::Dense::default(),
            session,
            player_is_local: info.player_is_local,
            local_player_idx,
            accumulator: default(),
            last_run: None,
            network_fps: network_fps as f64,
            original_fps: simulation_fps as f64,
            input_collector: InputTypes::InputCollector::default(),
            socket: info.socket.clone(),
            local_input_delay,
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
            match player_inputs.get_control_source(self.local_player_idx) {
                Some(control_source) => {
                    let control = self
                        .input_collector
                        .get_control(self.local_player_idx, control_source);

                    self.last_player_input = control.get_dense_input();
                },
                None => warn!("GgrsSessionRunner local_player_idx {} has no control source, no local input provided.",
                    self.local_player_idx)
            };
        }

        let current_frame_original = self.session.current_frame();
        // Current frame before we start network update loop
        // let current_frame_original = self.session.current_frame();
        for event in self.session.events() {
            match event {
                ggrs::GgrsEvent::Synchronizing { addr, total, count } => {
                    info!(player=%addr, %total, progress=%count, "Syncing network player");
                }
                ggrs::GgrsEvent::Synchronized { addr } => {
                    info!(player=%addr, "Syncrhonized network client");
                }
                // TODO
                ggrs::GgrsEvent::Disconnected { .. } => {} //return Err(SessionError::Disconnected)},
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
                }
            }
        }

        loop {
            if self.accumulator >= step {
                self.accumulator -= step;

                self.session
                    .add_local_input(self.local_player_idx, self.last_player_input)
                    .unwrap();

                let current_frame = self.session.current_frame();
                let confirmed_frame = self.session.confirmed_frame();
                NETWORK_DEBUG_CHANNEL
                    .sender
                    .try_send(NetworkDebugMessage::FrameUpdate {
                        current: current_frame,
                        last_confirmed: confirmed_frame,
                    })
                    .unwrap();

                if skip_frames > 0 {
                    skip_frames = skip_frames.saturating_sub(1);
                    continue;
                }

                match self.session.advance_frame() {
                    Ok(requests) => {
                        for request in requests {
                            match request {
                                ggrs::GgrsRequest::SaveGameState { cell, frame } => {
                                    cell.save(frame, Some(world.clone()), None)
                                }
                                ggrs::GgrsRequest::LoadGameState { cell, .. } => {
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
                                }
                                ggrs::GgrsRequest::AdvanceFrame {
                                    inputs: network_inputs,
                                } => {
                                    // Input has been consumed, signal that we are in new input frame
                                    self.input_collector.advance_frame();

                                    world.insert_resource(NetworkInfo {
                                        current_frame: self.session.current_frame(),
                                        last_confirmed_frame: self.session.confirmed_frame(),
                                        socket: self.socket.clone(),
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
                                                "Net player({player_idx}) local: {}, status: {status:?}, input: {:?}",
                                                self.local_player_idx == player_idx,
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

    fn restart_session(&mut self) {
        // Rebuild session info from runner + create new runner

        // Increment match id so messages from previous match that are still in flight
        // will be filtered out.
        self.socket.0.increment_match_id();

        let runner_info = GgrsSessionRunnerInfo {
            socket: self.socket.clone(),
            player_is_local: self.player_is_local,
            player_count: self.session.num_players(),
            max_prediction_window: Some(self.session.max_prediction()),
            local_input_delay: Some(self.local_input_delay),
        };
        *self = GgrsSessionRunner::new(self.original_fps as f32, runner_info);
    }
}
