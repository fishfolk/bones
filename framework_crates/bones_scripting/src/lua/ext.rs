use super::{
    bindings::{EcsRef, SchemaLuaEcsRefMetatable},
    *,
};

/// Extension trait for the [`Context`] that makes it easier to access our lua singletons.
pub trait CtxExt {
    /// Get a reference to the lua singletons.
    fn singletons(&self) -> &LuaSingletons;
}
impl CtxExt for piccolo::Context<'_> {
    fn singletons(&self) -> &LuaSingletons {
        let Value::UserData(data) = self.globals().get(*self, "luasingletons") else {
            unreachable!();
        };
        data.downcast_static::<LuaSingletons>().unwrap()
    }
}

/// Helper trait to get a singleton fn pointer for the metatable for a type.
pub trait MetatableFn {
    fn metatable_fn(&self) -> fn(piccolo::Context) -> piccolo::Table;
}
impl MetatableFn for SchemaRef<'_> {
    fn metatable_fn(&self) -> fn(piccolo::Context) -> piccolo::Table {
        self.schema()
            .type_data
            .get::<SchemaLuaEcsRefMetatable>()
            .map(|x| x.0)
            .unwrap_or(bindings::ecsref::metatable)
    }
}
impl MetatableFn for SchemaRefMut<'_> {
    fn metatable_fn(&self) -> fn(piccolo::Context) -> piccolo::Table {
        self.schema()
            .type_data
            .get::<SchemaLuaEcsRefMetatable>()
            .map(|x| x.0)
            .unwrap_or(bindings::ecsref::metatable)
    }
}
impl MetatableFn for EcsRef {
    /// Get the function that may be used to retrieve the metatable to use for this [`EcsRef`].
    fn metatable_fn(&self) -> fn(piccolo::Context) -> piccolo::Table {
        (|| {
            let b = self.borrow();
            Some(b.schema_ref().ok()?.metatable_fn())
        })()
        .unwrap_or(bindings::ecsref::metatable)
    }
}

/// Extension trait on top of [`Value`] to add helper functions.
pub trait ValueExt<'gc> {
    /// Convert to a static user data type.
    fn as_static_user_data<T: 'static>(&self) -> Result<&'gc T, piccolo::TypeError>;
}
impl<'gc> ValueExt<'gc> for Value<'gc> {
    fn as_static_user_data<T: 'static>(&self) -> Result<&'gc T, piccolo::TypeError> {
        if let Value::UserData(t) = self {
            Ok(t.downcast_static().map_err(|_| piccolo::TypeError {
                expected: std::any::type_name::<T>(),
                found: "other user data",
            })?)
        } else {
            Err(piccolo::TypeError {
                expected: std::any::type_name::<T>(),
                found: "other lua value",
            })
        }
    }
}
