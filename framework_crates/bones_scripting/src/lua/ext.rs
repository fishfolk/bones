use super::*;

/// Extension trait for the [`Context`] that makes it easier to access our lua singletons.
pub trait CtxExt {
    /// Get a reference to the lua singletons.
    fn singletons(&self) -> &LuaSingletons;
}
impl CtxExt for piccolo::Context<'_> {
    fn singletons(&self) -> &LuaSingletons {
        let Value::UserData(data) = self.state.globals.get(*self, "luasingletons") else {
            unreachable!();
        };
        data.downcast_static::<LuaSingletons>().unwrap()
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
