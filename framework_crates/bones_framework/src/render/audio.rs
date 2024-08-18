//! Audio components.
//! 

use std::collections::VecDeque;
use kira::sound::PlaybackState;
use std::io::Cursor;
use tracing::warn;
use crate::prelude::*;
pub use kira::{self, sound::static_sound::StaticSoundData};
use kira::{
    manager::{
        backend::{cpal::CpalBackend, mock::MockBackend, Backend},
        AudioManager as KiraAudioManager,
    },
    sound::SoundData,
    tween::Tween,
    sound::static_sound::{StaticSoundHandle, StaticSoundSettings},
};
use kira::{Volume, tween};
use std::time::Duration;

/// The amount of time to spend fading the music in and out.
pub const MUSIC_FADE_DURATION: Duration = Duration::from_millis(500);
/// Default music volume
pub const DEFAULT_MUSIC_VOLUME: f64 = 0.1;
/// Name of the default bones audio session
pub const DEFAULT_BONES_AUDIO_SESSION: &'static str  = "BONES_AUDIO";

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
        .add_system_to_stage(First, process_audio_events)
        .add_system_to_stage(Last, kill_finished_audios);
}

/// A resource that can be used to control game audios.
#[derive(HasSchema)]
#[schema(no_clone)]
pub struct AudioCenter {
    /// Buffer for audio events that have not yet been processed.
    events: VecDeque<AudioEvent>,
    /// The handle to the current music.
    music: Option<Audio>,
    /// The volume scale for main audio.
    main_volume_scale: f32,
    /// The volume scale for music.
    music_volume_scale: f32,
    /// The volume scale for sound effects.
    effects_volume_scale: f32,
}

impl Default for AudioCenter {
    fn default() -> Self {
        Self {
            events: VecDeque::with_capacity(16),
            music: None,
            main_volume_scale: 1.0,
            music_volume_scale: 1.0,
            effects_volume_scale: 1.0,
        }
    }
}

impl AudioCenter {
    /// Push an audio event to the queue for later processing.
    pub fn push_event(&mut self, event: AudioEvent) {
        self.events.push_back(event);
    }

    /// Get the playback state of the music.
    pub fn music_state(&self) -> Option<PlaybackState> {
        self.music.as_ref().map(|m| m.handle.state())
    }

    /// Play a sound. These are usually short audios that indicate something
    /// happened in game, e.g. a player jump, an explosion, etc.
    /// Volume is scaled by both main_volume_scale, and effects_volume_scale.
    pub fn play_sound(&mut self, sound_source: Handle<AudioSource>, volume: f64) {
        self.events.push_back(AudioEvent::PlaySound {
            sound_source,
            volume,
        })
    }

    /// Play some music, any current music is stopped. These may or may not loop.
    /// Volume is scaled by both main_volume_scale, and music_volume_scale.
    pub fn play_music(
        &mut self,
        sound_source: Handle<AudioSource>,
        sound_settings: StaticSoundSettings,
    ) {
        self.events.push_back(AudioEvent::PlayMusic {
            sound_source,
            sound_settings: Box::new(sound_settings),
        });
    }

    /// Sets the volume scale for main audio within the range of 0.0 to 1.0.
    pub fn set_main_volume_scale(&mut self, main: f32) {
        self.main_volume_scale = main.clamp(0.0, 1.0);
    }

    /// Sets the volume scale for music within the range of 0.0 to 1.0.
    pub fn set_music_volume_scale(&mut self, music: f32) {
        self.music_volume_scale = music.clamp(0.0, 1.0);
    }

    /// Sets the volume scale for effects within the range of 0.0 to 1.0.
    pub fn set_effects_volume_scale(&mut self, effects: f32) {
        self.effects_volume_scale = effects.clamp(0.0, 1.0);
    }

    /// Sets the volume scales for main, music, and effects within the range of 0.0 to 1.0.
    pub fn set_volume_scales(&mut self, main: f32, music: f32, effects: f32) {
        self.set_main_volume_scale(main);
        self.set_music_volume_scale(music);
        self.set_effects_volume_scale(effects);
        self.events.push_back(AudioEvent::VolumeScaleChange {
            main_volume: self.main_volume_scale,
            music_volume: self.music_volume_scale,
            effects_volume: self.effects_volume_scale,
        });
    }
}

/// An audio event that may be sent to the [`AudioCenter`] resource for
/// processing.
#[derive(Clone, Debug)]
pub enum AudioEvent {
    /// Update the volume of all audios using the new scale values.
    /// This event is used to adjust the overall volume of the application.
    VolumeScaleChange {
        /// The main volume scale factor.
        main_volume: f32,
        /// The music volume scale factor.
        music_volume: f32,
        /// The effects volume scale factor.
        effects_volume: f32,
    },
    /// Play some music.
    ///
    /// Any current music is stopped.
    PlayMusic {
        /// The handle for the music.
        sound_source: Handle<AudioSource>,
        /// The settings for the music.
        sound_settings: Box<StaticSoundSettings>,
    },
    /// Play a sound.
    PlaySound {
        /// The handle to the sound to play.
        sound_source: Handle<AudioSource>,
        /// The volume to play the sound at.
        volume: f64,
    },
}

/// Holds the handle and the volume to be played for a piece of Audio. 
#[derive(HasSchema)]
#[schema(no_clone, no_default, opaque)]
#[repr(C)]
pub struct Audio {
    /// The handle for the audio.
    handle: StaticSoundHandle,
    /// The original volume requested for the audio.
    volume: f64,
}

fn process_audio_events(
    mut audio_manager: ResMut<AudioManager>,
    mut audio_center: ResMut<AudioCenter>,
    assets: ResInit<AssetServer>,
    mut entities: ResMut<Entities>,
    mut audios: CompMut<Audio>,
) {
    for event in audio_center.events.drain(..).collect::<Vec<_>>() {
        match event {
            AudioEvent::VolumeScaleChange {
                main_volume,
                music_volume,
                effects_volume,
            } => {
                let tween = Tween::default();
                // Update music volume
                if let Some(music) = &mut audio_center.music {
                    let volume = (main_volume as f64) * (music_volume as f64) * music.volume;
                    if let Err(err) = music.handle.set_volume(volume, tween) {
                        warn!("Error setting music volume: {err}");
                    }
                }
                // Update sound volumes
                for audio in audios.iter_mut() {
                    let volume = (main_volume as f64) * (effects_volume as f64) * audio.volume;
                    if let Err(err) = audio.handle.set_volume(volume, tween) {
                        warn!("Error setting audio volume: {err}");
                    }
                }
            }
            AudioEvent::PlayMusic {
                sound_source,
                mut sound_settings,
            } => {
                // Stop the current music
                if let Some(mut music) = audio_center.music.take() {
                    let tween = Tween {
                        start_time: kira::StartTime::Immediate,
                        duration: MUSIC_FADE_DURATION,
                        easing: tween::Easing::Linear,
                    };
                    music.handle.stop(tween).unwrap();
                }
                // Scale the requested volume by the volume scales
                let volume = match sound_settings.volume {
                    tween::Value::Fixed(vol) => vol.as_amplitude(),
                    _ => DEFAULT_MUSIC_VOLUME,
                };
                let scaled_volume = (audio_center.main_volume_scale as f64) * (audio_center.music_volume_scale as f64) * volume;
                sound_settings.volume = tween::Value::Fixed(Volume::Amplitude(scaled_volume));
                // Play the new music
                let sound_data = assets.get(sound_source).with_settings(*sound_settings);
                match audio_manager.play(sound_data) {
                    Err(err) => warn!("Error playing music: {err}"),
                    Ok(handle) => audio_center.music = Some(Audio { handle, volume }),
                }
            }
            AudioEvent::PlaySound {
                sound_source,
                volume,
            } => {
                let scaled_volume = (audio_center.main_volume_scale as f64) * (audio_center.effects_volume_scale as f64) * volume;
                let sound_data = assets
                    .get(sound_source)
                    .with_settings(StaticSoundSettings::default().volume(scaled_volume));
                match audio_manager.play(sound_data) {
                    Err(err) => warn!("Error playing sound: {err}"),
                    Ok(handle) => {
                        let audio_ent = entities.create();
                        audios.insert(audio_ent, Audio { handle, volume });
                    }
                }
            }
        }
    }
}

fn kill_finished_audios(entities: Res<Entities>, audios: Comp<Audio>, mut commands: Commands) {
    for (audio_ent, audio) in entities.iter_with(&audios) {
        if audio.handle.state() == PlaybackState::Stopped {
            commands.add(move |mut entities: ResMut<Entities>| entities.kill(audio_ent));
        }
    }
}

/// The audio manager resource which can be used to play sounds.
#[derive(HasSchema, Deref, DerefMut)]
#[schema(no_clone)]
pub struct AudioManager(KiraAudioManager<CpalWithFallbackBackend>);
impl Default for AudioManager {
    fn default() -> Self {
        Self(KiraAudioManager::<CpalWithFallbackBackend>::new(default()).unwrap())
    }
}

/// Kira audio backend that will fall back to a dummy backend if setting up the Cpal backend
/// fails with an error.
#[allow(clippy::large_enum_variant)]
pub enum CpalWithFallbackBackend {
    /// This is a working Cpal backend.
    Cpal(CpalBackend),
    /// This is a dummy backend since Cpal didn't work.
    Dummy(MockBackend),
}

impl Backend for CpalWithFallbackBackend {
    type Settings = <CpalBackend as Backend>::Settings;
    type Error = <CpalBackend as Backend>::Error;

    fn setup(settings: Self::Settings) -> Result<(Self, u32), Self::Error> {
        match CpalBackend::setup(settings) {
            Ok((back, bit)) => Ok((Self::Cpal(back), bit)),
            Err(e) => {
                tracing::error!("Error starting audio backend, using dummy backend instead: {e}");
                Ok(MockBackend::setup(default())
                    .map(|(back, bit)| (Self::Dummy(back), bit))
                    .unwrap())
            }
        }
    }

    fn start(&mut self, renderer: kira::manager::backend::Renderer) -> Result<(), Self::Error> {
        match self {
            CpalWithFallbackBackend::Cpal(cpal) => cpal.start(renderer),
            CpalWithFallbackBackend::Dummy(dummy) => {
                dummy.start(renderer).unwrap();
                Ok(())
            }
        }
    }
}

/// The audio source asset type, contains no data, but [`Handle<AudioSource>`] is still useful
/// because it uniquely represents a sound/music that may be played outside of bones.
#[derive(Clone, HasSchema, Debug, Deref, DerefMut)]
#[schema(no_default)]
#[type_data(asset_loader(["ogg", "mp3", "flac", "wav"], AudioLoader))]
pub struct AudioSource(pub StaticSoundData);

impl SoundData for &AudioSource {
    type Error = <StaticSoundData as SoundData>::Error;
    type Handle = <StaticSoundData as SoundData>::Handle;

    fn into_sound(self) -> Result<(Box<dyn kira::sound::Sound>, Self::Handle), Self::Error> {
        self.0.clone().into_sound()
    }
}

/// The audio file asset loader.
pub struct AudioLoader;
impl AssetLoader for AudioLoader {
    fn load(
        &self,
        _ctx: AssetLoadCtx,
        bytes: &[u8],
    ) -> futures::future::Boxed<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let data = StaticSoundData::from_cursor(Cursor::new(bytes), default())?;
            Ok(SchemaBox::new(AudioSource(data)))
        })
    }
}