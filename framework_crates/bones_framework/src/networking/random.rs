//! Global, deterministic RNG generation resource.

use crate::prelude::*;
use crate::{
    prelude::bindings::EcsRef,
    scripting::lua::{
        bindings::SchemaLuaEcsRefMetatable,
        piccolo::{self as lua, Callback},
    },
};
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

    /// Generate a random printable ASCII character
    pub fn gen_random_ascii_char(&mut self) -> char {
        self.gen_u8_range(33, 126) as char
    }

    /// Generate a random ASCII string of the specified length
    pub fn gen_random_ascii_string(&mut self, length: u64) -> String {
        (0..length).map(|_| self.gen_random_ascii_char()).collect()
    }

    /// Generate a random u8 within the given range (inclusive)
    pub fn gen_u8_range(&mut self, start: u8, end: u8) -> u8 {
        loop {
            let n = self.gen_u64() as u8;
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random i8 within the given range (inclusive)
    pub fn gen_i8_range(&mut self, start: i8, end: i8) -> i8 {
        loop {
            let n = self.gen_i64() as i8;
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random u16 within the given range (inclusive)
    pub fn gen_u16_range(&mut self, start: u16, end: u16) -> u16 {
        loop {
            let n = self.gen_u64() as u16;
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random i16 within the given range (inclusive)
    pub fn gen_i16_range(&mut self, start: i16, end: i16) -> i16 {
        loop {
            let n = self.gen_i64() as i16;
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random u32 within the given range (inclusive)
    pub fn gen_u32_range(&mut self, start: u32, end: u32) -> u32 {
        loop {
            let n = self.gen_u64() as u32;
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random i32 within the given range (inclusive)
    pub fn gen_i32_range(&mut self, start: i32, end: i32) -> i32 {
        loop {
            let n = self.gen_i64() as i32;
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random u64 within the given range (inclusive)
    pub fn gen_u64_range(&mut self, start: u64, end: u64) -> u64 {
        loop {
            let n = self.gen_u64();
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random i64 within the given range (inclusive)
    pub fn gen_i64_range(&mut self, start: i64, end: i64) -> i64 {
        loop {
            let n = self.gen_i64();
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random usize within the given range (inclusive)
    pub fn gen_usize_range(&mut self, start: usize, end: usize) -> usize {
        loop {
            let n = self.gen_u64() as usize;
            if n >= start && n <= end {
                return n;
            }
        }
    }

    /// Generate a random isize within the given range (inclusive)
    pub fn gen_isize_range(&mut self, start: isize, end: isize) -> isize {
        loop {
            let n = self.gen_i64() as isize;
            if n >= start && n <= end {
                return n;
            }
        }
    }
}

// Implement Deref and DerefMut to allow direct access to AtomicRng methods
impl std::ops::Deref for RngGenerator {
    type Target = AtomicRng;

    fn deref(&self) -> &Self::Target {
        &self.internal_generator
    }
}

impl std::ops::DerefMut for RngGenerator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.internal_generator
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
