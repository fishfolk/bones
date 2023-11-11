use super::*;

/// A type data that can be used to specify a custom metatable to use for the type when it is
/// used in an [`EcsRef`] in the lua API.
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
pub struct SchemaLuaEcsRefMetatable(pub fn(piccolo::Context) -> piccolo::Table);

/// A reference to an ECS-compatible value.
#[derive(Clone)]
pub struct EcsRef {
    /// The kind of reference.
    pub data: EcsRefData,
    /// The path to the desired field.
    pub path: Ustr,
}
impl Default for EcsRef {
    fn default() -> Self {
        #[derive(HasSchema, Clone, Default)]
        struct Void;
        Self {
            data: EcsRefData::Free(Rc::new(AtomicCell::new(SchemaBox::new(Void)))),
            path: default(),
        }
    }
}
impl<'gc> FromValue<'gc> for &'gc EcsRef {
    fn from_value(_ctx: Context<'gc>, value: Value<'gc>) -> Result<Self, piccolo::TypeError> {
        value.as_static_user_data::<EcsRef>()
    }
}

impl EcsRef {
    /// Borrow the value pointed to by the [`EcsRef`]
    pub fn borrow(&self) -> EcsRefBorrow {
        EcsRefBorrow {
            borrow: self.data.borrow(),
            path: self.path,
        }
    }

    /// Mutably borrow the value pointed to by the [`EcsRef`]
    pub fn borrow_mut(&self) -> EcsRefBorrowMut {
        EcsRefBorrowMut {
            borrow: self.data.borrow_mut(),
            path: self.path,
        }
    }
}

/// A borrow of an [`EcsRef`].
pub struct EcsRefBorrow<'a> {
    borrow: EcsRefBorrowKind<'a>,
    path: Ustr,
}

impl EcsRefBorrow<'_> {
    /// Get the [`SchemaRef`].
    pub fn schema_ref(&self) -> Result<SchemaRef, EcsRefBorrowError> {
        let b = self.borrow.schema_ref()?;
        let b = b
            .field_path(FieldPath(self.path))
            .ok_or(EcsRefBorrowError::FieldNotFound(self.path))?;
        Ok(b)
    }
}

/// A mutable borrow of an [`EcsRef`].
pub struct EcsRefBorrowMut<'a> {
    borrow: EcsRefBorrowMutKind<'a>,
    path: Ustr,
}

impl EcsRefBorrowMut<'_> {
    /// Get the [`SchemaRef`].
    pub fn schema_ref_mut(&mut self) -> Result<SchemaRefMut, EcsRefBorrowError> {
        let b = self.borrow.schema_ref_mut()?;
        let b = b
            .into_field_path(FieldPath(self.path))
            .ok_or(EcsRefBorrowError::FieldNotFound(self.path))?;
        Ok(b)
    }
}

/// The kind of value reference for [`EcsRef`].
#[derive(Clone)]
pub enum EcsRefData {
    /// A resource ref.
    Resource(UntypedAtomicResource),
    /// A component ref.
    Component(ComponentRef),
    /// An asset ref.
    Asset(AssetRef),
    /// A free-standing ref, not stored in the ECS.
    Free(Rc<AtomicCell<SchemaBox>>),
}

/// A kind of borrow into an [`EcsRef`].
pub enum EcsRefBorrowKind<'a> {
    Resource(AtomicSchemaRef<'a>),
    Component(ComponentBorrow<'a>),
    Free(Ref<'a, SchemaBox>),
    Asset(Option<MappedRef<'a, Cid, LoadedAsset, SchemaBox>>),
}

/// An error that occurs when borrowing an [`EcsRef`].
#[derive(Debug)]
pub enum EcsRefBorrowError {
    MissingComponent {
        entity: Entity,
        component_name: &'static str,
    },
    AssetNotLoaded,
    FieldNotFound(Ustr),
}
impl std::error::Error for EcsRefBorrowError {}
impl std::fmt::Display for EcsRefBorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EcsRefBorrowError::MissingComponent {
                entity,
                component_name,
            } => write!(
                f,
                "Entity {entity:?} does not have component `{component_name}`"
            ),
            EcsRefBorrowError::AssetNotLoaded => write!(f, "Asset not loaded"),
            EcsRefBorrowError::FieldNotFound(field) => write!(f, "Field not found: {field}"),
        }
    }
}

impl EcsRefBorrowKind<'_> {
    /// Get the borrow as a [`SchemaRef`].
    ///
    /// Will return none if the value does not exist, such as an unloaded asset or a component
    /// that is not set for a given entity.
    pub fn schema_ref(&self) -> Result<SchemaRef, EcsRefBorrowError> {
        match self {
            EcsRefBorrowKind::Resource(r) => Ok(r.schema_ref()),
            EcsRefBorrowKind::Component(c) => {
                c.borrow
                    .get_ref(c.entity)
                    .ok_or(EcsRefBorrowError::MissingComponent {
                        entity: c.entity,
                        component_name: &c.borrow.schema().full_name,
                    })
            }
            EcsRefBorrowKind::Free(f) => Ok(f.as_ref()),
            EcsRefBorrowKind::Asset(a) => a
                .as_ref()
                .map(|x| x.as_ref())
                .ok_or(EcsRefBorrowError::AssetNotLoaded),
        }
    }
}

/// A component borrow into an [`EcsRef`].
pub struct ComponentBorrow<'a> {
    pub borrow: Ref<'a, UntypedComponentStore>,
    pub entity: Entity,
}

/// A mutable component borrow into an [`EcsRef`].
pub struct ComponentBorrowMut<'a> {
    pub borrow: RefMut<'a, UntypedComponentStore>,
    pub entity: Entity,
}

/// A kind of mutable borrow of an [`EcsRef`].
pub enum EcsRefBorrowMutKind<'a> {
    Resource(AtomicSchemaRefMut<'a>),
    Component(ComponentBorrowMut<'a>),
    Free(RefMut<'a, SchemaBox>),
    Asset(Option<MappedRefMut<'a, Cid, LoadedAsset, SchemaBox>>),
}

impl EcsRefBorrowMutKind<'_> {
    /// Get the borrow as a [`SchemaRefMut`].
    ///
    /// Will return none if the value does not exist, such as an unloaded asset or a component
    /// that is not set for a given entity.
    pub fn schema_ref_mut(&mut self) -> Result<SchemaRefMut, EcsRefBorrowError> {
        match self {
            EcsRefBorrowMutKind::Resource(r) => Ok(r.schema_ref_mut()),
            EcsRefBorrowMutKind::Component(c) => {
                c.borrow
                    .get_ref_mut(c.entity)
                    .ok_or(EcsRefBorrowError::MissingComponent {
                        entity: c.entity,
                        component_name: &c.borrow.schema().full_name,
                    })
            }
            EcsRefBorrowMutKind::Free(f) => Ok(f.as_mut()),
            EcsRefBorrowMutKind::Asset(a) => a
                .as_mut()
                .map(|x| x.as_mut())
                .ok_or(EcsRefBorrowError::AssetNotLoaded),
        }
    }
}

impl EcsRefData {
    /// Immutably borrow the data.
    pub fn borrow(&self) -> EcsRefBorrowKind {
        match self {
            EcsRefData::Resource(resource) => {
                let b = resource.borrow();
                EcsRefBorrowKind::Resource(b)
            }
            EcsRefData::Component(componentref) => {
                let b = componentref.store.borrow();
                EcsRefBorrowKind::Component(ComponentBorrow {
                    borrow: b,
                    entity: componentref.entity,
                })
            }
            EcsRefData::Asset(assetref) => {
                let b = assetref.server.try_get_untyped(assetref.handle);
                EcsRefBorrowKind::Asset(b)
            }
            EcsRefData::Free(rc) => {
                let b = rc.borrow();
                EcsRefBorrowKind::Free(b)
            }
        }
    }

    /// Mutably borrow the data.
    pub fn borrow_mut(&self) -> EcsRefBorrowMutKind {
        match self {
            EcsRefData::Resource(resource) => {
                let b = resource.borrow_mut();
                EcsRefBorrowMutKind::Resource(b)
            }
            EcsRefData::Component(componentref) => {
                let b = componentref.store.borrow_mut();
                EcsRefBorrowMutKind::Component(ComponentBorrowMut {
                    borrow: b,
                    entity: componentref.entity,
                })
            }
            EcsRefData::Asset(assetref) => {
                let b = assetref.server.try_get_untyped_mut(assetref.handle);
                EcsRefBorrowMutKind::Asset(b)
            }
            EcsRefData::Free(rc) => {
                let b = rc.borrow_mut();
                EcsRefBorrowMutKind::Free(b)
            }
        }
    }
}

/// A reference to component in an [`EcsRef`].
#[derive(Clone)]
pub struct ComponentRef {
    /// The component store.
    pub store: UntypedAtomicComponentStore,
    /// The entity to get the component data for.
    pub entity: Entity,
}

/// A reference to an asset in an [`EcsRef`]
#[derive(Clone)]
pub struct AssetRef {
    /// The asset server handle.
    pub server: AssetServer,
    /// The kind of asset we are referencing.
    pub handle: UntypedHandle,
}

pub fn metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);

    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, stack| {
                let this: &EcsRef = stack.consume(ctx)?;

                let b = this.borrow();
                if let Ok(value) = b.schema_ref() {
                    let access = value.access();
                    stack.push_front(Value::String(piccolo::String::from_slice(
                        &ctx,
                        format!("{access:?}"),
                    )));
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

                let mut newref = this.clone();
                newref.path = ustr(&format!("{}.{key}", this.path));
                let b = newref.borrow();

                match b.schema_ref()?.access() {
                    SchemaRefAccess::Primitive(p) if !matches!(p, PrimitiveRef::Opaque { .. }) => {
                        match p {
                            PrimitiveRef::Bool(b) => stack.push_front(Value::Boolean(*b)),
                            PrimitiveRef::U8(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::U16(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::U32(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::U64(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::U128(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::I8(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::I16(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::I32(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::I64(n) => stack.push_front(Value::Integer(*n)),
                            PrimitiveRef::I128(n) => stack.push_front(Value::Integer(*n as i64)),
                            PrimitiveRef::F32(n) => stack.push_front(Value::Number(*n as f64)),
                            PrimitiveRef::F64(n) => stack.push_front(Value::Number(*n)),
                            PrimitiveRef::String(s) => stack
                                .push_front(Value::String(piccolo::String::from_slice(&ctx, s))),
                            PrimitiveRef::Opaque { .. } => unreachable!(),
                        }
                    }
                    _ => {
                        let metatable = ctx.singletons().get(ctx, newref.metatable_fn());
                        let data = AnyUserData::new_static(&ctx, newref.clone());
                        data.set_metatable(&ctx, Some(metatable));
                        stack.push_front(data.into());
                    }
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

                let mut this = this.clone();
                this.path = ustr(&format!("{}.{key}", this.path));
                let mut b = this.borrow_mut();
                let mut this_ref = b.schema_ref_mut()?;

                match this_ref.access_mut() {
                    SchemaRefMutAccess::Struct(_)
                    | SchemaRefMutAccess::Vec(_)
                    | SchemaRefMutAccess::Enum(_)
                    | SchemaRefMutAccess::Map(_) => {
                        let newvalue = newvalue.as_static_user_data::<EcsRef>()?;
                        let newvalue_b = newvalue.borrow();
                        let newvalue_ref = newvalue_b.schema_ref()?;
                        this_ref.write(newvalue_ref)?;
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
                        (PrimitiveRefMut::F32(n), Value::Integer(newi)) => *n = newi as f32,
                        (PrimitiveRefMut::F64(n), Value::Integer(newi)) => *n = newi as f64,
                        (PrimitiveRefMut::String(s), Value::String(news)) => {
                            if let Ok(news) = news.to_str() {
                                s.clear();
                                s.push_str(news);
                            } else {
                                return Err(
                                    anyhow::format_err!("Non UTF-8 string assignment.").into()
                                );
                            }
                        }
                        (PrimitiveRefMut::Opaque { .. }, Value::UserData(_)) => {
                            todo!("Opaque type assignment")
                        }
                        _ => return Err(anyhow::format_err!("Invalid type").into()),
                    },
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}
