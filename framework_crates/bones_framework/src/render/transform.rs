//! Transform component.

use crate::prelude::*;

/// The main transform component.
///
/// Currently we don't have a hierarchy, and this is therefore a global transform.
#[derive(Clone, Copy, Debug, HasSchema)]
#[repr(C)]
pub struct Transform {
    /// The position of the entity in the world.
    pub translation: Vec3,
    /// The rotation of the entity.
    pub rotation: Quat,
    /// The scale of the entity.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Default::default(),
            rotation: Default::default(),
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Create a transform from a translation.
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..default()
        }
    }

    /// Create a transform from a rotation.
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..default()
        }
    }

    /// Create a transform from a scale.
    pub fn from_scale(scale: Vec3) -> Self {
        Self { scale, ..default() }
    }

    /// Converts the transform to a 4x4 matrix for rendering
    pub fn to_matrix(&self) -> Mat4 {
        let rotation = Quat::from_xyzw(0., 0., self.rotation.normalize().z, 1.);

        glam::Mat4::from_scale_rotation_translation(self.scale, rotation, self.translation)
    }
}
