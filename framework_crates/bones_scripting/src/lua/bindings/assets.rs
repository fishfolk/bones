use super::*;

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    metatable
        .set(ctx, "__newindex", ctx.singletons().get(ctx, no_newindex))
        .unwrap();
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
                stack.push_front(piccolo::String::from_static(&ctx, "Assets { root, get }").into());
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    let get_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
            let (world, ecsref): (&WorldRef, &EcsRef) = stack.consume(ctx)?;

            let b = ecsref.borrow();
            let handle = b.schema_ref()?.try_cast::<UntypedHandle>()?;

            let assetref = world.with(|world| EcsRef {
                data: EcsRefData::Asset(AssetRef {
                    server: (*world.resources.get::<AssetServer>().unwrap()).clone(),
                    handle: *handle,
                }),
                path: default(),
            });
            let metatable = ctx.singletons().get(ctx, assetref.metatable_fn());
            let assetref = AnyUserData::new_static(&ctx, assetref);
            assetref.set_metatable(&ctx, Some(metatable));

            stack.push_front(assetref.into());

            Ok(CallbackReturn::Return)
        }),
    );

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let (world, key): (&WorldRef, lua::Value) = stack.consume(ctx)?;

                if let Value::String(key) = key {
                    #[allow(clippy::single_match)]
                    match key.as_bytes() {
                        b"root" => {
                            world.with(|world| {
                                let asset_server = world.resources.get::<AssetServer>().unwrap();
                                let root = asset_server.core().root;
                                let assetref = EcsRef {
                                    data: EcsRefData::Asset(AssetRef {
                                        server: (*asset_server).clone(),
                                        handle: root,
                                    }),
                                    path: default(),
                                };
                                let metatable = ctx.singletons().get(ctx, assetref.metatable_fn());
                                let assetref = AnyUserData::new_static(&ctx, assetref);
                                assetref.set_metatable(&ctx, Some(metatable));
                                stack.push_front(assetref.into());
                            });
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
