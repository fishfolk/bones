//! Rendering components.

use bones_lib::prelude::*;

/// Module prelude.
pub mod prelude {
    pub use super::{camera::*, color::*, line::*, sprite::*, tilemap::*, transform::*};

    #[cfg(feature = "audio")]
    pub use super::super::audio::*;

    #[cfg(feature = "ui")]
    pub use super::ui::{widgets::*, *};
}

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
        .install_plugin(sprite::sprite_plugin)
        .install_plugin(camera::plugin);

    #[cfg(feature = "ui")]
    session.install_plugin(ui::ui_plugin);
}
