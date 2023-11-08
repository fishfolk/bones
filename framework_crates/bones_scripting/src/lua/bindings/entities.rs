use super::*;

pub fn entities_metatable(ctx: Context) -> StaticTable {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
                stack.push_front(piccolo::String::from_static(&ctx, "Entities { create }").into());
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
                .fetch(&ctx.luadata().callback(ctx, no_newindex)),
        )
        .unwrap();

    let create_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
            let this = stack.pop_front();
            let Value::UserData(this) = this else {
                return Err(
                    anyhow::format_err!("Type error on `self` of resources metatable.").into(),
                );
            };
            let ecsref = this.downcast_static::<EcsRef>()?;
            let mut b = ecsref.data.borrow_mut();
            let mut binding = b
                .access_mut()
                .unwrap()
                .field_path(FieldPath(ecsref.path))
                .unwrap()
                .into_schema_ref_mut();
            let entities = binding.cast_mut::<Entities>();
            let entity = entities.create();
            let newecsref = EcsRef {
                data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::new(entity)))),
                path: default(),
            };
            let metatable = ctx.luadata().table(ctx, newecsref.metatable_fn());
            let newecsref = AnyUserData::new_static(&ctx, newecsref);
            newecsref.set_metatable(&ctx, Some(ctx.state.registry.fetch(&metatable)));

            stack.push_front(newecsref.into());

            Ok(CallbackReturn::Return)
        }),
    );
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let _this = stack.pop_front();
                let key = stack.pop_front();

                if let Value::String(key) = key {
                    #[allow(clippy::single_match)]
                    match key.as_bytes() {
                        b"create" => {
                            stack.push_front(ctx.state.registry.fetch(&create_callback).into());
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

// pub fn entity_metatable(ctx: Context) -> StaticTable {
//     let metatable = Table::new(&ctx);
//     metatable
//         .set(
//             ctx,
//             "__tostring",
//             AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
//                 stack.push_front(piccolo::String::from_static(&ctx, "Entities").into());
//                 Ok(CallbackReturn::Return)
//             }),
//         )
//         .unwrap();
//     metatable
//         .set(
//             ctx,
//             "__newindex",
//             ctx.state
//                 .registry
//                 .fetch(&ctx.luadata().callback(ctx, no_newindex)),
//         )
//         .unwrap();

//     ctx.state.registry.stash(&ctx, metatable)
// }
