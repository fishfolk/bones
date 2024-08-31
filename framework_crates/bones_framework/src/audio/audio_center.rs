//! Audio Center resource and systems.

use super::{Audio, AudioManager, AudioSource};
use crate::prelude::*;
use kira;
use kira::{
    sound::{static_sound::StaticSoundSettings, PlaybackState},
    tween,
    tween::Tween,
    Volume,
};
use std::collections::VecDeque;
use std::time::Duration;
use tracing::warn;

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
    /// How long music should fade out for when stopping.
    music_fade_duration: Duration,
}

impl Default for AudioCenter {
    fn default() -> Self {
        Self {
            events: VecDeque::with_capacity(16),
            music: None,
            main_volume_scale: 1.0,
            music_volume_scale: 1.0,
            effects_volume_scale: 1.0,
            music_fade_duration: Duration::from_millis(500),
        }
    }
}

impl AudioCenter {
    /// Push an audio event to the queue for later processing.
    pub fn push_event(&mut self, event: AudioEvent) {
        self.events.push_back(event);
    }

    /// Returns the currently played music.
    pub fn music(&self) -> Option<&Audio> {
        self.music.as_ref()
    }

    /// Get the playback state of the current music.
    pub fn music_state(&self) -> Option<PlaybackState> {
        self.music().map(|m| m.handle.state())
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

    /// Plays music, forcibly stopping any current music.
    /// Volume is scaled by both main_volume_scale and music_volume_scale.
    pub fn play_music(&mut self, sound_source: Handle<AudioSource>, volume: f64, loop_music: bool) {
        let clamped_volume = volume.clamp(0.0, 1.0);
        let mut settings = StaticSoundSettings::new().volume(Volume::Amplitude(clamped_volume));

        if loop_music {
            settings = settings.loop_region(kira::sound::Region {
                start: 0.0.into(),
                end: kira::sound::EndPosition::EndOfAudio,
            });
        }

        self.events.push_back(AudioEvent::PlayMusic {
            sound_source,
            sound_settings: Box::new(settings),
            force_restart: true,
        });
    }

    /// Plays music with advanced settings.
    /// Volume is scaled by both main_volume_scale and music_volume_scale.
    ///
    /// # Parameters
    ///
    /// * `sound_source` - The handle for the audio source to play
    /// * `volume` - The volume to play the music at (0.0 to 1.0)
    /// * `loop_music` - Whether the music should loop indefinitely
    /// * `reverse` - Whether to play the audio in reverse
    /// * `start_position` - The position in seconds to start playback from
    /// * `playback_rate` - The playback rate (1.0 is normal speed, 0.5 is half speed, 2.0 is double speed)
    /// * `skip_restart` - If true, won't restart the music if it's already playing the same track
    #[allow(clippy::too_many_arguments)]
    pub fn play_music_advanced(
        &mut self,
        sound_source: Handle<AudioSource>,
        volume: f64,
        loop_music: bool,
        reverse: bool,
        start_position: f64,
        playback_rate: f64,
        skip_restart: bool,
    ) {
        let clamped_volume = volume.clamp(0.0, 1.0);
        let mut settings = StaticSoundSettings::new()
            .volume(Volume::Amplitude(clamped_volume))
            .start_position(kira::sound::PlaybackPosition::Seconds(start_position))
            .reverse(reverse)
            .playback_rate(playback_rate);

        if loop_music {
            settings = settings.loop_region(kira::sound::Region {
                start: 0.0.into(),
                end: kira::sound::EndPosition::EndOfAudio,
            });
        }

        self.events.push_back(AudioEvent::PlayMusic {
            sound_source,
            sound_settings: Box::new(settings),
            force_restart: !skip_restart,
        });
    }

    /// Plays music with custom StaticSoundSettings.
    /// Volume is scaled by both main_volume_scale and music_volume_scale.
    pub fn play_music_custom(
        &mut self,
        sound_source: Handle<AudioSource>,
        sound_settings: StaticSoundSettings,
        skip_restart: bool,
    ) {
        self.events.push_back(AudioEvent::PlayMusic {
            sound_source,
            sound_settings: Box::new(sound_settings),
            force_restart: !skip_restart,
        });
    }

    /// Sets the volume scale for main audio within the range of 0.0 to 1.0.
    /// The main volume scale impacts all other volume scales.
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
        self.events.push_back(AudioEvent::VolumeScaleUpdate {
            main_volume_scale: self.main_volume_scale,
            music_volume_scale: self.music_volume_scale,
            effects_volume_scale: self.effects_volume_scale,
        });
    }

    /// Returns the main volume scale (which impacts all other volume scales).
    pub fn main_volume_scale(&self) -> f32 {
        self.main_volume_scale
    }

    /// Returns the music volume scale.
    pub fn music_volume_scale(&self) -> f32 {
        self.music_volume_scale
    }

    /// Returns the volume scale for sound effects.
    pub fn effects_volume_scale(&self) -> f32 {
        self.effects_volume_scale
    }
    /// Returns the duration for music fade out.
    pub fn music_fade_duration(&self) -> Duration {
        self.music_fade_duration
    }

    /// Sets the duration for music fade out.
    pub fn set_music_fade_duration(&mut self, duration: Duration) {
        self.music_fade_duration = duration;
    }

    /// Stops the currently playing music
    pub fn stop_music(&mut self) {
        self.events.push_back(AudioEvent::StopMusic);
    }

    /// Stops all currently playing sounds.
    /// * `fade_out` - If true, fades out the sounds using the music fade duration. If false, stops the sounds instantly.
    pub fn stop_all_sounds(&mut self, fade_out: bool) {
        self.events
            .push_back(AudioEvent::StopAllSounds { fade_out });
    }
}

/// An audio event that may be sent to the [`AudioCenter`] resource for
/// processing.
#[derive(Clone, Debug)]
pub enum AudioEvent {
    /// Update the volume of all audios using the new scale values.
    /// This event is used to adjust the overall volume of the application.
    VolumeScaleUpdate {
        /// The main volume scale factor.
        main_volume_scale: f32,
        /// The music volume scale factor.
        music_volume_scale: f32,
        /// The effects volume scale factor.
        effects_volume_scale: f32,
    },
    /// Play some music.
    ///
    /// Any current music is stopped if force_restart is true or if the new music is different.
    PlayMusic {
        /// The handle for the music.
        sound_source: Handle<AudioSource>,
        /// The settings for the music.
        sound_settings: Box<StaticSoundSettings>,
        /// Whether to force restart the music even if it's the same as the current music.
        force_restart: bool,
    },
    /// Stop the currently playing music.
    StopMusic,
    /// Play a sound.
    PlaySound {
        /// The handle to the sound to play.
        sound_source: Handle<AudioSource>,
        /// The volume to play the sound at.
        volume: f64,
    },
    /// Stop all currently playing sounds.
    StopAllSounds {
        /// Whether to fade out the sounds or stop them instantly.
        fade_out: bool,
    },
}

/// Internally used sytem for processing audio events in the bones audio session.
pub fn _process_audio_events(
    mut audio_manager: ResMut<AudioManager>,
    mut audio_center: ResMut<AudioCenter>,
    assets: ResInit<AssetServer>,
    mut entities: ResMut<Entities>,
    mut audios: CompMut<Audio>,
) {
    for event in audio_center.events.drain(..).collect::<Vec<_>>() {
        match event {
            AudioEvent::VolumeScaleUpdate {
                main_volume_scale,
                music_volume_scale,
                effects_volume_scale,
            } => {
                let tween = Tween::default();
                // Update music volume
                if let Some(music) = &mut audio_center.music {
                    let volume =
                        (main_volume_scale as f64) * (music_volume_scale as f64) * music.volume;
                    music.handle.set_volume(volume, tween);
                }
                // Update sound volumes
                for audio in audios.iter_mut() {
                    let volume =
                        (main_volume_scale as f64) * (effects_volume_scale as f64) * audio.volume;
                    audio.handle.set_volume(volume, tween);
                }
            }
            AudioEvent::PlayMusic {
                sound_source,
                mut sound_settings,
                force_restart,
            } => {
                let should_play = force_restart
                    || audio_center.music.as_ref().map_or(true, |current_music| {
                        sound_source != current_music.bones_handle
                    });

                if should_play {
                    // Stop the current music
                    if let Some(mut music) = audio_center.music.take() {
                        let tween = Tween {
                            start_time: kira::StartTime::Immediate,
                            duration: audio_center.music_fade_duration,
                            easing: tween::Easing::Linear,
                        };
                        music.handle.stop(tween);
                    }
                    // Scale the requested volume by the volume scales
                    let volume = match sound_settings.volume {
                        tween::Value::Fixed(vol) => vol.as_amplitude(),
                        _ => 1.0,
                    };
                    let scaled_volume = (audio_center.main_volume_scale as f64)
                        * (audio_center.music_volume_scale as f64)
                        * volume;
                    sound_settings.volume = tween::Value::Fixed(Volume::Amplitude(scaled_volume));
                    // Play the new music
                    let sound_data = assets.get(sound_source).with_settings(*sound_settings);
                    match audio_manager.play(sound_data) {
                        Err(err) => warn!("Error playing music: {err}"),
                        Ok(handle) => {
                            audio_center.music = Some(Audio {
                                handle,
                                volume,
                                bones_handle: sound_source,
                            })
                        }
                    }
                }
            }
            AudioEvent::StopMusic => {
                if let Some(mut music) = audio_center.music.take() {
                    let tween = Tween {
                        start_time: kira::StartTime::Immediate,
                        duration: audio_center.music_fade_duration,
                        easing: tween::Easing::Linear,
                    };
                    music.handle.stop(tween);
                }
            }
            AudioEvent::StopAllSounds { fade_out } => {
                let tween = if fade_out {
                    Tween {
                        start_time: kira::StartTime::Immediate,
                        duration: audio_center.music_fade_duration,
                        easing: tween::Easing::Linear,
                    }
                } else {
                    Tween {
                        start_time: kira::StartTime::Immediate,
                        duration: Duration::from_secs_f64(0.001),
                        easing: tween::Easing::Linear,
                    }
                };

                for (_, audio) in entities.iter_with(&mut audios) {
                    audio.handle.stop(tween);
                }
            }
            AudioEvent::PlaySound {
                sound_source,
                volume,
            } => {
                let scaled_volume = (audio_center.main_volume_scale as f64)
                    * (audio_center.effects_volume_scale as f64)
                    * volume;
                let sound_data = assets
                    .get(sound_source)
                    .with_settings(StaticSoundSettings::default().volume(scaled_volume));
                match audio_manager.play(sound_data) {
                    Err(err) => warn!("Error playing sound: {err}"),
                    Ok(handle) => {
                        let audio_ent = entities.create();
                        audios.insert(
                            audio_ent,
                            Audio {
                                handle,
                                volume,
                                bones_handle: sound_source,
                            },
                        );
                    }
                }
            }
        }
    }
}

/// Internally used sytem for killing finished audios (generally sounds) which were emitted as separate entities.
/// Used in the bones audio session.
pub fn _kill_finished_audios(entities: Res<Entities>, audios: Comp<Audio>, mut commands: Commands) {
    for (audio_ent, audio) in entities.iter_with(&audios) {
        if audio.handle.state() == PlaybackState::Stopped {
            commands.add(move |mut entities: ResMut<Entities>| entities.kill(audio_ent));
        }
    }
}
