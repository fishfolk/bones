use turborand::rng::Rng;

pub use turborand::{GenCore, TurboRand};

std::thread_local! {
    /// A fast, non-cryptographic, thread-local random number generator powered by turborand.
    pub static THREAD_RNG: Rng = Rng::new();
}
