use super::*;

/// A wrapper around [`Schema`] that indicates that it should be excluded from, for example,
/// an `entities:iter_with()` lua call.
pub(super) struct WithoutSchema(pub &'static Schema);

pub fn schema_fn(ctx: Context) -> AnyCallback {
    AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
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
    })
}

pub fn schema_of_fn(ctx: Context) -> AnyCallback {
    AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
        let singletons = ctx.singletons();
        let schema_metatable = singletons.get(ctx, schema::metatable);

        let ecsref: &EcsRef = stack.consume(ctx)?;
        let schema = ecsref.borrow().schema_ref()?.schema();

        let schema = AnyUserData::new_static(&ctx, schema);
        schema.set_metatable(&ctx, Some(schema_metatable));
        stack.replace(ctx, schema);

        Ok(CallbackReturn::Return)
    })
}

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let this: AnyUserData = stack.consume(ctx)?;
                let this = this.downcast_static::<&Schema>()?;
                let s = piccolo::String::from_slice(&ctx, &format!("Schema({})", this.full_name));

                stack.push_front(Value::String(s));
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    let create_fn = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let this: AnyUserData = stack.consume(ctx)?;
            let this = this.downcast_static::<&Schema>()?;

            let ecsref = EcsRef {
                data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::default(this)))),
                path: default(),
            }
            .into_value(ctx);
            stack.push_front(ecsref);

            Ok(CallbackReturn::Return)
        }),
    );

    let without_fn = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let this: AnyUserData = stack.consume(ctx)?;
            let this = this.downcast_static::<&Schema>()?;
            stack.replace(ctx, AnyUserData::new_static(&ctx, WithoutSchema(this)));
            Ok(CallbackReturn::Return)
        }),
    );

    let eq_fn = AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
        let (this, other): (AnyUserData, AnyUserData) = stack.consume(ctx)?;
        let (this, other) = (
            this.downcast_static::<&Schema>()?,
            other.downcast_static::<&Schema>()?,
        );
        stack.replace(ctx, this.id() == other.id());
        Ok(CallbackReturn::Return)
    });

    metatable.set(ctx, "__eq", eq_fn).unwrap();
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let (this, key): (AnyUserData, lua::String) = stack.consume(ctx)?;
                let this = this.downcast_static::<&Schema>()?;

                match key.as_bytes() {
                    b"name" => {
                        stack.replace(
                            ctx,
                            Value::String(piccolo::String::from_static(&ctx, this.name.as_bytes())),
                        );
                    }
                    b"full_name" => {
                        stack.replace(
                            ctx,
                            Value::String(piccolo::String::from_static(
                                &ctx,
                                this.full_name.as_bytes(),
                            )),
                        );
                    }
                    b"create" => stack.replace(ctx, ctx.state.registry.fetch(&create_fn)),
                    b"without" => stack.replace(ctx, ctx.state.registry.fetch(&without_fn)),
                    _ => (),
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
