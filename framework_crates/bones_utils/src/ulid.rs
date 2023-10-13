use crate::prelude::*;

pub use ulid::Ulid;

/// Extension trait for [`Ulid`].
pub trait UlidExt {
    /// Constructor that) is the same as [`Ulid::new()`], but that works on WASM, too using the
    /// [`instant`] crate.
    fn create() -> Self;
}

impl UlidExt for Ulid {
    fn create() -> Self {
        Ulid::from_parts(
            instant::now().floor() as u64,
            THREAD_RNG.with(|rng| rng.gen_u128()),
        )
    }
}
