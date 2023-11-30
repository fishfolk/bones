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
            AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
                stack.push_front(
                    piccolo::String::from_static(&ctx, "Resources { len, get }").into(),
                );
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    let get_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let (world, schema): (&WorldRef, AnyUserData) = stack.consume(ctx)?;

            let schema = schema.downcast_static::<&Schema>()?;

            world.with(|world| {
                let cell = world.resources.untyped().get_cell(schema);
                let ecsref = EcsRef {
                    data: EcsRefData::Resource(cell),
                    path: default(),
                }
                .into_value(ctx);
                stack.push_front(ecsref);
            });

            Ok(CallbackReturn::Return)
        }),
    );

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let (_world, key): (&WorldRef, lua::String) = stack.consume(ctx)?;

                #[allow(clippy::single_match)]
                match key.as_bytes() {
                    b"get" => {
                        stack.push_front(ctx.state.registry.fetch(&get_callback).into());
                    }
                    _ => (),
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
