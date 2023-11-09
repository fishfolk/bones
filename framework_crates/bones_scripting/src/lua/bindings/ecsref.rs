use super::*;

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);

    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this: &EcsRef = stack.consume(ctx)?;

                let b = this.data.borrow();
                if let Some(value) = b.schema_ref() {
                    if let Some(value) = value.field_path(FieldPath(this.path)) {
                        let access = value.access();
                        stack.push_front(Value::String(piccolo::String::from_slice(
                            &ctx,
                            format!("{access:?}"),
                        )));
                    }
                }
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let (this, key): (&EcsRef, lua::String) = stack.consume(ctx)?;

                let b = this.data.borrow();
                let newpath = ustr(&format!("{}.{key}", this.path));

                if let Some(field) = b
                    .schema_ref()
                    .and_then(|x| x.field_path(FieldPath(newpath)))
                {
                    match field.access() {
                        SchemaRefAccess::Primitive(p)
                            if !matches!(p, PrimitiveRef::Opaque { .. }) =>
                        {
                            match p {
                                PrimitiveRef::Bool(b) => stack.push_front(Value::Boolean(*b)),
                                PrimitiveRef::U8(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U16(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U32(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U64(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::U128(n) => {
                                    stack.push_front(Value::Integer(*n as i64))
                                }
                                PrimitiveRef::I8(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::I16(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::I32(n) => stack.push_front(Value::Integer(*n as i64)),
                                PrimitiveRef::I64(n) => stack.push_front(Value::Integer(*n)),
                                PrimitiveRef::I128(n) => {
                                    stack.push_front(Value::Integer(*n as i64))
                                }
                                PrimitiveRef::F32(n) => stack.push_front(Value::Number(*n as f64)),
                                PrimitiveRef::F64(n) => stack.push_front(Value::Number(*n)),
                                PrimitiveRef::String(s) => stack.push_front(Value::String(
                                    piccolo::String::from_slice(&ctx, s),
                                )),
                                PrimitiveRef::Opaque { .. } => unreachable!(),
                            }
                        }
                        _ => {
                            let mut newref = this.clone();
                            newref.path = newpath;
                            let metatable = ctx.singletons().get(ctx, newref.metatable_fn());
                            let data = AnyUserData::new_static(&ctx, newref);
                            data.set_metatable(&ctx, Some(metatable));
                            stack.push_front(data.into());
                        }
                    }
                } else {
                    return Err(anyhow::format_err!("Invalid field {newpath}").into());
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(
            ctx,
            "__newindex",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let (this, key, newvalue): (&EcsRef, lua::Value, lua::Value) =
                    stack.consume(ctx)?;

                let mut borrow = this.data.borrow_mut();
                let Some(mut schema_ref) = borrow.schema_ref_mut() else {
                    return Err(anyhow::format_err!("Value not found").into());
                };
                let newpath = ustr(&format!("{}.{key}", this.path));

                let field_idx = match key {
                    Value::Integer(i) => FieldIdx::Idx(i as usize),
                    Value::String(s) => FieldIdx::Name(match s.to_str() {
                        Ok(s) => s,
                        Err(_) => return Err(anyhow::format_err!("Non UTF-8 string").into()),
                    }),
                    _ => return Err(anyhow::format_err!("Invalid index: {key}").into()),
                };

                match schema_ref.access_mut() {
                    SchemaRefMutAccess::Struct(s) => {
                        match s
                            .into_field(field_idx)
                            .map_err(|_| anyhow::format_err!("Field not found: {field_idx}"))?
                        {
                            SchemaRefMutAccess::Struct(s) => {
                                if s.into_field(field_idx).is_ok() {
                                    let newecsref = EcsRef {
                                        data: this.data.clone(),
                                        path: newpath,
                                    };
                                    let metatable =
                                        ctx.singletons().get(ctx, newecsref.metatable_fn());
                                    let newecsref = AnyUserData::new_static(&ctx, newecsref);
                                    newecsref.set_metatable(&ctx, Some(metatable));
                                    stack.push_front(newecsref.into());
                                }
                            }
                            SchemaRefMutAccess::Vec(_)
                            | SchemaRefMutAccess::Enum(_)
                            | SchemaRefMutAccess::Map(_) => {
                                todo!("Implement vec, enum, and map assigment")
                            }
                            SchemaRefMutAccess::Primitive(p) => match (p, newvalue) {
                                (PrimitiveRefMut::Bool(b), Value::Boolean(newb)) => *b = newb,
                                (PrimitiveRefMut::U8(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U16(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U32(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U64(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::U128(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I8(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I16(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I32(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::I64(n), Value::Integer(newi)) => *n = newi,
                                (PrimitiveRefMut::I128(n), Value::Integer(newi)) => {
                                    *n = newi.try_into().unwrap()
                                }
                                (PrimitiveRefMut::F32(n), Value::Number(newf)) => *n = newf as f32,
                                (PrimitiveRefMut::F64(n), Value::Number(newf)) => *n = newf,
                                (PrimitiveRefMut::String(s), Value::String(news)) => {
                                    if let Ok(news) = news.to_str() {
                                        s.clear();
                                        s.push_str(news);
                                    } else {
                                        return Err(anyhow::format_err!(
                                            "Non UTF-8 string assignment."
                                        )
                                        .into());
                                    }
                                }
                                (PrimitiveRefMut::Opaque { .. }, Value::UserData(_)) => {
                                    todo!("Opaque type assignment")
                                }
                                _ => return Err(anyhow::format_err!("Invalid type").into()),
                            },
                        }
                    }
                    SchemaRefMutAccess::Vec(_)
                    | SchemaRefMutAccess::Enum(_)
                    | SchemaRefMutAccess::Map(_) => {
                        todo!("Implement vec, enum, and map assigment.")
                    }
                    SchemaRefMutAccess::Primitive(_) => {
                        return Err(
                            anyhow::format_err!("Cannot assign to field of primitive.").into()
                        )
                    }
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
