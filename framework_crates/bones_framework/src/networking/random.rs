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
use std::collections::VecDeque;
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

    /// Generate a random Vec2
    pub fn gen_vec2(&mut self) -> Vec2 {
        Vec2::new(self.gen_f32(), self.gen_f32())
    }

    /// Generate a random Vec2 within the given ranges for each component
    pub fn gen_vec2_range<R1: RangeBounds<f32>, R2: RangeBounds<f32>>(
        &mut self,
        x_range: R1,
        y_range: R2,
    ) -> Vec2 {
        Vec2::new(self.gen_f32_range(x_range), self.gen_f32_range(y_range))
    }

    /// Generate a random Vec3
    pub fn gen_vec3(&mut self) -> Vec3 {
        Vec3::new(self.gen_f32(), self.gen_f32(), self.gen_f32())
    }

    /// Generate a random Vec3 within the given ranges for each component
    pub fn gen_vec3_range<R1: RangeBounds<f32>, R2: RangeBounds<f32>, R3: RangeBounds<f32>>(
        &mut self,
        x_range: R1,
        y_range: R2,
        z_range: R3,
    ) -> Vec3 {
        Vec3::new(
            self.gen_f32_range(x_range),
            self.gen_f32_range(y_range),
            self.gen_f32_range(z_range),
        )
    }

    /// Shuffle a Vec in place
    pub fn shuffle_vec<T>(&mut self, vec: &mut [T]) {
        self.internal_generator.shuffle(vec);
    }

    /// Shuffle an SVec in place
    pub fn shuffle_svec<T: HasSchema>(&mut self, svec: &mut SVec<T>) {
        for i in (1..svec.len()).rev() {
            let j = self.gen_usize_range(0..=i);
            svec.swap(i, j);
        }
    }

    /// Shuffle a VecDeque in place
    pub fn shuffle_vecdeque<T>(&mut self, deque: &mut VecDeque<T>) {
        let len = deque.len();
        for i in (1..len).rev() {
            let j = self.gen_usize_range(0..=i);
            deque.swap(i, j);
        }
    }

    /// Generates a random `char` in ranges a-z and A-Z.
    pub fn gen_alphabetic(&mut self) -> char {
        self.internal_generator.alphabetic()
    }

    /// Generates a random `char` in ranges a-z, A-Z and 0-9.
    pub fn gen_alphanumeric(&mut self) -> char {
        self.internal_generator.alphanumeric()
    }

    /// Generates a random `char` in the range a-z.
    pub fn gen_lowercase(&mut self) -> char {
        self.internal_generator.lowercase()
    }

    /// Generates a random `char` in the range A-Z.
    pub fn gen_uppercase(&mut self) -> char {
        self.internal_generator.uppercase()
    }

    /// Generate a random digit in the given `radix`.
    /// Digits are represented by `char`s in ranges 0-9 and a-z.
    ///
    /// # Panics
    /// Panics if the `radix` is zero or greater than 36.
    pub fn gen_digit(&mut self, radix: u8) -> char {
        self.internal_generator.digit(radix)
    }

    /// Generates a random `char` in the given range.
    ///
    /// # Panics
    /// Panics if the range is empty.
    pub fn gen_char<R: RangeBounds<char>>(&mut self, bounds: R) -> char {
        self.internal_generator.char(bounds)
    }

    /// Generate a random printable ASCII character
    pub fn gen_random_ascii_char(&mut self) -> char {
        self.gen_u8_range(33..=126) as char
    }

    /// Generate a random ASCII string of the specified length
    pub fn gen_random_ascii_string(&mut self, length: u64) -> String {
        (0..length).map(|_| self.gen_random_ascii_char()).collect()
    }

    /// Returns a boolean, where `success_rate` represents the chance to return a true value,
    /// with 0.0 being no chance and 1.0 will always return true.
    pub fn gen_chance(&mut self, success_rate: f64) -> bool {
        let clamped_rate = success_rate.clamp(0.0, 1.0);
        self.internal_generator.chance(clamped_rate)
    }

    /// Samples a random item from a slice of values.
    pub fn gen_sample<'a, T>(&mut self, list: &'a [T]) -> Option<&'a T> {
        self.internal_generator.sample(list)
    }

    /// Samples a random item from an iterator of values.
    pub fn gen_sample_iter<T: Iterator>(&mut self, list: T) -> Option<T::Item> {
        self.internal_generator.sample_iter(list)
    }

    /// Samples a random &mut item from a slice of values.
    pub fn gen_sample_mut<'a, T>(&mut self, list: &'a mut [T]) -> Option<&'a mut T> {
        self.internal_generator.sample_mut(list)
    }

    /// Samples multiple unique items from a slice of values.
    pub fn gen_sample_multiple<'a, T>(&mut self, list: &'a [T], amount: usize) -> Vec<&'a T> {
        self.internal_generator.sample_multiple(list, amount)
    }

    /// Samples multiple unique items from a mutable slice of values.
    pub fn gen_sample_multiple_mut<'a, T>(
        &mut self,
        list: &'a mut [T],
        amount: usize,
    ) -> Vec<&'a mut T> {
        self.internal_generator.sample_multiple_mut(list, amount)
    }

    /// Samples multiple unique items from an iterator of values.
    pub fn gen_sample_multiple_iter<T: Iterator>(
        &mut self,
        list: T,
        amount: usize,
    ) -> Vec<T::Item> {
        self.internal_generator.sample_multiple_iter(list, amount)
    }

    /// Stochastic Acceptance implementation of Roulette Wheel weighted selection.
    pub fn gen_weighted_sample<'a, T, F>(
        &mut self,
        list: &'a [T],
        weight_sampler: F,
    ) -> Option<&'a T>
    where
        F: Fn((&T, usize)) -> f64,
    {
        self.internal_generator
            .weighted_sample(list, weight_sampler)
    }

    /// Stochastic Acceptance implementation of Roulette Wheel weighted selection for mutable references.
    pub fn gen_weighted_sample_mut<'a, T, F>(
        &mut self,
        list: &'a mut [T],
        weight_sampler: F,
    ) -> Option<&'a mut T>
    where
        F: Fn((&T, usize)) -> f64,
    {
        self.internal_generator
            .weighted_sample_mut(list, weight_sampler)
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
