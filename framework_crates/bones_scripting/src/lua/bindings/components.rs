use super::*;

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, stack| {
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

    let get_callback = ctx.state.registry.stash(&ctx, lua::Value::Nil);
    let insert_callback = ctx.state.registry.stash(&ctx, lua::Value::Nil);
    let remove_callback = ctx.state.registry.stash(&ctx, lua::Value::Nil);

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let (_world, key): (&WorldRef, lua::Value) = stack.consume(ctx)?;

                if let Value::String(key) = key {
                    #[allow(clippy::single_match)]
                    match key.as_bytes() {
                        b"get" => {
                            stack.push_front(ctx.state.registry.fetch(&get_callback));
                        }
                        b"insert" => {
                            stack.push_front(ctx.state.registry.fetch(&insert_callback));
                        }
                        b"remove" => {
                            stack.push_front(ctx.state.registry.fetch(&remove_callback));
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
