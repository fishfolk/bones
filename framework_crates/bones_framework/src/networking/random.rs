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
#[derive(Clone, HasSchema, Deref, DerefMut)]
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
