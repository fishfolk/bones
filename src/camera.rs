//! Camera utilities.

use std::collections::VecDeque;

use crate::prelude::*;

/// Install the camera utilities on the given [`SystemStages`].
pub fn install(stages: &mut SystemStages) {
    stages
        .add_system_to_stage(CoreStage::Last, apply_shake)
        .add_system_to_stage(CoreStage::Last, apply_trauma)
        .add_system_to_stage(CoreStage::Last, decay_trauma);
}

/// Resource providing a noise source for [`CameraShake`] entities to use.
#[derive(Clone, TypeUlid, Default)]
#[ulid = "01GPPYCY6132940HAQ38J1QM70"]
pub struct ShakeNoise(noise::Perlin);

/// Component for an entity with camera shake.
#[derive(Clone, TypeUlid, Debug, Copy)]
#[ulid = "01GPPYERDFZKZS1EGV5G0XF3ME"]
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

/// Queue that can be used to send camera trauma events.
#[derive(Default, Clone, TypeUlid)]
#[ulid = "01GPREAP8HCT5JJ29CX19HT8FC"]
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
    mut trauma_events: ResMut<CameraTraumaEvents>,
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
fn decay_trauma(
    entities: Res<Entities>,
    mut camera_shakes: CompMut<CameraShake>,
    frame_time: Res<FrameTime>,
) {
    for (_ent, shake) in entities.iter_with(&mut camera_shakes) {
        shake.trauma = 0.0f32.max(shake.trauma - shake.decay_rate * frame_time.0)
    }
}
fn apply_shake(
    entities: Res<Entities>,
    mut transforms: CompMut<Transform>,
    camera_shakes: Comp<CameraShake>,
    time: Res<Time>,
    noise: Res<ShakeNoise>,
) {
    use noise::NoiseFn;
    const SHAKE_SPEED: f32 = 3.0;
    macro_rules! offset_noise {
        ($offset:expr) => {
            noise
                .0
                .get([((time.elapsed + $offset) * SHAKE_SPEED).into()]) as f32
        };
    }

    for (_ent, (shake, mut transform)) in entities.iter_with((&camera_shakes, &mut transforms)) {
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
