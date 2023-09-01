//! Egui widgets.

mod bordered_button;
mod bordered_frame;

pub use bordered_button::*;
pub use bordered_frame::*;

use crate::prelude::*;

/// Metadata describing a border image.
///
/// A border image is a 9-patch style image that can be applied to buttons and panels.
#[derive(HasSchema, Clone, Debug)]
#[repr(C)]
pub struct BorderImageMeta {
    /// The image for the border.
    pub image: Handle<Image>,
    /// The size of the border image in pixels.
    pub image_size: UVec2,
    /// The size of the border on each side.
    pub border_size: MarginMeta,
    /// The scale to render the border image at.
    ///
    /// This is useful for pixel-art borders you want to scale up to make more visible.
    pub scale: f32,
}

impl Default for BorderImageMeta {
    fn default() -> Self {
        Self {
            image: Default::default(),
            image_size: Default::default(),
            border_size: Default::default(),
            scale: 1.0,
        }
    }
}

/// Metadata describing a themed button.
#[derive(HasSchema, Clone, Debug, Default)]
#[repr(C)]
pub struct ButtonThemeMeta {
    /// The font family, size, and color to use for the button.
    pub font: FontMeta,
    /// The amount of space to pad around the internal edges of the button.
    pub padding: MarginMeta,
    /// The border images to use for different button states.
    pub borders: ButtonBordersMeta,
}

/// The border images to use for a [`ButtonThemeMeta`] when the button is in different states.
#[derive(HasSchema, Clone, Debug, Default)]
#[repr(C)]
pub struct ButtonBordersMeta {
    /// The default button state.
    pub default: BorderImageMeta,
    /// When the button is hovered for focused on.
    pub focused: BorderImageMeta,
    /// When the button is clicked on.
    pub clicked: BorderImageMeta,
}

/// A margin specification.
#[derive(HasSchema, Default, serde::Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct MarginMeta {
    /// The top margin.
    pub top: f32,
    /// The bottom margin.
    pub bottom: f32,
    /// The left margin.
    pub left: f32,
    /// The right margin.
    pub right: f32,
}

impl From<MarginMeta> for egui::style::Margin {
    fn from(m: MarginMeta) -> Self {
        Self {
            left: m.left,
            right: m.right,
            top: m.top,
            bottom: m.bottom,
        }
    }
}
