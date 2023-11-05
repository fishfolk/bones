use bones_lib::prelude::*;
use gc_arena_derive::Collect;
use piccolo::{
    meta_ops, meta_ops::MetaResult, AnyCallback, AnySequence, CallbackReturn, Context, Error,
    Sequence, SequencePoll, Stack,
};

use super::*;

/// Registers lua binding typedatas for bones_framework types.
pub fn register_lua_typedata() {
    // <AssetServer as HasSchema>::schema()
    //     .type_data
    //     .insert(SchemaLuaMetatable(assetserver_metatable))
    //     .unwrap();
}

pub fn no_newindex(ctx: Context) -> StaticCallback {
    ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, |_ctx, _fuel, _stack| {
            Err(anyhow::format_err!("Creating fields not allowed on this type").into())
        }),
    )
}

pub fn luaref_metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let static_metatable = ctx.state.registry.stash(&ctx, metatable);
    let static_metatable_ = static_metatable.clone();

    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this = stack.pop_front();
                let Value::UserData(this) = this else {
                    return Err(anyhow::format_err!("Invalid type").into());
                };
                let this = this.downcast_static::<EcsRef>()?;
                let b = this.data.borrow();
                if let Some(value) = b.access() {
                    if let Some(value) = value.field_path(FieldPath(this.path)) {
                        stack.push_front(Value::String(piccolo::String::from_slice(
                            &ctx,
                            format!("{value:?}"),
                        )));
                    }
                }
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
                let Value::UserData(this) = this else {
                    return Err(anyhow::format_err!(
                        "Invalid type for `self` in schemabox metatable."
                    )
                    .into());
                };
                let this = this.downcast_static::<EcsRef>()?;
                let b = this.data.borrow();
                let newpath = ustr(&format!("{}.{key}", this.path));

                if let Some(field) = b.access().and_then(|x| x.field_path(FieldPath(newpath))) {
                    match field {
                        SchemaRefAccess::Primitive(p)
                            if !matches!(p, PrimitiveRef::Opaque { .. }) =>
                        {
                            match p {
                                PrimitiveRef::Bool(b) => stack.push_front(Value::Boolean(*b)),
                                PrimitiveRef::U8(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U16(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U32(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U64(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U128(n) => {
                                    stack.push_front(Value::Integer(*n as i64))
                                }
                                PrimitiveRef::I8(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::I16(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::I32(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::I64(n) => stack.push_front(Value::Integer(*n)),
                                PrimitiveRef::I128(n) => {
                                    stack.push_front(Value::Integer(*n as i64))
                                }
                                PrimitiveRef::F32(n) => stack.push_front(Value::Number(*n as f64)),
                                PrimitiveRef::F64(n) => stack.push_front(Value::Number(*n)),
                                PrimitiveRef::String(s) => stack.push_front(Value::String(
                                    piccolo::String::from_slice(&ctx, s),
                                )),
                                PrimitiveRef::Opaque { .. } => unreachable!(),
                            }
                        }
                        _ => {
                            let mut newref = this.clone();
                            newref.path = newpath;
                            let data = AnyUserData::new_static(&ctx, newref);
                            data.set_metatable(
                                &ctx,
                                Some(ctx.state.registry.fetch(&static_metatable_)),
                            );
                            stack.push_front(data.into());
                        }
                    }
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(
            ctx,
            "__newindex",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this = stack.pop_front();
                let key = stack.pop_front();
                let newvalue = stack.pop_front();
                let Value::UserData(this) = this else {
                    return Err(anyhow::format_err!(
                        "Invalid type for `self` in schemabox metatable."
                    )
                    .into());
                };
                let this = this.downcast_static::<EcsRef>()?;
                let mut borrow = this.data.borrow_mut();
                let Some(access) = borrow.access_mut() else {
                    return Err(anyhow::format_err!("Value not found").into());
                };
                let newpath = ustr(&format!("{}.{key}", this.path));

                let field_idx = match key {
                    Value::Integer(i) => FieldIdx::Idx(i as usize),
                    Value::String(s) => FieldIdx::Name(match s.to_str() {
                        Ok(s) => s,
                        Err(_) => return Err(anyhow::format_err!("Non UTF-8 string").into()),
                    }),
                    _ => return Err(anyhow::format_err!("Invalid index: {key}").into()),
                };

                match access {
                    SchemaRefMutAccess::Struct(s) => {
                        match s
                            .into_field(field_idx)
                            .map_err(|_| anyhow::format_err!("Field not found: {field_idx}"))?
                        {
                            SchemaRefMutAccess::Struct(s) => {
                                if s.into_field(field_idx).is_ok() {
                                    let newecsref = EcsRef {
                                        data: this.data.clone(),
                                        path: newpath,
                                    };
                                    let newecsref = AnyUserData::new_static(&ctx, newecsref);
                                    newecsref.set_metatable(
                                        &ctx,
                                        Some(ctx.state.registry.fetch(&static_metatable)),
                                    );
                                    stack.push_front(newecsref.into());
                                }
                            }
                            SchemaRefMutAccess::Vec(_)
                            | SchemaRefMutAccess::Enum(_)
                            | SchemaRefMutAccess::Map(_) => {
                                todo!("Implement vec, enum, and map assigment")
                            }
                            SchemaRefMutAccess::Primitive(p) => match (p, newvalue) {
                                (PrimitiveRefMut::Bool(b), Value::Boolean(newb)) => *b = newb,
                                (PrimitiveRefMut::U8(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U16(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U32(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U64(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U128(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I8(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I16(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I32(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I64(n), Value::Integer(newi)) => *n = newi,
                                (PrimitiveRefMut::I128(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::F32(n), Value::Number(newf)) => *n = newf as f32,
                                (PrimitiveRefMut::F64(n), Value::Number(newf)) => *n = newf,
                                (PrimitiveRefMut::String(s), Value::String(news)) => {
                                    if let Ok(news) = news.to_str() {
                                        s.clear();
                                        s.push_str(news);
                                    } else {
                                        return Err(anyhow::format_err!(
                                            "Non UTF-8 string assignment."
                                        )
                                        .into());
                                    }
                                }
                                (PrimitiveRefMut::Opaque { .. }, Value::UserData(_)) => {
                                    todo!("Opaque type assignment")
                                }
                                _ => return Err(anyhow::format_err!("Invalid type").into()),
                            },
                        }
                    }
                    SchemaRefMutAccess::Vec(_)
                    | SchemaRefMutAccess::Enum(_)
                    | SchemaRefMutAccess::Map(_) => {
                        todo!("Implement vec, enum, and map assigment.")
                    }
                    SchemaRefMutAccess::Primitive(_) => {
                        return Err(
                            anyhow::format_err!("Cannot assign to field of primitive.").into()
                        )
                    }
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

pub fn resources_metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let luadata = ctx.luadata();
    let luaref_metatable = luadata.table(ctx, luaref_metatable);
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
                        EcsRef {
                            data: EcsRefData::Resource(cell),
                            path: default(),
                        },
                    );
                    data.set_metatable(&ctx, Some(ctx.state.registry.fetch(&luaref_metatable)));
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

pub fn components_metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__newindex",
            ctx.state
                .registry
                .fetch(&ctx.luadata().callback(ctx, no_newindex)),
        )
        .unwrap();

    ctx.state.registry.stash(&ctx, metatable)
}

/// Build the world metatable.
pub fn world_metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let luadata = ctx.luadata();
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
pub fn schema_metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let luadata = ctx.luadata();
    let ecsref_metatable = luadata.table(ctx, luaref_metatable);

    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this = stack.pop_front();
                let type_err = anyhow::format_err!("World metatable `self` is invalid.");
                let Value::UserData(this) = this else {
                    return Err(type_err.into());
                };
                let this = this.downcast_static::<&Schema>()?;
                let s = piccolo::String::from_slice(&ctx, &format!("Schema({})", this.full_name));

                stack.push_front(Value::String(s));
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    let create_fn = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
            let this = stack.pop_front();
            let type_err = anyhow::format_err!("World metatable `self` is invalid.");
            let Value::UserData(this) = this else {
                return Err(type_err.into());
            };
            let this = this.downcast_static::<&Schema>()?;

            let data = AnyUserData::new_static(
                &ctx,
                EcsRef {
                    data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::default(this)))),
                    path: default(),
                },
            );
            data.set_metatable(&ctx, Some(ctx.state.registry.fetch(&ecsref_metatable)));
            stack.push_front(data.into());

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
                        b"create" => stack.push_front(ctx.state.registry.fetch(&create_fn).into()),
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
pub fn env(ctx: Context) -> StaticTable {
    let env = Table::new(&ctx);
    let luadata = ctx.luadata();
    let schema_metatable = luadata.table(ctx, schema_metatable);

    let schema_fn = AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
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
    });
    env.set(ctx, "schema", schema_fn).unwrap();
    env.set(ctx, "s", schema_fn).unwrap(); // Alias for schema

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
