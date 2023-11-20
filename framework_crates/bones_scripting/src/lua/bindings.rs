use bones_lib::prelude::*;
use gc_arena_derive::Collect;
use piccolo::{
    self as lua, meta_ops, meta_ops::MetaResult, AnyCallback, AnySequence, CallbackReturn, Context,
    Error, Sequence, SequencePoll, Stack,
};

use super::*;

pub mod assets;
pub mod components;
pub mod entities;
pub mod resources;
pub mod schema;
pub mod world;

pub mod ecsref;
pub use ecsref::*;

/// Registers lua binding typedatas for bones_framework types.
pub fn register_lua_typedata() {
    Entities::schema()
        .type_data
        .insert(SchemaLuaEcsRefMetatable(entities::entities_metatable))
        .unwrap();
}

pub fn no_newindex(ctx: Context) -> AnyCallback {
    AnyCallback::from_fn(&ctx, |_ctx, _fuel, _stack| {
        Err(anyhow::format_err!("Creating fields not allowed on this type").into())
    })
}

/// Generate the environment table for executing scripts under.
pub fn env(ctx: Context) -> Table {
    let env = Table::new(&ctx);

    env.set(ctx, "math", ctx.state.globals.get(ctx, "math"))
        .unwrap();

    let schema_fn = AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
        let singletons = ctx.singletons();
        let schema_metatable = singletons.get(ctx, schema::metatable);

        let schema_name = stack.pop_front();
        let Value::String(schema_name) = schema_name else {
            return Err(anyhow::format_err!("Type error: expected string schema name").into());
        };
        let mut matches = SCHEMA_REGISTRY.schemas.iter().filter(|schema| {
            schema.name.as_bytes() == schema_name.as_bytes()
                || schema.full_name.as_bytes() == schema_name.as_bytes()
        });

        if let Some(next_match) = matches.next() {
            if matches.next().is_some() {
                return Err(anyhow::format_err!("Found multiple schemas matching name.").into());
            }

            // TODO: setup `toString` implementation so that printing schemas gives more information.
            let schema = AnyUserData::new_static(&ctx, next_match);
            schema.set_metatable(&ctx, Some(schema_metatable));
            stack.push_front(schema.into());
        } else {
            return Err(anyhow::format_err!("Schema not found: {schema_name}").into());
        }

        Ok(CallbackReturn::Return)
    });
    env.set(ctx, "schema", schema_fn).unwrap();
    env.set(ctx, "s", schema_fn).unwrap(); // Alias for schema

    WorldRef::default().add_to_env(ctx, env);

    // Set the `CoreStage` enum global
    let core_stage_table = Table::new(&ctx);
    for (name, stage) in [
        ("First", CoreStage::First),
        ("PreUpdate", CoreStage::PreUpdate),
        ("Update", CoreStage::Update),
        ("PostUpdate", CoreStage::PostUpdate),
        ("Last", CoreStage::Last),
    ] {
        core_stage_table
            .set(ctx, name, AnyUserData::new_static(&ctx, stage))
            .unwrap();
    }
    env.set(ctx, "CoreStage", core_stage_table).unwrap();

    macro_rules! add_log_fn {
        ($level:ident) => {
            let $level = AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
                #[derive(Debug, Copy, Clone, Eq, PartialEq, Collect)]
                #[collect(require_static)]
                enum Mode {
                    Init,
                    First,
                }

                #[derive(Collect)]
                #[collect(no_drop)]
                struct PrintSeq<'gc> {
                    mode: Mode,
                    values: Vec<Value<'gc>>,
                }

                impl<'gc> Sequence<'gc> for PrintSeq<'gc> {
                    fn poll<'a>(
                        &mut self,
                        ctx: Context<'gc>,
                        _fuel: &mut Fuel,
                        mut stack: Stack<'gc, 'a>,
                    ) -> Result<SequencePoll<'gc>, Error<'gc>> {
                        if self.mode == Mode::Init {
                            self.mode = Mode::First;
                        } else {
                            self.values.push(stack.get(0));
                        }
                        stack.clear();

                        while let Some(value) = self.values.pop() {
                            match meta_ops::tostring(ctx, value)? {
                                MetaResult::Value(v) => tracing::$level!("{v}"),
                                MetaResult::Call(call) => {
                                    stack.extend(call.args);
                                    return Ok(SequencePoll::Call {
                                        function: call.function,
                                        is_tail: false,
                                    });
                                }
                            }
                        }

                        Ok(SequencePoll::Return)
                    }
                }

                Ok(CallbackReturn::Sequence(AnySequence::new(
                    &ctx,
                    PrintSeq {
                        mode: Mode::Init,
                        values: stack.drain(..).rev().collect(),
                    },
                )))
            });
            env.set(ctx, stringify!($level), $level).unwrap();
        };
    }

    // Register logging callbacks
    add_log_fn!(trace);
    add_log_fn!(debug);
    add_log_fn!(info);
    add_log_fn!(warn);
    add_log_fn!(error);
    env.set(ctx, "print", info).unwrap();

    // Prevent creating new items in the global scope, by overrideing the __newindex metamethod
    // on the _ENV metatable.
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__newindex",
            AnyCallback::from_fn(&ctx, |_ctx, _fuel, _stack| {
                Err(anyhow::format_err!(
                    "Cannot set global variables, you must use `world` \
                        to persist any state across frames."
                )
                .into())
            }),
        )
        .unwrap();
    env.set_metatable(&ctx, Some(metatable));

    env
}
