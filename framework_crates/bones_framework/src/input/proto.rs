//! Serializable data types for network messages used by the game.

// use bevy::reflect::Reflect;
use numquant::{IntRange, Quantized};

use crate::prelude::*;

/// A newtype around [`Vec2`] that implements [`From<u16>`] and [`Into<u16>`] as a way to compress
/// user stick input for use in [`crate::input::DenseInput`].
#[derive(Debug, Deref, DerefMut, Default)]
pub struct DenseMoveDirection(pub Vec2);

/// This is the specific [`Quantized`] type that we use to represent movement directions in
/// [`DenseMoveDirection`]. This encodes magnitude of direction, but sign is encoded separately.
type MoveDirQuant = Quantized<IntRange<u16, 0b11111, 0, 1>>;

impl From<u16> for DenseMoveDirection {
    fn from(bits: u16) -> Self {
        // maximum movement value representable, we use 6 bits to represent each movement direction.
        // Most significant is sign, and other 5 encode float value between 0 and
        let bit_length = 6;
        let quantized = 0b011111;
        let sign = 0b100000;
        // The first six bits represent the x movement
        let x_move_bits = bits & quantized;
        let x_move_sign = if bits & sign == 0 { 1.0 } else { -1.0 };
        // The second six bits represents the y movement
        let y_move_bits = (bits >> bit_length) & quantized;
        let y_move_sign = if (bits >> bit_length) & sign == 0 {
            1.0
        } else {
            -1.0
        };

        // Round near-zero values to zero
        let mut x = MoveDirQuant::from_raw(x_move_bits).to_f32();
        x *= x_move_sign;
        if x.abs() < 0.02 {
            x = 0.0;
        }
        let mut y = MoveDirQuant::from_raw(y_move_bits).to_f32();
        y *= y_move_sign;
        if y.abs() < 0.02 {
            y = 0.0;
        }

        DenseMoveDirection(Vec2::new(x, y))
    }
}

impl From<DenseMoveDirection> for u16 {
    fn from(dir: DenseMoveDirection) -> Self {
        let x_bits = MoveDirQuant::from_f32(dir.x.abs()).raw();
        let y_bits = MoveDirQuant::from_f32(dir.y.abs()).raw();
        let x_sign_bit = if dir.x.is_sign_positive() {
            0
        } else {
            0b100000
        };
        let y_sign_bit = if dir.y.is_sign_positive() {
            0
        } else {
            0b100000
        };

        (x_bits | x_sign_bit) | ((y_bits | y_sign_bit) << 6)
    }
}

impl From<u32> for DenseMoveDirection {
    fn from(bits: u32) -> Self {
        let bits_16 = bits as u16;
        bits_16.into()
    }
}

impl From<DenseMoveDirection> for u32 {
    fn from(dir: DenseMoveDirection) -> Self {
        let bits_16 = u16::from(dir);
        bits_16 as u32
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::proto::DenseMoveDirection;

    #[test]
    /// GGRS currently uses zero'd player input as prediction on first frame,
    /// so ensure our move direction representation is no input when built from
    /// 0 bits.
    pub fn zeroed_dense_move_dir() {
        let bits: u16 = 0;
        let dense_move_dir = DenseMoveDirection::from(bits);
        assert_eq!(dense_move_dir.0, glam::Vec2::ZERO);
    }
}
