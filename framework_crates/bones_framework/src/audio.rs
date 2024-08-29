//! Audio session, systems, and resources.

pub mod audio_center;
pub mod audio_manager;

use crate::prelude::*;
pub use audio_center::*;
pub use audio_manager::*;
pub use kira;
pub use kira::sound::static_sound::StaticSoundData;
use kira::sound::static_sound::StaticSoundHandle;

/// Name of the default bones audio session
pub const DEFAULT_BONES_AUDIO_SESSION: &str = "BONES_AUDIO";

/// Sets up audio-related resources and the default bones audio session
pub fn game_plugin(game: &mut Game) {
    AudioSource::register_schema();
    game.init_shared_resource::<AudioCenter>();
    game.insert_shared_resource(AudioManager::default());
    game.init_shared_resource::<AssetServer>();

    let session = game.sessions.create(DEFAULT_BONES_AUDIO_SESSION);
    // Audio doesn't do any rendering
    session.visible = false;
    session
        .stages
        .add_system_to_stage(First, _process_audio_events)
        .add_system_to_stage(Last, _kill_finished_audios);
}

/// Holds the handles and the volume to be played for a piece of Audio.
#[derive(HasSchema)]
#[schema(no_clone, no_default, opaque)]
#[repr(C)]
pub struct Audio {
    /// The handle for the audio.
    handle: StaticSoundHandle,
    /// The original volume requested for the audio.
    volume: f64,
    /// The bones handle for the audio source.
    bones_handle: Handle<AudioSource>,
}
