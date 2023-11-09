use super::*;

pub fn entities_metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
                stack.push_front(
                    piccolo::String::from_static(&ctx, "Entities { create, kill }").into(),
                );
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(ctx, "__newindex", ctx.singletons().get(ctx, no_newindex))
        .unwrap();

    let create_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
            let this: &EcsRef = stack.consume(ctx)?;

            let mut b = this.data.borrow_mut();
            let mut binding = b
                .schema_ref_mut()
                .unwrap()
                .into_field_path(FieldPath(this.path))
                .unwrap();
            let entities = binding.cast_mut::<Entities>();
            let entity = entities.create();
            let newecsref = EcsRef {
                data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::new(entity)))),
                path: default(),
            };
            let metatable = ctx.singletons().get(ctx, newecsref.metatable_fn());
            let newecsref = AnyUserData::new_static(&ctx, newecsref);
            newecsref.set_metatable(&ctx, Some(metatable));

            stack.push_front(newecsref.into());

            Ok(CallbackReturn::Return)
        }),
    );
    let kill_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
            let (this, entity_ecsref): (&EcsRef, &EcsRef) = stack.consume(ctx)?;
            let mut b = this.data.borrow_mut();
            let mut binding = b
                .schema_ref_mut()
                .unwrap()
                .into_field_path(FieldPath(this.path))
                .unwrap();
            let entities = binding.cast_mut::<Entities>();

            let b = entity_ecsref.data.borrow();
            let binding = b
                .schema_ref()
                .unwrap()
                .field_path(FieldPath(entity_ecsref.path))
                .unwrap();
            let entity = binding.cast::<Entity>();
            entities.kill(*entity);

            Ok(CallbackReturn::Return)
        }),
    );
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let (_this, key): (lua::Value, lua::String) = stack.consume(ctx)?;

                #[allow(clippy::single_match)]
                match key.as_bytes() {
                    b"create" => {
                        stack.push_front(ctx.state.registry.fetch(&create_callback).into());
                    }
                    b"kill" => {
                        stack.push_front(ctx.state.registry.fetch(&kill_callback).into());
                    }
                    _ => (),
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
