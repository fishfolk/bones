//! UI resources & components.

use std::sync::Arc;

use crate::prelude::*;

pub use ::egui;
use serde::Deserialize;

pub mod widgets;

/// The Bones Framework UI plugin.
pub fn ui_plugin(_session: &mut Session) {
    // TODO: remove this plugin if it remains unused.
}

/// Resource containing the [`egui::Context`] that can be used to render UI.
#[derive(HasSchema, Clone, Debug, Default, Deref, DerefMut)]
pub struct EguiCtx(pub egui::Context);

/// [Shared resource](Game::insert_shared_resource) that, if inserted, allows you to modify the raw
/// egui input based on the state of the last game frame.
///
/// This can be useful, for example, for generating arrow-key and Enter key presses from gamepad
/// inputs.
#[derive(HasSchema, Clone, Deref, DerefMut)]
#[schema(no_default)]
pub struct EguiInputHook(pub Arc<dyn Fn(&mut Game, &mut egui::RawInput) + Sync + Send + 'static>);

impl EguiInputHook {
    /// Create a new egui input hook.
    pub fn new<F: Fn(&mut Game, &mut egui::RawInput) + Sync + Send + 'static>(hook: F) -> Self {
        Self(Arc::new(hook))
    }
}

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

/// A font asset.
#[derive(HasSchema, Clone)]
#[schema(no_default)]
#[type_data(asset_loader(["ttf", "otf"], FontLoader))]
pub struct Font {
    /// The name of the loaded font family.
    pub family_name: Arc<str>,
    /// The egui font data.
    pub data: egui::FontData,
    /// Whether or not this is a monospace font.
    pub monospace: bool,
}

/// Font metadata for buttons, headings, etc, describing the font, size, and color of text to be
/// rendered.
#[derive(HasSchema, Debug, serde::Deserialize, Clone)]
#[derive_type_data(SchemaDeserialize)]
pub struct FontMeta {
    /// The font-family to use.
    #[serde(deserialize_with = "deserialize_arc_str")]
    pub family: Arc<str>,
    /// The font size.
    pub size: f32,
    /// The font color.
    pub color: Color,
}

impl Default for FontMeta {
    fn default() -> Self {
        Self {
            family: "".into(),
            size: Default::default(),
            color: Default::default(),
        }
    }
}

impl From<FontMeta> for egui::FontSelection {
    fn from(val: FontMeta) -> Self {
        egui::FontSelection::FontId(val.id())
    }
}

impl FontMeta {
    /// Get the Egui font ID.
    pub fn id(&self) -> egui::FontId {
        egui::FontId::new(self.size, egui::FontFamily::Name(self.family.clone()))
    }

    /// Create an [`egui::RichText`] that can be passed to [`ui.label()`][egui::Ui::label].
    pub fn rich(&self, t: impl Into<String>) -> egui::RichText {
        egui::RichText::new(t).color(self.color).font(self.id())
    }

    /// Clone the font and set a new color.
    pub fn with_color(&self, color: Color) -> Self {
        Self {
            family: self.family.clone(),
            size: self.size,
            color,
        }
    }
}

fn deserialize_arc_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Arc<str>, D::Error> {
    String::deserialize(d).map(|x| x.into())
}

/// The [`Font`] asset loader.
pub struct FontLoader;
impl AssetLoader for FontLoader {
    fn load(&self, _ctx: AssetLoadCtx, bytes: &[u8]) -> BoxedFuture<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let (family_name, monospace) = {
                let face = ttf_parser::Face::parse(&bytes, 0)?;
                (
                    face.names()
                        .into_iter()
                        .filter(|x| x.name_id == ttf_parser::name_id::FAMILY)
                        .find_map(|x| x.to_string())
                        .ok_or_else(|| {
                            anyhow::format_err!("Could not read font family from font file")
                        })?
                        .into(),
                    face.is_monospaced(),
                )
            };
            let data = egui::FontData::from_owned(bytes.to_vec());

            Ok(SchemaBox::new(Font {
                family_name,
                data,
                monospace,
            }))
        })
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

/// Extension trait with helpers for the egui context
pub trait EguiContextExt {
    /// Clear the UI focus
    fn clear_focus(self);

    /// Get a global runtime state from the EGUI context, returning the default value if it is not
    /// present.
    ///
    /// This is just a convenience wrapper around Egui's built in temporary data store.
    ///
    /// The value will be cloned to get it out of the store without holding a lock.
    fn get_state<T: Clone + Default + Sync + Send + 'static>(self) -> T;

    /// Set a global runtime state from the EGUI context.
    ///
    /// This is just a convenience wrapper around Egui's built in temporary data store.
    fn set_state<T: Clone + Default + Sync + Send + 'static>(self, value: T);
}

impl EguiContextExt for &egui::Context {
    fn clear_focus(self) {
        self.memory_mut(|r| r.request_focus(egui::Id::null()));
    }
    fn get_state<T: Clone + Default + Sync + Send + 'static>(self) -> T {
        self.data_mut(|data| data.get_temp_mut_or_default::<T>(egui::Id::null()).clone())
    }
    fn set_state<T: Clone + Default + Sync + Send + 'static>(self, value: T) {
        self.data_mut(|data| *data.get_temp_mut_or_default::<T>(egui::Id::null()) = value);
    }
}

/// Extension trait with helpers for egui responses
pub trait EguiResponseExt {
    /// Set this response to focused if nothing else is focused
    fn focus_by_default(self, ui: &mut egui::Ui) -> egui::Response;
}

impl EguiResponseExt for egui::Response {
    fn focus_by_default(self, ui: &mut egui::Ui) -> egui::Response {
        if ui.ctx().memory(|r| r.focus().is_none()) {
            ui.ctx().memory_mut(|r| r.request_focus(self.id));

            self
        } else {
            self
        }
    }
}
