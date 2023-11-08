use super::*;

pub fn metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
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

            let ecsref = EcsRef {
                data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::default(this)))),
                path: default(),
            };
            let metatable = ctx.luadata().table(ctx, ecsref.metatable_fn());
            let data = AnyUserData::new_static(&ctx, ecsref);
            data.set_metatable(&ctx, Some(ctx.state.registry.fetch(&metatable)));
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
