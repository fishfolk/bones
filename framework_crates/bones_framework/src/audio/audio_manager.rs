//! Audio Manager resource and systems.

use crate::prelude::*;
use kira;
use kira::{
    manager::{
        backend::{cpal::CpalBackend, mock::MockBackend, Backend},
        AudioManager as KiraAudioManager,
    },
    sound::{static_sound::StaticSoundData, SoundData},
};
use std::io::Cursor;

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
    fn load(&self, _ctx: AssetLoadCtx, bytes: &[u8]) -> BoxedFuture<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let data = StaticSoundData::from_cursor(Cursor::new(bytes))?;
            Ok(SchemaBox::new(AudioSource(data)))
        })
    }
}
