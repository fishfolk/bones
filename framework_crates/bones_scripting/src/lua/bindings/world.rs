use super::*;

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
                stack.push_front(
                    piccolo::String::from_static(&ctx, "World { resources, components, assets }")
                        .into(),
                );
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
        .set(ctx, "__newindex", ctx.singletons().get(ctx, no_newindex))
        .unwrap();
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let (world, key): (&WorldRef, lua::String) = stack.consume(ctx)?;

                let singletons = ctx.singletons();
                let resources_metatable = singletons.get(ctx, super::resources::metatable);
                let components_metatable = singletons.get(ctx, super::components::metatable);
                let assets_metatable = singletons.get(ctx, super::assets::metatable);

                match key.as_bytes() {
                    b"resources" => {
                        let resources = AnyUserData::new_static(&ctx, world.clone());
                        resources.set_metatable(&ctx, Some(resources_metatable));
                        stack.push_front(resources.into());
                    }
                    b"components" => {
                        let components = AnyUserData::new_static(&ctx, world.clone());
                        components.set_metatable(&ctx, Some(components_metatable));
                        stack.push_front(components.into());
                    }
                    b"assets" => {
                        let assets = AnyUserData::new_static(&ctx, world.clone());
                        assets.set_metatable(&ctx, Some(assets_metatable));
                        stack.push_front(assets.into());
                    }
                    _ => (),
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
