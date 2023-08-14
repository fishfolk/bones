//! Implements the system API for the ECS.

use crate::prelude::*;

#[derive(Deref, DerefMut)]
struct Test {
    s: String,
}

/// Struct used to run a system function using the world.
pub struct System<Out = ()> {
    /// This should be called once to initialize the system, allowing it to intialize any resources
    /// or components in the world.
    ///
    /// Usually only called once, but this is not guaranteed so the implementation should be
    /// idempotent.
    pub initialize: Box<dyn Send + Sync + Fn(&mut World)>,
    /// This is run every time the system is executed
    pub run: Box<dyn Send + Sync + FnMut(&World) -> SystemResult<Out>>,
    /// A best-effort name for the system, for diagnostic purposes.
    pub name: &'static str,
}

impl<Out> std::fmt::Debug for System<Out> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<Out> System<Out> {
    /// Initializes the resources required to run this system inside of the provided [`World`], if
    /// those resources don't already exist.
    ///
    /// This is usually only called once, but this is not guaranteed so the implementation should be
    /// idempotent.
    pub fn initialize(&self, world: &mut World) {
        (self.initialize)(world)
    }

    /// Runs the system's function using the provided [`World`]
    pub fn run(&mut self, world: &World) -> SystemResult<Out> {
        (self.run)(world)
    }

    /// Returns the underlying type name of the system.
    ///
    /// This is not guranteed to be stable or human-readable, but can be used for diagnostics.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

/// Converts a function into a [`System`].
///
/// [`IntoSystem`] is automatically implemented for all functions and closures that:
///
/// - Have 26 or less arguments,
/// - Where every argument implments [`SystemParam`], and
/// - That returns either `()` or [`SystemResult`]
///
/// [`IntoSystem`] is also implemented for functions that take [`&World`][World] as an argument, and
/// return either `()` or [`SystemResult`].
///
/// The most common [`SystemParam`] types that you will use as arguments to a system will be:
///  - [`Res`] and [`ResMut`] parameters to access resources
/// - [`Comp`] and [`CompMut`] parameters to access components
pub trait IntoSystem<Args, Out> {
    /// Convert into a [`System`].
    fn system(self) -> System<Out>;
}

impl<Out> IntoSystem<System<Out>, Out> for System<Out> {
    fn system(self) -> System<Out> {
        self
    }
}

impl<F, Out> IntoSystem<(World, F), Out> for F
where
    F: FnMut(&World) -> Out + Send + Sync + 'static,
{
    fn system(mut self) -> System<Out> {
        System {
            initialize: Box::new(|_| ()),
            run: Box::new(move |world| Ok(self(world))),
            name: std::any::type_name::<F>(),
        }
    }
}
impl<F, Out> IntoSystem<(World, F, SystemResult<Out>), Out> for F
where
    F: FnMut(&World) -> SystemResult<Out> + Send + Sync + 'static,
{
    fn system(self) -> System<Out> {
        System {
            initialize: Box::new(|_| ()),
            run: Box::new(self),
            name: std::any::type_name::<F>(),
        }
    }
}

/// Trait used to implement parameters for [`System`] functions.
///
/// Functions that only take arguments implementing [`SystemParam`] automatically implment
/// [`IntoSystem`].
///
/// Implementing [`SystemParam`] manually can be useful for creating new kinds of parameters you may
/// use in your system funciton arguments. Examples might inlclude event readers and writers or
/// other custom ways to access the data inside a [`World`].
pub trait SystemParam: Sized {
    /// The intermediate state for the parameter, that may be extracted from the world.
    type State;
    /// The type of the parameter, ranging over the lifetime of the intermediate state.
    ///
    /// > **ℹ️ Important:** This type must be the same type as `Self`, other than the fact that it
    /// > may range over the lifetime `'s` instead of a generic lifetime from your `impl`.
    /// >
    /// > If the type is not the same, then system functions will not be able to take it as an
    /// > argument.
    type Param<'s>;
    /// This will be called to give the parameter a chance to initialize it's world storage.
    ///
    /// You can use this chance to init any resources or components you need in the world.
    fn initialize(world: &mut World);
    /// This is called to produce the intermediate state of the system parameter.
    ///
    /// This state will be created immediately before the system is run, and will kept alive until
    /// the system is done running.
    fn get_state(world: &World) -> Self::State;
    /// This is used create an instance of the system parame, possibly borrowed from the
    /// intermediate parameter state.
    #[allow(clippy::needless_lifetimes)] // Explicit lifetimes help clarity in this case
    fn borrow<'s>(state: &'s mut Self::State) -> Self::Param<'s>;
}

/// [`SystemParam`] for getting read access to a resource.
///
/// Use [`Res`] if you want to automatically initialize the resource.
pub struct Res<'a, T: HasSchema>(AtomicRef<'a, T>);
impl<'a, T: HasSchema> std::ops::Deref for Res<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// [`SystemParam`] for getting read access to a resource and initialzing it if it doesn't already
/// exist.
///
/// Use [`Res`] if you don't want to automatically initialize the resource.
pub struct ResInit<'a, T: HasSchema + FromWorld>(AtomicRef<'a, T>);
impl<'a, T: HasSchema + FromWorld> std::ops::Deref for ResInit<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// [`SystemParam`] for getting mutable access to a resource.
///
/// Use [`ResMutInit`] if you want to automatically initialize the resource.
pub struct ResMut<'a, T: HasSchema>(AtomicRefMut<'a, T>);
impl<'a, T: HasSchema> std::ops::Deref for ResMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, T: HasSchema> std::ops::DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// [`SystemParam`] for getting mutable access to a resource and initializing it if it doesn't
/// already exist.
///
/// Use [`ResMut`] if you don't want to automatically initialize the resource.
pub struct ResMutInit<'a, T: HasSchema + FromWorld>(AtomicRefMut<'a, T>);
impl<'a, T: HasSchema + FromWorld> std::ops::Deref for ResMutInit<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, T: HasSchema + FromWorld> std::ops::DerefMut for ResMutInit<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: HasSchema> SystemParam for Res<'a, T> {
    type State = AtomicResource<T>;
    type Param<'p> = Res<'p, T>;

    fn initialize(_world: &mut World) {}

    fn get_state(world: &World) -> Self::State {
        world.resources.get_cell::<T>().unwrap_or_else(|| {
            panic!(
                "Resource of type `{}` not in world. \
                You may need to insert or initialize the resource or use \
                `ResInit` instead of `Res` to automatically initialize the \
                resource with the default value.",
                std::any::type_name::<T>()
            )
        })
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        Res(state.borrow())
    }
}

impl<'a, T: HasSchema + FromWorld> SystemParam for ResInit<'a, T> {
    type State = AtomicResource<T>;
    type Param<'p> = ResInit<'p, T>;

    fn initialize(world: &mut World) {
        if !world.resources.contains::<T>() {
            world.init_resource::<T>();
        }
    }

    fn get_state(world: &World) -> Self::State {
        world.resources.get_cell::<T>().unwrap()
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        ResInit(state.borrow())
    }
}

impl<'a, T: HasSchema> SystemParam for ResMut<'a, T> {
    type State = AtomicResource<T>;
    type Param<'p> = ResMut<'p, T>;

    fn initialize(_world: &mut World) {}

    fn get_state(world: &World) -> Self::State {
        world.resources.get_cell::<T>().unwrap_or_else(|| {
            panic!(
                "Resource of type `{}` not in world. \
                You may need to insert or initialize the resource or use \
                `ResInit` instead of `Res` to automatically initialize the \
                resource with the default value.",
                std::any::type_name::<T>()
            )
        })
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        ResMut(state.borrow_mut())
    }
}

impl<'a, T: HasSchema + FromWorld> SystemParam for ResMutInit<'a, T> {
    type State = AtomicResource<T>;
    type Param<'p> = ResMutInit<'p, T>;

    fn initialize(world: &mut World) {
        if !world.resources.contains::<T>() {
            world.init_resource::<T>();
        }
    }

    fn get_state(world: &World) -> Self::State {
        world.resources.get_cell::<T>().unwrap()
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        ResMutInit(state.borrow_mut())
    }
}

/// [`SystemParam`] for getting read access to a [`ComponentStore`].
pub type Comp<'a, T> = AtomicComponentStoreRef<'a, T>;
/// [`SystemParam`] for getting mutable access to a [`ComponentStore`].
pub type CompMut<'a, T> = AtomicComponentStoreRefMut<'a, T>;

impl<'a, T: HasSchema> SystemParam for Comp<'a, T> {
    type State = AtomicComponentStore<T>;
    type Param<'p> = Comp<'p, T>;

    fn initialize(world: &mut World) {
        world.components.init::<T>();
    }

    fn get_state(world: &World) -> Self::State {
        world.components.get_cell::<T>()
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        state.borrow()
    }
}

impl<'a, T: HasSchema> SystemParam for CompMut<'a, T> {
    type State = AtomicComponentStore<T>;
    type Param<'p> = CompMut<'p, T>;

    fn initialize(world: &mut World) {
        world.components.init::<T>();
    }

    fn get_state(world: &World) -> Self::State {
        world.components.get_cell::<T>()
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        state.borrow_mut()
    }
}

macro_rules! impl_system {
    ($($args:ident,)*) => {
        #[allow(unused_parens)]
        impl<
            F,
            Out,
            $(
                $args: SystemParam,
            )*
        > IntoSystem<(F, $($args,)*), Out> for F
        where for<'a> F: 'static + Send + Sync +
            FnMut(
                $(
                    <$args as SystemParam>::Param<'a>,
                )*
            ) -> SystemResult<Out> +
            FnMut(
                $(
                    $args,
                )*
            ) -> SystemResult<Out>
        {
            fn system(mut self) -> System<Out> {
                System {
                    name: std::any::type_name::<F>(),
                    initialize: Box::new(|_world| {
                        $(
                            $args::initialize(_world);
                        )*
                    }),
                    run: Box::new(move |_world| {
                        $(
                            #[allow(non_snake_case)]
                            let mut $args = $args::get_state(_world);
                        )*

                        self(
                            $(
                                $args::borrow(&mut $args),
                            )*
                        )
                    })
                }
            }
        }
    };
}

macro_rules! impl_system_with_empty_return {
    ($($args:ident,)*) => {
        #[allow(unused_parens)]
        impl<
            F,
            $(
                $args: SystemParam,
            )*
        > IntoSystem<(F, $($args,)* ()), ()> for F
        where for<'a> F: 'static + Send + Sync +
            FnMut(
                $(
                    <$args as SystemParam>::Param<'a>,
                )*
            ) +
            FnMut(
                $(
                    $args,
                )*
            )
        {
            fn system(mut self) -> System<()> {
                System {
                    name: std::any::type_name::<F>(),
                    initialize: Box::new(|_world| {
                        $(
                            $args::initialize(_world);
                        )*
                    }),
                    run: Box::new(move |_world| {
                        $(
                            #[allow(non_snake_case)]
                            let mut $args = $args::get_state(_world);
                        )*

                        self(
                            $(
                                $args::borrow(&mut $args),
                            )*
                        );

                        Ok(())
                    })
                }
            }
        }
    };
}

macro_rules! impl_systems {
    // base case
    () => {};
    ($head:ident, $($idents:ident,)*) => {
        // recursive call
        impl_system!($head, $($idents,)*);
        impl_system_with_empty_return!($head, $($idents,)*);
        impl_systems!($($idents,)*);
    }
}

impl_system!();
impl_system_with_empty_return!();
impl_systems!(A, B, C, D, E, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn convert_system() {
        fn tmp(
            _var1: AtomicComponentStoreRef<u32>,
            _var2: AtomicComponentStoreRef<u64>,
            _var3: Res<i32>,
            _var4: ResMut<i64>,
        ) -> SystemResult {
            Ok(())
        }
        // Technically reusing the same type is incorrect and causes a runtime panic.
        // However, there doesn't seem to be a clean way to handle type inequality in generics.
        #[allow(clippy::too_many_arguments)]
        fn tmp2(
            _var7: Comp<i64>,
            _var8: CompMut<i64>,
            _var1: Res<u32>,
            _var2: ResMut<u64>,
            _var3: Res<u32>,
            _var4: ResMut<u64>,
            _var5: Res<u32>,
            _var6: ResMut<u64>,
            _var9: Comp<i64>,
            _var10: CompMut<i64>,
            _var11: Comp<i64>,
            _var12: CompMut<u64>,
        ) -> SystemResult {
            Ok(())
        }
        let _ = tmp.system();
        let _ = tmp2.system();
    }

    #[test]
    fn system_is_send() {
        let x = 6;
        send(
            (move |_var1: Res<u32>| {
                let _y = x;
                Ok(())
            })
            .system(),
        );
        send((|| Ok(())).system());
        send(sys.system());
    }

    fn sys(_var1: Res<u32>) -> SystemResult {
        Ok(())
    }
    fn send<T: Send>(_t: T) {}

    #[test]
    fn manual_system_run() {
        let mut world = World::default();
        world.init_resource::<u32>();
    }

    #[test]
    fn system_replace_resource() {
        #[derive(Default, HasSchema, Clone, PartialEq, Eq, Debug)]
        #[schema(opaque)]
        pub struct A;
        #[derive(Default, HasSchema, Clone, Debug)]
        #[schema(opaque)]
        pub struct B {
            x: u32,
        }
        let mut world = World::default();
        let mut my_system = (|_a: Res<A>, mut b: ResMut<B>| {
            let b2 = B { x: 45 };
            *b = b2;
            Ok(())
        })
        .system();

        assert!(world.resources.get_cell::<B>().is_none());
        my_system.initialize(&mut world);

        let res = world.resource::<B>();
        assert_eq!(res.x, 0);

        my_system.run(&world).unwrap();

        let res = world.resource::<B>();
        assert_eq!(res.x, 45);

        let res = world.resource::<A>();
        assert_eq!(*res, A);
    }
}
