use super::*;

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
                stack.push_front(
                    piccolo::String::from_static(&ctx, "Components { insert, remove, get }").into(),
                );
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(ctx, "__newindex", ctx.singletons().get(ctx, no_newindex))
        .unwrap();

    let get_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
            let (world, entity_ecsref, schema): (&WorldRef, &EcsRef, AnyUserData) =
                stack.consume(ctx)?;

            let b = entity_ecsref.borrow();
            let entity = *b.schema_ref()?.try_cast::<Entity>()?;

            let schema = *schema.downcast_static::<&Schema>()?;

            let store = world.with(|world| {
                let store = world.components.get_cell_by_schema(schema);
                Ok::<_, anyhow::Error>(store)
            })?;

            let ecsref = EcsRef {
                data: EcsRefData::Component(ComponentRef { store, entity }),
                path: default(),
            };

            // Return nil if the component is not present
            if ecsref.borrow().schema_ref().is_err() {
                stack.replace(ctx, Value::Nil);
                return Ok(CallbackReturn::Return);
            }

            let ecsref = ecsref.into_value(ctx);
            stack.replace(ctx, ecsref);

            Ok(CallbackReturn::Return)
        }),
    );
    let insert_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
            let (world, entity_ecsref, value_ecsref): (&WorldRef, &EcsRef, &EcsRef) =
                stack.consume(ctx)?;

            let b = entity_ecsref.borrow();
            let entity = *b.schema_ref()?.try_cast::<Entity>()?;

            let value = {
                let b = value_ecsref.borrow();
                let value = b.schema_ref()?;
                value.clone_into_box()
            };

            world.with(|world| {
                let store = world.components.get_by_schema(value.schema());
                let mut store = store.borrow_mut();
                store.insert_box(entity, value);
                Ok::<_, anyhow::Error>(())
            })?;

            Ok(CallbackReturn::Return)
        }),
    );
    let remove_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
            let (world, entity_ecsref, schema): (&WorldRef, &EcsRef, AnyUserData) =
                stack.consume(ctx)?;

            let b = entity_ecsref.borrow();
            let entity = *b.schema_ref()?.try_cast::<Entity>()?;

            let schema = *schema.downcast_static::<&Schema>()?;

            world.with(|world| {
                let store = world.components.get_by_schema(schema);
                let mut store = store.borrow_mut();
                store.remove_box(entity);
                Ok::<_, anyhow::Error>(())
            })?;

            Ok(CallbackReturn::Return)
        }),
    );

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let (_world, key): (&WorldRef, lua::Value) = stack.consume(ctx)?;

                if let Value::String(key) = key {
                    #[allow(clippy::single_match)]
                    match key.as_bytes() {
                        b"get" => {
                            stack.push_front(ctx.state.registry.fetch(&get_callback).into());
                        }
                        b"insert" => {
                            stack.push_front(ctx.state.registry.fetch(&insert_callback).into());
                        }
                        b"remove" => {
                            stack.push_front(ctx.state.registry.fetch(&remove_callback).into());
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
