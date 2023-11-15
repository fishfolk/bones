use super::*;

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
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let (this, key): (AnyUserData, lua::String) = stack.consume(ctx)?;
                let this = this.downcast_static::<&Schema>()?;

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

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
