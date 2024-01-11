use bones_lib::prelude::*;
use gc_arena_derive::Collect;
use piccolo::{
    self as lua, meta_ops, meta_ops::MetaResult, BoxSequence, Callback, CallbackReturn, Context,
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

pub fn no_newindex(ctx: Context) -> Callback {
    Callback::from_fn(&ctx, |_ctx, _fuel, _stack| {
        Err(anyhow::format_err!("Creating fields not allowed on this type").into())
    })
}

/// Generate the environment table for executing scripts under.
pub fn env(ctx: Context) -> Table {
    let env = Table::new(&ctx);

    env.set(ctx, "math", ctx.globals().get(ctx, "math"))
        .unwrap();

    let schema_fn = ctx.singletons().get(ctx, schema::schema_fn);
    env.set(ctx, "schema", schema_fn).unwrap();
    env.set(ctx, "s", schema_fn).unwrap(); // Alias for schema
    let schema_of_fn = ctx.singletons().get(ctx, schema::schema_of_fn);
    env.set(ctx, "schema_of", schema_of_fn).unwrap();

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
            .set(ctx, name, UserData::new_static(&ctx, stage))
            .unwrap();
    }
    env.set(ctx, "CoreStage", core_stage_table).unwrap();

    macro_rules! add_log_fn {
        ($level:ident) => {
            let $level = Callback::from_fn(&ctx, |ctx, _fuel, mut stack| {
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
                        _ex: piccolo::Execution<'gc, '_>,
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

                Ok(CallbackReturn::Sequence(BoxSequence::new(
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
            Callback::from_fn(&ctx, |_ctx, _fuel, _stack| {
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
