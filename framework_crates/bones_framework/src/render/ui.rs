//! UI resources & components.

use crate::prelude::*;

pub use ::egui;

/// Resource containing the [`egui::Context`] that can be used to render UI.
#[derive(HasSchema, Clone, Debug, Default, Deref, DerefMut)]
pub struct EguiCtx(pub egui::Context);

/// Resource that maps image handles to their associated egui textures.
#[derive(HasSchema, Clone, Debug, Default, Deref, DerefMut)]
pub struct EguiTextures(pub HashMap<Handle<Image>, egui::TextureId>);

impl EguiTextures {
    /// Get the [`egui::TextureId`] for the given bones [`Handle<Image>`].
    #[track_caller]
    pub fn get(&self, handle: Handle<Image>) -> egui::TextureId {
        *self.0.get(&handle).unwrap()
    }
}

/// Resource for configuring egui rendering.
#[derive(HasSchema, Clone, Debug)]
#[repr(C)]
pub struct EguiSettings {
    /// Custom scale for the UI.
    pub scale: f64,
}

impl Default for EguiSettings {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}
