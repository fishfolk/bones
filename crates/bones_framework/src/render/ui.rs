//! UI resources & components.

use crate::prelude::*;

pub use ::egui;

/// Resource containing the [`egui::Context`] that can be used to render UI.
#[derive(HasSchema, Clone, Debug, Default)]
#[schema(opaque)]
pub struct EguiCtx(pub egui::Context);
