//! Rendering components.

use bones_lib::prelude::*;

/// Module prelude.
pub mod prelude {
    pub use super::{
        camera::*, color::*, line::*, sprite::*, tilemap::*, transform::*, Renderer, RendererApi,
    };

    #[cfg(feature = "audio")]
    pub use super::audio::*;

    #[cfg(feature = "ui")]
    pub use super::ui::{widgets::*, *};
}

#[cfg(feature = "audio")]
pub mod audio;

pub mod camera;
pub mod color;
pub mod line;
pub mod sprite;
pub mod tilemap;
pub mod transform;

#[cfg(feature = "ui")]
pub mod ui;

/// Bones framework rendering plugin.
pub fn render_plugin(session: &mut Session) {
    session
        .install_plugin(ui::ui_plugin)
        .install_plugin(camera::plugin);
}

/// Trait for the interface exposed by external bones renderers.
///
/// These methods allow the game to notify the renderer when certain things happen, and to allow the
/// game to instruct the renderer to do certain things.
///
///
pub trait RendererApi: Sync + Send {
    /// Have the renderer delete the session.
    ///
    /// The default implementation doesn't do anything, and that may be appropriate for some
    /// renderers. Other renderers may need to clean up synchronized entities that are present in
    /// the deleted session.
    fn delete_session(&self, session: Session) {
        let _ = session;
    }
}

/// Resource containing the [`RendererApi`] implementation provided by the bones renderer.
#[derive(HasSchema)]
#[schema(opaque, no_clone, no_default)]
pub struct Renderer(Box<dyn RendererApi>);
