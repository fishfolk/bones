//! Standardized rendering components for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

pub mod audio;
pub mod camera;
pub mod color;
pub mod datatypes;
pub mod line;
pub mod sprite;
pub mod tilemap;
pub mod transform;

/// The prelude
pub mod prelude {
    pub use {bones_asset::prelude::*, bones_ecs::prelude::*, glam::*, type_ulid::TypeUlid};

    pub use crate::{
        audio::*, camera::*, color::*, datatypes::*, key, line::*, sprite::*, tilemap::*,
        transform::*,
    };
}

/// Create a new const [`Key`][datatypes] parsed at compile time.
#[macro_export]
macro_rules! key {
    ($s:literal) => {{
        const KEY: Key = match Key::new($s) {
            Ok(key) => key,
            Err(KeyError::TooLong) => panic!("Key too long"),
            Err(KeyError::NotAscii) => panic!("Key not ascii"),
        };
        KEY
    }};
}

#[cfg(feature = "bevy")]
mod bevy {
    use bones_bevy_utils::IntoBevy;

    impl IntoBevy<bevy_transform::components::Transform> for super::transform::Transform {
        fn into_bevy(self) -> bevy_transform::components::Transform {
            bevy_transform::components::Transform {
                translation: self.translation,
                rotation: self.rotation,
                scale: self.scale,
            }
        }
    }

    impl IntoBevy<bevy_render::color::Color> for super::color::Color {
        fn into_bevy(self) -> bevy_render::color::Color {
            bevy_render::color::Color::from(self.as_rgba_f32())
        }
    }
}
