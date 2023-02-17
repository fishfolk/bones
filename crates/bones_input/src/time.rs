//! Time functionality for the Bones framework.
//!
//! This is a slimmed down version of [`bevy_time`].
//!
//! [`bevy_time`] is licensed under MIT OR Apache-2.0.
//!
//! [`bevy_time`]: https://github.com/bevyengine/bevy/tree/aa4170d9a471c6f6a4f3bea4e41ed2c39de98e16/crates/bevy_time

use instant::{Duration, Instant};

use type_ulid::TypeUlid;

mod stopwatch;
pub use stopwatch::*;
mod timer;
pub use timer::*;

/// A clock that tracks how much it has advanced (and how much real time has elapsed) since
/// its previous update and since its creation.
#[derive(Clone, Copy, Debug, TypeUlid)]
#[ulid = "01GNR4DNDZRH0E9XCSV79WRGXH"]
pub struct Time {
    startup: Instant,
    last_update: Option<Instant>,
    first_update: Option<Instant>,

    // pausing
    paused: bool,

    delta: Duration,
    delta_seconds: f32,
    delta_seconds_f64: f64,

    elapsed: Duration,
    elapsed_seconds: f32,
    elapsed_seconds_f64: f64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            paused: false,
            first_update: None,
            last_update: None,
            delta_seconds: 0.0,
            delta: Duration::ZERO,
            elapsed_seconds: 0.0,
            delta_seconds_f64: 0.0,
            startup: Instant::now(),
            elapsed: Duration::ZERO,
            elapsed_seconds_f64: 0.0,
        }
    }
}

impl Time {
    /// Constructs a new `Time` instance with a specific startup `Instant`.
    pub fn new(startup: Instant) -> Self {
        Self {
            startup,
            ..Default::default()
        }
    }

    /// Updates the internal time measurements.
    ///
    /// Calling this method as part of your app will most likely result in inaccurate timekeeping,
    /// as the `Time` resource is ordinarily managed by the bones rendering backend.
    pub fn update(&mut self) {
        let now = Instant::now();
        self.update_with_instant(now);
    }

    /// Updates time with a specified [`Instant`].
    ///
    /// This method is provided for use in tests. Calling this method as part of your app will most
    /// likely result in inaccurate timekeeping, as the `Time` resource is ordinarily managed by
    /// whatever bones renderer you are using.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use bones_input::prelude::*;
    /// # use bones_ecs::prelude::*;
    /// # use std::time::Duration;
    /// # fn main () {
    /// #     test_health_system();
    /// # }
    /// #[derive(Resource)]
    /// struct Health {
    ///     // Health value between 0.0 and 1.0
    ///     health_value: f32,
    /// }
    ///
    /// fn health_system(time: Res<Time>, mut health: ResMut<Health>) {
    ///     // Increase health value by 0.1 per second, independent of frame rate,
    ///     // but not beyond 1.0
    ///     health.health_value = (health.health_value + 0.1 * time.delta_seconds()).min(1.0);
    /// }
    ///
    /// // Mock time in tests
    /// fn test_health_system() {
    ///     let mut world = World::default();
    ///     let mut time = Time::default();
    ///     time.update();
    ///     world.insert_resource(time);
    ///     world.insert_resource(Health { health_value: 0.2 });
    ///
    ///     let mut schedule = Schedule::new();
    ///     schedule.add_system(health_system);
    ///
    ///     // Simulate that 30 ms have passed
    ///     let mut time = world.resource_mut::<Time>();
    ///     let last_update = time.last_update().unwrap();
    ///     time.update_with_instant(last_update + Duration::from_millis(30));
    ///
    ///     // Run system
    ///     schedule.run(&mut world);
    ///
    ///     // Check that 0.003 has been added to the health value
    ///     let expected_health_value = 0.2 + 0.1 * 0.03;
    ///     let actual_health_value = world.resource::<Health>().health_value;
    ///     assert_eq!(expected_health_value, actual_health_value);
    /// }
    /// ```
    pub fn update_with_instant(&mut self, instant: Instant) {
        let raw_delta = instant - self.last_update.unwrap_or(self.startup);
        let delta = if self.paused {
            Duration::ZERO
        } else {
            // avoid rounding when at normal speed
            raw_delta
        };

        if self.last_update.is_some() {
            self.delta = delta;
            self.delta_seconds = self.delta.as_secs_f32();
            self.delta_seconds_f64 = self.delta.as_secs_f64();
        } else {
            self.first_update = Some(instant);
        }

        self.elapsed += delta;
        self.elapsed_seconds = self.elapsed.as_secs_f32();
        self.elapsed_seconds_f64 = self.elapsed.as_secs_f64();

        self.last_update = Some(instant);
    }

    /// Advance the time exactly by the given duration.
    ///
    /// This is useful when ticking the time exactly by a fixed timestep.
    pub fn advance_exact(&mut self, duration: Duration) {
        let next_instant = self.last_update.unwrap_or_else(Instant::now) + duration;
        self.update_with_instant(next_instant);
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as a [`Duration`].
    #[inline]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f32`] seconds.
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f64`] seconds.
    #[inline]
    pub fn delta_seconds_f64(&self) -> f64 {
        self.delta_seconds_f64
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`Duration`].
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`f32`] seconds.
    ///
    /// **Note:** This is a monotonically increasing value. It's precision will degrade over time.
    /// If you need an `f32` but that precision loss is unacceptable,
    /// use [`elapsed_seconds_wrapped`](#method.elapsed_seconds_wrapped).
    #[inline]
    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_seconds_f64(&self) -> f64 {
        self.elapsed_seconds_f64
    }

    /// Stops the clock, preventing it from advancing until resumed.
    ///
    /// **Note:** This does affect the `raw_*` measurements.
    #[inline]
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resumes the clock if paused.
    #[inline]
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Returns `true` if the clock is currently paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::Time;

    use std::time::{Duration, Instant};

    #[test]
    fn update_test() {
        let start_instant = Instant::now();
        let mut time = Time::new(start_instant);

        // Ensure `time` was constructed correctly.
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_seconds(), 0.0);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(time.elapsed(), Duration::ZERO);
        assert_eq!(time.elapsed_seconds(), 0.0);
        assert_eq!(time.elapsed_seconds_f64(), 0.0);

        // Update `time` and check results.
        // The first update to `time` normally happens before other systems have run,
        // so the first delta doesn't appear until the second update.
        let first_update_instant = Instant::now();
        time.update_with_instant(first_update_instant);

        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_seconds(), 0.0);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(time.elapsed(), first_update_instant - start_instant,);
        assert_eq!(
            time.elapsed_seconds(),
            (first_update_instant - start_instant).as_secs_f32(),
        );
        assert_eq!(
            time.elapsed_seconds_f64(),
            (first_update_instant - start_instant).as_secs_f64(),
        );

        // Update `time` again and check results.
        // At this point its safe to use time.delta().
        let second_update_instant = Instant::now();
        time.update_with_instant(second_update_instant);
        assert_eq!(time.delta(), second_update_instant - first_update_instant);
        assert_eq!(
            time.delta_seconds(),
            (second_update_instant - first_update_instant).as_secs_f32(),
        );
        assert_eq!(
            time.delta_seconds_f64(),
            (second_update_instant - first_update_instant).as_secs_f64(),
        );
        assert_eq!(time.elapsed(), second_update_instant - start_instant,);
        assert_eq!(
            time.elapsed_seconds(),
            (second_update_instant - start_instant).as_secs_f32(),
        );
        assert_eq!(
            time.elapsed_seconds_f64(),
            (second_update_instant - start_instant).as_secs_f64(),
        );
    }

    #[test]
    fn pause_test() {
        let start_instant = Instant::now();
        let mut time = Time::new(start_instant);

        let first_update_instant = Instant::now();
        time.update_with_instant(first_update_instant);

        assert!(!time.is_paused());

        time.pause();

        assert!(time.is_paused());

        let second_update_instant = Instant::now();
        time.update_with_instant(second_update_instant);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), first_update_instant - start_instant);

        time.unpause();

        assert!(!time.is_paused());

        let third_update_instant = Instant::now();
        time.update_with_instant(third_update_instant);
        assert_eq!(time.delta(), third_update_instant - second_update_instant);
        assert_eq!(
            time.elapsed(),
            (third_update_instant - second_update_instant) + (first_update_instant - start_instant),
        );
    }
}
