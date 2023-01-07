//! This crate provides 2D camera shake using the methodology described in this excellent [GDC
//! talk](https://www.youtube.com/watch?v=tu-Qe66AvtY) by Squirrel Eiserloh.
//!
//! # Example
//! ```
//! # use bevy_core_pipeline::prelude::*;
//! use bevy::prelude::*;
//! use bones_camera_shake::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugin(CameraShakePlugin)
//!         .add_startup_system(spawn_camera);
//! }
//!
//! fn spawn_camera(mut commands: Commands) {
//!     commands.spawn((
//!     commands.spawn_bundle(
//!         Camera2dBundle::default(),
//!     )
//!     .insert(CameraShake::new(90.0, Vec2::splat(100.0), 0.5));
//! }
//!
//! fn add_camera_shake(mut ev_trauma: EventWriter<CameraTrauma>) {
//!     /* some traumatic event... */
//!     ev_trauma.send(CameraTrauma(0.5));
//! }
use bevy::prelude::*;

use noise::{NoiseFn, Perlin};

/// Add this plugin to your app to enable camera shake.
pub struct CameraShakePlugin;

impl Plugin for CameraShakePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CameraTrauma>()
            .add_system(apply_shake)
            .add_system(apply_trauma)
            .add_system(decay_trauma)
            .insert_resource(ShakeNoise(Perlin::default()));
    }
}

/// Component for an entity with camera shake.
#[derive(Component)]
pub struct CameraShake {
    /// Value from 0-1 that indicates the intensity of the shake. Should be set with
    /// `CameraShake::add_trauma` and not manually decayed.
    trauma: f32,
    /// The maximum offset angle in radians that the camera shake can cause.
    pub max_angle_rad: f32,
    /// The maximum offset position that the camera shake can cause.
    pub max_offset: Vec2,
    /// The the length of time in seconds for the camera trauma to decay from 1 to 0.
    pub decay_rate: f32,
    /// The camera will always restore to this position.
    pub center: Vec3,
}

impl Default for CameraShake {
    fn default() -> Self {
        Self {
            trauma: 0.0,
            max_angle_rad: 90.0,
            max_offset: Vec2::splat(100.0),
            decay_rate: 0.5,
            center: Vec3::ZERO,
        }
    }
}

impl CameraShake {
    /// Create a new [`CameraShake`] component with the provided maximum offset angle (in degrees)
    /// and position as well as the trauma decay rate in seconds.
    pub fn new(max_angle_deg: f32, max_offset: Vec2, decay_rate: f32) -> Self {
        Self {
            max_angle_rad: max_angle_deg * (std::f32::consts::PI / 180.0),
            max_offset,
            decay_rate,
            ..default()
        }
    }

    /// Create a new [`CameraShake`] component with the provided maximum offset angle (in degrees)
    /// and position and its initial trauma set to some value (clamped between 0 and 1).
    pub fn with_trauma(trauma: f32, max_angle_deg: f32, max_offset: Vec2, decay_rate: f32) -> Self {
        let mut shake = Self::new(max_angle_deg, max_offset, decay_rate);
        shake.trauma = trauma.min(1.0).max(0.0);
        shake
    }

    /// Adds trauma to the camera, capping it at 1.0
    pub fn add_trauma(&mut self, value: f32) {
        self.trauma += value;
        if 1.0 < self.trauma {
            self.trauma = 1.0;
        }
    }
}

/// Event to add trauma to the camera. Provide a value between 0 and 1 for the trauma amount.
///
/// ```
/// # use bevy::prelude::*;
/// # use bones_camera_shake::*;
/// fn my_system(mut ev_trauma: EventWriter<CameraTrauma>) {
///     /* some traumatic event... */
///     ev_trauma.send(CameraTrauma(0.5));
/// }
pub struct CameraTrauma(pub f32);

/// System to apply the trauma sent by the [`CameraTrauma`] event to all the [`CameraShake`]
/// components.
fn apply_trauma(mut cameras: Query<&mut CameraShake>, mut ev_trauma: EventReader<CameraTrauma>) {
    cameras
        .iter_mut()
        .for_each(|mut c| c.add_trauma(ev_trauma.iter().fold(0.0, |acc, trauma| acc + trauma.0)));
}

/// System to decay the trauma linearly over time.
fn decay_trauma(mut q: Query<&mut CameraShake>, time: Res<Time>) {
    for mut shake in q.iter_mut() {
        shake.trauma = 0.0f32.max(shake.trauma - shake.decay_rate * time.delta_seconds());
    }
}

/// Resource that provides a source of noise for [`CameraShake`] entities to use.
struct ShakeNoise(Perlin);

/// System to apply camera shake based on the current trauma.
fn apply_shake(
    mut q: Query<(&CameraShake, &mut Transform)>,
    time: Res<Time>,
    noise: Res<ShakeNoise>,
) {
    const SHAKE_SPEED: f32 = 3.0;
    macro_rules! offset_noise {
        ($offset:expr) => {
            noise
                .0
                .get([((time.seconds_since_startup() as f32 + $offset) * SHAKE_SPEED).into()]) as f32
        };
    }

    for (shake, mut transform) in q.iter_mut() {
        (transform.rotation, transform.translation) = if shake.trauma > 0.0 {
            let sqr_trauma = shake.trauma * shake.trauma;

            let rotation = Quat::from_axis_angle(
                Vec3::Z,
                sqr_trauma * offset_noise!(0.0) * shake.max_angle_rad,
            );

            let x_offset = sqr_trauma * offset_noise!(100.0) * shake.max_offset.x;
            let y_offset = sqr_trauma * offset_noise!(200.0) * shake.max_offset.y;

            (rotation, shake.center + Vec3::new(x_offset, y_offset, 0.0))
        } else {
            // In future we may need to provide a rotation field on `CameraShake` should we need to
            // rotate the camera in another context.
            (Quat::IDENTITY, shake.center)
        }
    }
}