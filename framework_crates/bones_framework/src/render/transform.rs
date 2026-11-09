//! Transform component.

use crate::prelude::*;
use std::f32::consts::PI;

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

    /// Create a transform from a transformation matrix.
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();

        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Converts the transform to a 4x4 matrix for rendering
    pub fn to_matrix(&self, translation_scale: Vec3) -> Mat4 {
        let angle = self.rotation.z.rem_euclid(2.0 * PI);
        let rotation = Mat4::from_rotation_z(angle);

        let scale_rotation = rotation * Mat4::from_scale(self.scale);
        Mat4::from_translation(self.translation * translation_scale) * scale_rotation
    }

    /// Converts to a matrix without translation scale
    pub fn to_matrix_none(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Converts the transform to a 4x4 matrix for rendering,
    /// scaling off from the given pivot (for example, the center of the screen).
    pub fn to_matrix_with_pivot(&self, pivot: Vec3) -> Mat4 {
        let angle = self.rotation.z.rem_euclid(2.0 * PI);
        let rotation = Mat4::from_rotation_z(angle);

        let scale_offset = Mat4::from_translation(pivot)
            * Mat4::from_scale(self.scale)
            * Mat4::from_translation(-pivot);

        Mat4::from_translation(self.translation) * rotation * scale_offset
    }

    /// Converts the transform to a 4x4 matrix for rendering,
    /// using a separate pivot for scaling and for rotation.
    pub fn to_matrix_with_pivots(&self, pivot_scale: Vec3, pivot_rot: Vec3) -> Mat4 {
        let scale_transform = Mat4::from_translation(pivot_scale)
            * Mat4::from_scale(self.scale)
            * Mat4::from_translation(-pivot_scale);

        let angle = self.rotation.z.rem_euclid(2.0 * std::f32::consts::PI);
        let rotation = Mat4::from_rotation_z(angle);

        let rotation_transform =
            Mat4::from_translation(pivot_rot) * rotation * Mat4::from_translation(-pivot_rot);

        Mat4::from_translation(self.translation) * rotation_transform * scale_transform
    }
}
