use super::*;

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    let singletons = ctx.singletons();
    metatable
        .set(ctx, "__newindex", singletons.get(ctx, no_newindex))
        .unwrap();
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
                stack.push_front(
                    piccolo::String::from_static(&ctx, "Resources { len, get }").into(),
                );
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    let get_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
            let world = stack.pop_front();
            let Value::UserData(world) = world else {
                return Err(anyhow::format_err!(
                    "`get` must be called as a method: resources:get()"
                )
                .into());
            };
            let world = world.downcast_static::<WorldRef>()?;

            let schema = stack.pop_front();
            let Value::UserData(schema) = schema else {
                return Err(anyhow::format_err!(
                    "Type error in `get()`: argument must be a Schema.`"
                )
                .into());
            };
            let schema = schema.downcast_static::<&Schema>()?;

            world.with(|world| {
                let cell = world.resources.untyped().get_cell(schema.id());
                if let Some(cell) = cell {
                    let ecsref = EcsRef {
                        data: EcsRefData::Resource(cell),
                        path: default(),
                    };
                    let metatable = ctx.singletons().get(ctx, ecsref.metatable_fn());
                    let data = AnyUserData::new_static(&ctx, ecsref);
                    data.set_metatable(&ctx, Some(metatable));
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

    metatable
}
