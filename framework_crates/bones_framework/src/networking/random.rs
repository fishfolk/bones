//! Global, deterministic RNG generation resource.

use crate::prelude::*;
use crate::{
    prelude::bindings::EcsRef,
    scripting::lua::{
        bindings::SchemaLuaEcsRefMetatable,
        piccolo::{self as lua, Callback},
    },
};
use core::ops::RangeBounds;
pub use turborand::prelude::*;

/// Resource that produces deterministic pseudo-random numbers/strings.
///
/// Access in a system with [`Res<RngGenerator>`].
#[derive(Clone, HasSchema)]
#[type_data(SchemaLuaEcsRefMetatable(lua_metatable))]
pub struct RngGenerator {
    internal_generator: AtomicRng,
}

impl Default for RngGenerator {
    /// Creates a new `RngGenerator`, initializing it with a harcoded seed
    fn default() -> Self {
        Self {
            internal_generator: AtomicRng::with_seed(7),
        }
    }
}

impl RngGenerator {
    /// Creates a new `RngGenerator`, initializing it with the provided seed
    pub fn new(seed: u64) -> Self {
        Self {
            internal_generator: AtomicRng::with_seed(seed),
        }
    }

    /// Generate a random u8
    pub fn gen_u8(&mut self) -> u8 {
        self.internal_generator.gen_u8()
    }

    /// Generate a random u8 within the given range
    pub fn gen_u8_range<R: RangeBounds<u8>>(&mut self, range: R) -> u8 {
        self.internal_generator.u8(range)
    }

    /// Generate a random i8
    pub fn gen_i8(&mut self) -> i8 {
        self.internal_generator.gen_i8()
    }

    /// Generate a random i8 within the given range
    pub fn gen_i8_range<R: RangeBounds<i8>>(&mut self, range: R) -> i8 {
        self.internal_generator.i8(range)
    }

    /// Generate a random u16
    pub fn gen_u16(&mut self) -> u16 {
        self.internal_generator.gen_u16()
    }

    /// Generate a random u16 within the given range
    pub fn gen_u16_range<R: RangeBounds<u16>>(&mut self, range: R) -> u16 {
        self.internal_generator.u16(range)
    }

    /// Generate a random i16
    pub fn gen_i16(&mut self) -> i16 {
        self.internal_generator.gen_i16()
    }

    /// Generate a random i16 within the given range
    pub fn gen_i16_range<R: RangeBounds<i16>>(&mut self, range: R) -> i16 {
        self.internal_generator.i16(range)
    }

    /// Generate a random u32
    pub fn gen_u32(&mut self) -> u32 {
        self.internal_generator.gen_u32()
    }

    /// Generate a random u32 within the given range
    pub fn gen_u32_range<R: RangeBounds<u32>>(&mut self, range: R) -> u32 {
        self.internal_generator.u32(range)
    }

    /// Generate a random i32
    pub fn gen_i32(&mut self) -> i32 {
        self.internal_generator.gen_i32()
    }

    /// Generate a random i32 within the given range
    pub fn gen_i32_range<R: RangeBounds<i32>>(&mut self, range: R) -> i32 {
        self.internal_generator.i32(range)
    }

    /// Generate a random u64
    pub fn gen_u64(&mut self) -> u64 {
        self.internal_generator.gen_u64()
    }

    /// Generate a random u64 within the given range
    pub fn gen_u64_range<R: RangeBounds<u64>>(&mut self, range: R) -> u64 {
        self.internal_generator.u64(range)
    }

    /// Generate a random i64
    pub fn gen_i64(&mut self) -> i64 {
        self.internal_generator.gen_i64()
    }

    /// Generate a random i64 within the given range
    pub fn gen_i64_range<R: RangeBounds<i64>>(&mut self, range: R) -> i64 {
        self.internal_generator.i64(range)
    }

    /// Generate a random usize
    pub fn gen_usize(&mut self) -> usize {
        self.internal_generator.gen_usize()
    }

    /// Generate a random usize within the given range
    pub fn gen_usize_range<R: RangeBounds<usize>>(&mut self, range: R) -> usize {
        self.internal_generator.usize(range)
    }

    /// Generate a random isize
    pub fn gen_isize(&mut self) -> isize {
        self.internal_generator.gen_isize()
    }

    /// Generate a random isize within the given range
    pub fn gen_isize_range<R: RangeBounds<isize>>(&mut self, range: R) -> isize {
        self.internal_generator.isize(range)
    }

    /// Generate a random f32
    pub fn gen_f32(&mut self) -> f32 {
        self.internal_generator.f32()
    }

    /// Generate a random f32 within the given range
    pub fn gen_f32_range<R: RangeBounds<f32>>(&mut self, range: R) -> f32 {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&n) => n,
            std::ops::Bound::Excluded(&n) => n,
            std::ops::Bound::Unbounded => 0.0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&n) => n,
            std::ops::Bound::Excluded(&n) => n,
            std::ops::Bound::Unbounded => 1.0,
        };
        loop {
            let n = self.gen_f32();
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random f64
    pub fn gen_f64(&mut self) -> f64 {
        self.internal_generator.f64()
    }

    /// Generate a random f64 within the given range
    pub fn gen_f64_range<R: RangeBounds<f64>>(&mut self, range: R) -> f64 {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&n) => n,
            std::ops::Bound::Excluded(&n) => n,
            std::ops::Bound::Unbounded => 0.0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&n) => n,
            std::ops::Bound::Excluded(&n) => n,
            std::ops::Bound::Unbounded => 1.0,
        };
        loop {
            let n = self.gen_f64();
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random bool
    pub fn gen_bool(&mut self) -> bool {
        self.internal_generator.bool()
    }

    /// Generate a random printable ASCII character
    pub fn gen_random_ascii_char(&mut self) -> char {
        self.gen_u8_range(33..=126) as char
    }

    /// Generate a random ASCII string of the specified length
    pub fn gen_random_ascii_string(&mut self, length: u64) -> String {
        (0..length).map(|_| self.gen_random_ascii_char()).collect()
    }
}

fn lua_metatable(ctx: lua::Context) -> lua::Table {
    let metatable = lua::Table::new(&ctx);

    let f32_fn = ctx.registry().stash(
        &ctx,
        Callback::from_fn(&ctx, |ctx, _fuel, mut stack| {
            let this: &EcsRef = stack.consume(ctx)?;
            let mut b = this.borrow_mut();
            let rng_generator = b.schema_ref_mut()?.cast_into_mut::<RngGenerator>();
            let n = rng_generator.internal_generator.f32();
            stack.replace(ctx, n);
            Ok(lua::CallbackReturn::Return)
        }),
    );
    metatable
        .set(
            ctx,
            "__index",
            Callback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let (_this, key): (lua::Value, lua::String) = stack.consume(ctx)?;

                #[allow(clippy::single_match)]
                match key.as_bytes() {
                    b"f32" => {
                        stack.push_front(ctx.registry().fetch(&f32_fn).into());
                    }
                    _ => (),
                }
                Ok(lua::CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
