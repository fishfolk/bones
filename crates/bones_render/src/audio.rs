//! Audio components.

use std::collections::VecDeque;

use crate::prelude::*;

/// The audio source asset type, contains no data, but [`Handle<AudioSource>`] is still useful
/// because it uniquely represents a sound/music that may be played outside of bones.
#[derive(Copy, Clone, TypeUlid, Debug)]
#[ulid = "01GP2E03WS03EE9H65E90GZW2D"]
pub struct AudioSource;

/// Resource containing the audio event queue.
#[derive(Default, TypeUlid, Clone, Debug)]
#[ulid = "01GP7HESF20YKNKVNVCYDJS9DR"]
pub struct AudioEvents {
    /// List of audio events that haven't been handled by the audio system yet.
    pub queue: VecDeque<AudioEvent>,
}

impl AudioEvents {
    /// Add an event to the audio event queue.
    pub fn send(&mut self, event: AudioEvent) {
        self.queue.push_back(event);
    }

    /// Play a sound.
    ///
    /// Shortcut for sending an [`AudioEvent`] with [`send()`][Self::send].
    pub fn play(&mut self, sound_source: Handle<AudioSource>, volume: f64) {
        self.queue.push_back(AudioEvent::PlaySound {
            sound_source,
            volume,
        })
    }
}

/// An audio event that may be sent to the [`AudioEvents`] resource.
#[derive(Clone, Debug)]
pub enum AudioEvent {
    /// Play a sound.
    PlaySound {
        /// The handle to the sound to play.
        sound_source: Handle<AudioSource>,
        /// The volume to play the sound at.
        volume: f64,
    },
}
