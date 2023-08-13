//! UI resources & components.

use crate::prelude::*;

pub use ::egui;

/// Resource containing the [`egui::Context`] that can be used to render UI.
#[derive(HasSchema, Clone, Debug, Default, Deref, DerefMut)]
#[schema(opaque)]
pub struct EguiCtx(pub egui::Context);

/// Resource that maps image handles to their associated egui textures.
#[derive(HasSchema, Clone, Debug, Default, Deref, DerefMut)]
#[schema(opaque)]
pub struct EguiTextures(pub HashMap<Handle<Image>, egui::TextureId>);
