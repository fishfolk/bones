//! Input resources.

pub mod time;
pub mod window;

/// Module prelude.
pub mod prelude {
    pub use super::{time::*, window::*};
}
