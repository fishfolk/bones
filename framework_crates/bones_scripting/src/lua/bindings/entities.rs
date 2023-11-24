use lua::Variadic;

use crate::prelude::bindings::schema::WithoutSchema;

use super::*;

pub fn entities_metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);
    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
                stack.push_front(
                    piccolo::String::from_static(&ctx, "Entities { create, kill, iter_with }")
                        .into(),
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
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let this: &EcsRef = stack.consume(ctx)?;

            let mut b = this.borrow_mut();
            let entities = b.schema_ref_mut()?.cast_into_mut::<Entities>();

            let entity = entities.create();
            let newecsref = EcsRef {
                data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::new(entity)))),
                path: default(),
            }
            .into_value(ctx);
            stack.push_front(newecsref);

            Ok(CallbackReturn::Return)
        }),
    );
    let kill_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let (this, entity_ecsref): (&EcsRef, &EcsRef) = stack.consume(ctx)?;
            let mut b = this.borrow_mut();
            let entities = b.schema_ref_mut()?.cast_into_mut::<Entities>();

            let b = entity_ecsref.borrow();
            let entity = b.schema_ref()?.cast::<Entity>();
            entities.kill(*entity);

            Ok(CallbackReturn::Return)
        }),
    );
    let iter_with_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let (this, schema_args): (&EcsRef, Variadic<Vec<AnyUserData>>) = stack.consume(ctx)?;
            let mut b = this.borrow_mut();
            let entities = b.schema_ref_mut()?.cast_into_mut::<Entities>();
            let world = ctx
                .state
                .globals
                .get(ctx, "world")
                .as_static_user_data::<WorldRef>()?;
            let mut bitset = entities.bitset().clone();

            let mut schemas = Vec::with_capacity(schema_args.len());
            world.with(|world| {
                for schema_arg in &schema_args {
                    if let Ok(schema) = schema_arg.downcast_static::<&Schema>() {
                        let components = world.components.get_by_schema(schema);
                        let components = components.borrow();
                        bitset.bit_and(components.bitset());
                        schemas.push(*schema);
                    } else if let Ok(without_schema) = schema_arg.downcast_static::<WithoutSchema>()
                    {
                        let components = world.components.get_by_schema(without_schema.0);
                        let components = components.borrow();
                        bitset.bit_and(components.bitset().clone().bit_not());
                    } else {
                        return Err(anyhow::format_err!(
                            "Invalid type for argument to `entities:iter_with()`: {schema_arg:?}"
                        ));
                    }
                }
                Ok::<_, anyhow::Error>(())
            })?;
            let entities = entities
                .iter_with_bitset(&bitset)
                .collect::<Vec<_>>()
                .into_iter();

            struct IteratorState {
                pub entities: std::vec::IntoIter<Entity>,
                schemas: Vec<&'static Schema>,
            }

            let iter_fn = AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
                let state: AnyUserData = stack.consume(ctx)?;
                let state = state.downcast_static::<AtomicCell<IteratorState>>()?;
                let mut state = state.borrow_mut();
                let next_ent = state.entities.next();

                if let Some(entity) = next_ent {
                    let world = ctx
                        .state
                        .globals
                        .get(ctx, "world")
                        .as_static_user_data::<WorldRef>()?;

                    let ecsref = EcsRef {
                        data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::new(entity)))),
                        path: default(),
                    }
                    .into_value(ctx);
                    stack.push_back(ecsref);

                    world.with(|world| {
                        for schema in &state.schemas {
                            let store = world.components.get_cell_by_schema(schema);
                            let ecsref = EcsRef {
                                data: EcsRefData::Component(ComponentRef { store, entity }),
                                path: default(),
                            }
                            .into_value(ctx);
                            stack.push_back(ecsref);
                        }

                        Ok::<_, anyhow::Error>(())
                    })?;
                }

                Ok(CallbackReturn::Return)
            });

            let iterator_state =
                AnyUserData::new_static(&ctx, AtomicCell::new(IteratorState { entities, schemas }));

            stack.replace(ctx, (iter_fn, iterator_state));

            Ok(CallbackReturn::Return)
        }),
    );

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let (_this, key): (lua::Value, lua::String) = stack.consume(ctx)?;

                #[allow(clippy::single_match)]
                match key.as_bytes() {
                    b"create" => {
                        stack.push_front(ctx.state.registry.fetch(&create_callback).into());
                    }
                    b"kill" => {
                        stack.push_front(ctx.state.registry.fetch(&kill_callback).into());
                    }
                    b"iter_with" => {
                        stack.push_front(ctx.state.registry.fetch(&iter_with_callback).into());
                    }
                    _ => (),
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
