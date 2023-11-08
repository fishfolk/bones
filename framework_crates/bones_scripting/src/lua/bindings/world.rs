use super::*;

pub fn metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    let luadata = ctx.luadata();
    let resources_metatable = luadata.table(ctx, super::resources::metatable);
    let components_metatable = luadata.table(ctx, super::components::metatable);
    let assets_metatable = luadata.table(ctx, super::assets::metatable);
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
                let assets_metatable = ctx.state.registry.fetch(&assets_metatable);

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
                        b"assets" => {
                            let assets = AnyUserData::new_static(&ctx, this.clone());
                            assets.set_metatable(&ctx, Some(assets_metatable));
                            stack.push_front(assets.into());
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
