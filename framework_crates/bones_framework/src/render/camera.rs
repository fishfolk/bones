//! Camera components.

use std::collections::VecDeque;

use crate::prelude::*;

/// Makes an entity behave like a camera.
///
/// The entity must also have a [`Transform`] component for the camera to render anything.
#[derive(Clone, Copy, Debug, HasSchema)]
// TODO: make repr(C) when `Option`s are supported.
// We don't have `Option` support in `bones_schema` right now.
// Once we do, we can make this type `#[repr(C)]` instead of `#[schema(opaque)]`.
pub struct Camera {
    /// The height of the camera in in-game pixels.
    ///
    /// The width of the camera will be determined from the window aspect ratio.
    // TODO: implement different scaling modes for bones cameras.
    pub height: f32,
    /// Whether or not the camera is enabled and rendering.
    pub active: bool,
    /// An optional viewport override, allowing you to specify that the camera should render to only
    /// a portion of the window.
    ///
    /// This can be used, for example, for split screen functionality.
    pub viewport: Option<Viewport>,
    /// Cameras with a higher priority will be rendered on top of cameras with a lower priority.
    pub priority: i32,
}

/// A custom viewport specification for a [`Camera`].
#[derive(Clone, Copy, Debug, HasSchema, Default)]
#[repr(C)]
pub struct Viewport {
    /// The physical position to render this viewport to within the RenderTarget of this Camera.
    /// (0,0) corresponds to the top-left corner.
    pub position: UVec2,
    /// The physical size of the viewport rectangle to render to within the RenderTarget of this
    /// Camera. The origin of the rectangle is in the top-left corner.
    pub size: UVec2,
    /// The minimum depth to render (on a scale from 0.0 to 1.0).
    pub depth_min: f32,
    /// The maximum depth to render (on a scale from 0.0 to 1.0).
    pub depth_max: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            height: 400.0,
            active: true,
            viewport: None,
            priority: 0,
        }
    }
}

/// Resource for controlling the clear color.
#[derive(Deref, DerefMut, Clone, Copy, HasSchema, Default)]
pub struct ClearColor(pub Color);

/// Utility function that spawns the camera in a default position.
///
/// Camera will be spawned such that it is positioned at `0` on X and Y axis and at `1000` on the Z
/// axis, allowing it to see sprites with a Z position from `0` to `1000` non-inclusive.
pub fn spawn_default_camera(
    entities: &mut Entities,
    transforms: &mut CompMut<Transform>,
    cameras: &mut CompMut<Camera>,
) -> Entity {
    let ent = entities.create();
    cameras.insert(ent, default());
    transforms.insert(ent, Transform::from_translation(Vec3::new(0., 0., 1000.)));
    ent
}

/// Install the camera utilities on the given [`SystemStages`].
pub fn plugin(session: &mut Session) {
    session.stages
        .add_system_to_stage(CoreStage::Last, apply_shake)
        .add_system_to_stage(CoreStage::Last, apply_trauma)
        .add_system_to_stage(CoreStage::Last, decay_trauma);
}

/// Resource providing a noise source for [`CameraShake`] entities to use.
#[derive(Clone, HasSchema)]
pub struct ShakeNoise(pub noise::permutationtable::PermutationTable);

impl Default for ShakeNoise {
    fn default() -> Self {
        Self(noise::permutationtable::PermutationTable::new(0))
    }
}

/// Component for an entity with camera shake.
#[derive(Clone, HasSchema, Debug, Copy)]
#[repr(C)]
pub struct CameraShake {
    /// Value from 0-1 that indicates the intensity of the shake. Should usually be set with
    /// `CameraShake::add_trauma` and not manually decayed.
    pub trauma: f32,
    /// The maximum offset angle in radians that the camera shake can cause.
    pub max_angle_rad: f32,
    /// The maximum offset position that the camera shake can cause.
    pub max_offset: Vec2,
    /// The the length of time in seconds for the camera trauma to decay from 1 to 0.
    pub decay_rate: f32,
    /// The speed that the screen is shook.
    pub speed: f32,
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
            speed: 1.5,
            center: Vec3::ZERO,
        }
    }
}

impl CameraShake {
    /// Create a new [`CameraShake`] component with the provided maximum offset angle (in degrees)
    /// and position as well as the trauma decay rate in seconds.
    pub fn new(max_angle_deg: f32, max_offset: Vec2, speed: f32, decay_rate: f32) -> Self {
        Self {
            max_angle_rad: max_angle_deg.to_radians(),
            max_offset,
            decay_rate,
            speed,
            ..default()
        }
    }

    /// Create a new [`CameraShake`] component with the provided maximum offset angle (in degrees)
    /// and position and its initial trauma set to some value (clamped between 0 and 1).
    pub fn with_trauma(
        trauma: f32,
        max_angle_deg: f32,
        max_offset: Vec2,
        speed: f32,
        decay_rate: f32,
    ) -> Self {
        let mut shake = Self::new(max_angle_deg, max_offset, speed, decay_rate);
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

/// Queue that can be used to send camera trauma events.
#[derive(Default, Clone, HasSchema)]
pub struct CameraTraumaEvents {
    /// The event queue.
    pub queue: VecDeque<f32>,
}

impl CameraTraumaEvents {
    /// Send a camera trauma event.
    pub fn send(&mut self, trauma: f32) {
        self.queue.push_back(trauma);
    }
}

fn apply_trauma(
    entities: Res<Entities>,
    mut camera_shakes: CompMut<CameraShake>,
    mut trauma_events: ResMutInit<CameraTraumaEvents>,
) {
    for (_ent, camera_shake) in entities.iter_with(&mut camera_shakes) {
        camera_shake.add_trauma(
            trauma_events
                .queue
                .iter()
                .fold(0.0, |acc, trauma| acc + trauma),
        );
    }
    trauma_events.queue.clear();
}
fn decay_trauma(entities: Res<Entities>, mut camera_shakes: CompMut<CameraShake>, time: Res<Time>) {
    for (_ent, shake) in entities.iter_with(&mut camera_shakes) {
        shake.trauma = 0.0f32.max(shake.trauma - shake.decay_rate * time.delta_seconds())
    }
}
fn apply_shake(
    entities: Res<Entities>,
    mut transforms: CompMut<Transform>,
    camera_shakes: Comp<CameraShake>,
    time: Res<Time>,
    noise: ResInit<ShakeNoise>,
) {
    macro_rules! offset_noise {
        ($offset:expr, $shake_speed:expr) => {
            perlin_noise::perlin_1d(
                ((time.elapsed_seconds() + $offset) * $shake_speed * 0.001).into(),
                &noise.0,
            ) as f32
        };
    }

    for (_ent, (shake, transform)) in entities.iter_with((&camera_shakes, &mut transforms)) {
        (transform.rotation, transform.translation) = if shake.trauma > 0.0 {
            let sqr_trauma = shake.trauma * shake.trauma;

            let rotation = Quat::from_axis_angle(
                Vec3::Z,
                sqr_trauma * offset_noise!(0.0, shake.speed) * shake.max_angle_rad,
            );

            let x_offset = sqr_trauma * offset_noise!(1.0, shake.speed) * shake.max_offset.x;
            let y_offset = sqr_trauma * offset_noise!(2.0, shake.speed) * shake.max_offset.y;

            (rotation, shake.center + Vec3::new(x_offset, y_offset, 0.0))
        } else {
            // In future we may need to provide a rotation field on `CameraShake` should we need to
            // rotate the camera in another context.
            (Quat::IDENTITY, shake.center)
        }
    }
}

/// This module is copied from code from this commit:
/// <https://github.com/Razaekel/noise-rs/commit/1a2b5e0880656e8d2ae1025df576d70180d7592a>.
///
/// We temporarily vendor the code here because the 1D perlin noise hasn't been released yet:
/// <https://github.com/Razaekel/noise-rs/issues/306>
///
/// From the repo:
///
/// > Licensed under either of
/// >
/// > Apache License, Version 2.0 (LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
/// > MIT license (LICENSE-MIT or <http://opensource.org/licenses/MIT>)
/// > at your option.
mod perlin_noise {
    #[inline(always)]
    pub fn perlin_1d<NH>(point: f64, hasher: &NH) -> f64
    where
        NH: noise::permutationtable::NoiseHasher + ?Sized,
    {
        // Unscaled range of linearly interpolated perlin noise should be (-sqrt(N)/2, sqrt(N)/2).
        // Need to invert this value and multiply the unscaled result by the value to get a scaled
        // range of (-1, 1).
        //
        // 1/(sqrt(N)/2), N=1 -> 1/2
        const SCALE_FACTOR: f64 = 0.5;

        #[inline(always)]
    #[rustfmt::skip]
    fn gradient_dot_v(perm: usize, point: f64) -> f64 {
        let x = point;

        match perm & 0b1 {
            0 =>  x, // ( 1 )
            1 => -x, // (-1 )
            _ => unreachable!(),
        }
    }

        let floored = point.floor();
        let corner = floored as isize;
        let distance = point - floored;

        macro_rules! call_gradient(
        ($x_offset:expr) => {
            {
                gradient_dot_v(
                    hasher.hash(&[corner + $x_offset]),
                    distance - $x_offset as f64
                )
            }
        }
    );

        let g0 = call_gradient!(0);
        let g1 = call_gradient!(1);

        let u = map_quintic(distance);

        let unscaled_result = linear_interpolation(u, g0, g1);

        let scaled_result = unscaled_result * SCALE_FACTOR;

        // At this point, we should be really damn close to the (-1, 1) range, but some float errors
        // could have accumulated, so let's just clamp the results to (-1, 1) to cut off any
        // outliers and return it.
        scaled_result.clamp(-1.0, 1.0)
    }
    #[inline(always)]
    fn linear_interpolation(u: f64, g0: f64, g1: f64) -> f64 {
        let k0 = g0;
        let k1 = g1 - g0;
        k0 + k1 * u
    }
    fn map_quintic(n: f64) -> f64 {
        let x = n.clamp(0.0, 1.0);

        x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
    }
}
