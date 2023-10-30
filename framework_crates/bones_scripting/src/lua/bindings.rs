use bones_lib::prelude::*;
use gc_arena_derive::Collect;
use piccolo::Context;

use super::*;

/// Registers lua binding typedatas for bones_framework types.
pub fn register_lua_typedata() {
    <SchemaBox as HasSchema>::schema()
        .type_data
        .insert(SchemaLuaMetatable(schemabox_metatable))
        .unwrap();
}

pub fn schemabox_metatable(_luadata: &LuaData, ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);

    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, move |_ctx, _fuel, stack| {
                stack.pop_front();
                stack.push_front(Value::Integer(777));

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

pub fn no_newindex(_luadata: &LuaData, ctx: Context) -> StaticCallback {
    ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, |_ctx, _fuel, _stack| {
            Err(anyhow::format_err!("Creating fields not allowed on this type").into())
        }),
    )
}

pub fn atomicresource_metatable(_luadata: &LuaData, ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let static_metatable = ctx.state.registry.stash(&ctx, metatable);

    let static_metatable_ = static_metatable.clone();
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let value = stack.pop_front();
                if let Value::UserData(data) = value {
                    if let Ok(res) = data.downcast_static::<ResourceRef>() {
                        let data = res.cell.borrow().get_field_path(FieldPath(res.path))?;
                        stack.push_front(
                            piccolo::String::from_static(&ctx, data.schema().full_name.as_ref())
                                .into(),
                        );
                    } else {
                        stack.push_front(
                            piccolo::String::from_slice(&ctx, &format!("{value}")).into(),
                        );
                    }
                } else {
                    stack.push_front(piccolo::String::from_slice(&ctx, &format!("{value}")).into());
                };
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this = stack.pop_front();
                let key = stack.pop_front();
                let type_err = "Invalid type for `self` in schemabox metatable.";
                let Value::UserData(this) = this else {
                    return Err(anyhow::format_err!(type_err).into());
                };

                let schemaref;
                let mut path;
                let cell;
                if let Ok(res) = this.downcast_static::<ResourceRef>() {
                    cell = res.cell.clone();
                    schemaref = res.cell.borrow();
                    path = res.path;
                } else {
                    return Err(anyhow::format_err!(type_err).into());
                };

                if let Value::String(key) = key {
                    path = ustr(&format!("{path}.{}", key.to_str()?));
                } else if let Value::Integer(i) = key {
                    path = ustr(&format!("{path}.{i}"));
                } else {
                    return Err(anyhow::format_err!("Invalid index: {key}").into());
                }
                let schemaref = schemaref.get_field_path(FieldPath(path))?;

                match &schemaref.schema().kind {
                    SchemaKind::Struct(_) | SchemaKind::Primitive(Primitive::Opaque { .. }) => {
                        let new_ref = AnyUserData::new_static(&ctx, ResourceRef { cell, path });
                        new_ref.set_metatable(
                            &ctx,
                            Some(ctx.state.registry.fetch(&static_metatable_)),
                        );
                        stack.push_front(new_ref.into());
                    }
                    SchemaKind::Vec(_) => todo!(),
                    SchemaKind::Enum(_) => todo!(),
                    SchemaKind::Map { .. } => todo!(),
                    SchemaKind::Box(_) => todo!(),
                    SchemaKind::Primitive(prim) => stack.push_front(match prim {
                        Primitive::Bool => Value::Boolean(*schemaref.cast::<bool>()),
                        Primitive::U8 => Value::Integer(*schemaref.cast::<u8>() as i64),
                        Primitive::U16 => Value::Integer(*schemaref.cast::<u16>() as i64),
                        Primitive::U32 => Value::Integer(*schemaref.cast::<u32>() as i64),
                        Primitive::U64 => Value::Integer(*schemaref.cast::<u64>() as i64),
                        Primitive::U128 => Value::Integer(*schemaref.cast::<u128>() as i64),
                        Primitive::I8 => Value::Integer(*schemaref.cast::<i16>() as i64),
                        Primitive::I16 => Value::Integer(*schemaref.cast::<i16>() as i64),
                        Primitive::I32 => Value::Integer(*schemaref.cast::<i32>() as i64),
                        Primitive::I64 => Value::Integer(*schemaref.cast::<i64>()),
                        Primitive::I128 => Value::Integer(*schemaref.cast::<i128>() as i64),
                        Primitive::F32 => Value::Number(*schemaref.cast::<f32>() as f64),
                        Primitive::F64 => Value::Number(*schemaref.cast::<f64>()),
                        Primitive::String => Value::String(piccolo::String::from_slice(
                            &ctx,
                            schemaref.cast::<String>().clone(),
                        )),
                        Primitive::Opaque { .. } => unreachable!(),
                    }),
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(
            ctx,
            "__newindex",
            AnyCallback::from_fn(&ctx, move |_ctx, _fuel, stack| {
                let this = stack.pop_front();
                let key = stack.pop_front();
                let newvalue = stack.pop_front();
                let type_err = "Invalid type for `self` in schemabox metatable.";
                let Value::UserData(this) = this else {
                    return Err(anyhow::format_err!(type_err).into());
                };

                let mut schemaref;
                let mut path;
                if let Ok(res) = this.downcast_static::<ResourceRef>() {
                    schemaref = res.cell.borrow_mut();
                    path = res.path;
                } else {
                    return Err(anyhow::format_err!(type_err).into());
                };

                if let Value::String(key) = key {
                    path = ustr(&format!("{path}.{}", key.to_str()?));
                } else if let Value::Integer(i) = key {
                    path = ustr(&format!("{path}.{i}"));
                } else {
                    return Err(anyhow::format_err!("Invalid index: {key}").into());
                }
                let mut schemaref =
                    schemaref.try_get_field_path(FieldPath(path)).map_err(|_| {
                        piccolo::Error::from(anyhow::format_err!("Attribute doesn't exist: {path}"))
                    })?;

                match &schemaref.schema().kind {
                    SchemaKind::Struct(_) | SchemaKind::Primitive(Primitive::Opaque { .. }) => {
                        return Err(anyhow::format_err!("Cannot assign to structs directly").into());
                    }
                    SchemaKind::Vec(_) => todo!(),
                    SchemaKind::Enum(_) => todo!(),
                    SchemaKind::Map { .. } => todo!(),
                    SchemaKind::Box(_) => todo!(),
                    SchemaKind::Primitive(prim) => match (prim, newvalue) {
                        (Primitive::Bool, Value::Boolean(value)) => {
                            *schemaref.cast_mut::<bool>() = value;
                        }
                        (Primitive::U8, Value::Integer(value)) => {
                            *schemaref.cast_mut::<u8>() = value as u8;
                        }
                        (Primitive::U16, Value::Integer(value)) => {
                            *schemaref.cast_mut::<u16>() = value as u16;
                        }
                        (Primitive::U32, Value::Integer(value)) => {
                            *schemaref.cast_mut::<u32>() = value as u32;
                        }
                        (Primitive::U64, Value::Integer(value)) => {
                            *schemaref.cast_mut::<u64>() = value as u64;
                        }
                        (Primitive::U128, Value::Integer(value)) => {
                            *schemaref.cast_mut::<u128>() = value as u128;
                        }
                        (Primitive::I8, Value::Integer(value)) => {
                            *schemaref.cast_mut::<i8>() = value as i8;
                        }
                        (Primitive::I16, Value::Integer(value)) => {
                            *schemaref.cast_mut::<i16>() = value as i16;
                        }
                        (Primitive::I32, Value::Integer(value)) => {
                            *schemaref.cast_mut::<i32>() = value as i32;
                        }
                        (Primitive::I64, Value::Integer(value)) => {
                            *schemaref.cast_mut::<i64>() = value;
                        }
                        (Primitive::I128, Value::Integer(value)) => {
                            *schemaref.cast_mut::<i128>() = value as i128;
                        }
                        (Primitive::F32, Value::Integer(value)) => {
                            *schemaref.cast_mut::<f32>() = value as f32;
                        }
                        (Primitive::F64, Value::Integer(value)) => {
                            *schemaref.cast_mut::<f32>() = value as f32;
                        }
                        (Primitive::F32, Value::Number(value)) => {
                            *schemaref.cast_mut::<f32>() = value as f32;
                        }
                        (Primitive::F64, Value::Number(value)) => {
                            *schemaref.cast_mut::<f64>() = value;
                        }
                        (Primitive::String, Value::String(string)) => {
                            *schemaref.cast_mut::<String>() = string.to_str()?.to_owned();
                        }
                        (Primitive::Opaque { .. }, _) => unreachable!(),
                        _ => {
                            return Err(anyhow::format_err!(
                                "Type mismatch, expected `{prim:?}` found {newvalue:?}"
                            )
                            .into())
                        }
                    },
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

pub fn resources_metatable(luadata: &LuaData, ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let schemabox_metatable = luadata.table(ctx, atomicresource_metatable);
    metatable
        .set(
            ctx,
            "__newindex",
            ctx.state
                .registry
                .fetch(&luadata.callback(ctx, no_newindex)),
        )
        .unwrap();

    let get_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
            let world = stack.pop_front();
            let Value::UserData(world) = world else {
                return Err(
                    anyhow::format_err!("Type error on `self` of resources metatable.").into(),
                );
            };
            let world = world.downcast_static::<WorldRef>()?;

            let schema = stack.pop_front();
            let Value::UserData(schema) = schema else {
                return Err(
                    anyhow::format_err!("Type error on `self` of resources metatable.").into(),
                );
            };
            let schema = schema.downcast_static::<&Schema>()?;

            world.with(|world| {
                let cell = world.resources.untyped().get_cell(schema.id());

                if let Some(cell) = cell {
                    let data = AnyUserData::new_static(
                        &ctx,
                        ResourceRef {
                            cell,
                            path: ustr(""),
                        },
                    );
                    data.set_metatable(&ctx, Some(ctx.state.registry.fetch(&schemabox_metatable)));
                    stack.push_front(data.into());
                }
            });

            Ok(CallbackReturn::Return)
        }),
    );

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this = stack.pop_front();
                let key = stack.pop_front();
                let Value::UserData(world) = this else {
                    return Err(anyhow::format_err!(
                        "Type error on `self` of resources metatable."
                    )
                    .into());
                };
                let world = world.downcast_static::<WorldRef>()?;

                if let Value::String(key) = key {
                    #[allow(clippy::single_match)]
                    match key.as_bytes() {
                        b"len" => {
                            stack.push_front(Value::Integer(
                                world.with(|world| world.resources.len()) as i64,
                            ));
                        }
                        b"get" => {
                            stack.push_front(ctx.state.registry.fetch(&get_callback).into());
                        }
                        _ => (),
                    }
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

pub fn components_metatable(luadata: &LuaData, ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__newindex",
            ctx.state
                .registry
                .fetch(&luadata.callback(ctx, no_newindex)),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

/// Build the world metatable.
pub fn world_metatable(luadata: &LuaData, ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let resources_metatable = luadata.table(ctx, resources_metatable);
    let components_metatable = luadata.table(ctx, components_metatable);
    metatable
        .set(
            ctx,
            "__newindex",
            ctx.state
                .registry
                .fetch(&luadata.callback(ctx, no_newindex)),
        )
        .unwrap();
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this = stack.pop_front();
                let key = stack.pop_front();
                let type_err = anyhow::format_err!("World metatable `self` is invalid.");
                let Value::UserData(this) = this else {
                    return Err(type_err.into());
                };
                let this = this.downcast_static::<WorldRef>()?;

                let resources_metatable = ctx.state.registry.fetch(&resources_metatable);
                let components_metatable = ctx.state.registry.fetch(&components_metatable);

                if let Value::String(key) = key {
                    match key.as_bytes() {
                        b"resources" => {
                            let resources = AnyUserData::new_static(&ctx, this.clone());
                            resources.set_metatable(&ctx, Some(resources_metatable));
                            stack.push_front(resources.into());
                        }
                        b"components" => {
                            let components = AnyUserData::new_static(&ctx, this.clone());
                            components.set_metatable(&ctx, Some(components_metatable));
                            stack.push_front(components.into());
                        }
                        _ => (),
                    }
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

/// The metatable for `&'static Schema`.
pub fn schema_metatable(luadata: &LuaData, ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let schemabox_metatable = <SchemaBox as HasSchema>::schema()
        .type_data
        .get::<SchemaLuaMetatable>()
        .map(|x| (x.0)(luadata, ctx));

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this = stack.pop_front();
                let key = stack.pop_front();
                let type_err = anyhow::format_err!("World metatable `self` is invalid.");
                let Value::UserData(this) = this else {
                    return Err(type_err.into());
                };
                let this = this.downcast_static::<&Schema>()?;

                if let Value::String(key) = key {
                    match key.as_bytes() {
                        b"name" => {
                            stack.push_front(Value::String(piccolo::String::from_static(
                                &ctx,
                                this.name.as_bytes(),
                            )));
                        }
                        b"full_name" => {
                            stack.push_front(Value::String(piccolo::String::from_static(
                                &ctx,
                                this.full_name.as_bytes(),
                            )));
                        }
                        b"create" => {
                            let data = AnyUserData::new_static(&ctx, SchemaBox::default(this));
                            data.set_metatable(
                                &ctx,
                                schemabox_metatable
                                    .as_ref()
                                    .map(|x| ctx.state.registry.fetch(x)),
                            );
                            stack.push_front(data.into());
                        }
                        _ => (),
                    }
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

/// Generate the environment table for executing scripts under.
pub fn env(luadata: &LuaData, ctx: Context) -> StaticTable {
    let env = Table::new(&ctx);
    let schema_metatable = luadata.table(ctx, schema_metatable);

    env.set(
        ctx,
        "schema",
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
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

                // TODO: setup `toString` implementation so that printing schemas is informative.
                let schema = AnyUserData::new_static(&ctx, next_match);
                schema.set_metatable(&ctx, Some(ctx.state.registry.fetch(&schema_metatable)));
                stack.push_front(schema.into());
            } else {
                return Err(anyhow::format_err!("Schema not found: {schema_name}").into());
            }

            Ok(CallbackReturn::Return)
        }),
    )
    .unwrap();

    macro_rules! add_log_fn {
        ($level:ident) => {
            env.set(
                ctx,
                stringify!($level),
                AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
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
                        fn poll(
                            &mut self,
                            ctx: Context<'gc>,
                            _fuel: &mut Fuel,
                            stack: &mut Stack<'gc>,
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
                }),
            )
            .unwrap();
        };
    }

    // Register logging callbacks
    add_log_fn!(trace);
    add_log_fn!(debug);
    add_log_fn!(info);
    add_log_fn!(warn);
    add_log_fn!(error);

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

    ctx.state.registry.stash(&ctx, env)
}
