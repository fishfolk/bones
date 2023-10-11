//! Implements the system API for the ECS.

use std::sync::Arc;

use crate::prelude::*;

#[derive(Deref, DerefMut)]
struct Test {
    s: String,
}

/// Trait implemented by systems.
pub trait System<In, Out> {
    /// Initialize the system, creating any component or resource storages necessary for the system
    /// to run in the world.
    fn initialize(&self, world: &mut World);
    /// Run the system.
    fn run(&mut self, world: &World, input: In) -> Out;
    /// Get a best-effort name for the system, used in diagnostics.
    fn name(&self) -> &str;
}

/// Struct containing a static system.
pub struct StaticSystem<In, Out> {
    /// This should be called once to initialize the system, allowing it to intialize any resources
    /// or components in the world.
    ///
    /// Usually only called once, but this is not guaranteed so the implementation should be
    /// idempotent.
    pub initialize: fn(&mut World),
    /// This is run every time the system is executed
    pub run: Box<dyn FnMut(&World, In) -> Out + Send + Sync>,
    /// A best-effort name for the system, for diagnostic purposes.
    pub name: &'static str,
}

impl<In, Out> System<In, Out> for StaticSystem<In, Out> {
    fn initialize(&self, world: &mut World) {
        (self.initialize)(world)
    }
    fn run(&mut self, world: &World, input: In) -> Out {
        (self.run)(world, input)
    }
    fn name(&self) -> &str {
        self.name
    }
}

/// Converts a function into a [`System`].
///
/// [`IntoSystem`] is automatically implemented for all functions and closures that:
///
/// - Have 26 or less arguments,
/// - Where every argument implments [`SystemParam`], and
///
/// The most common [`SystemParam`] types that you will use as arguments to a system will be:
/// - [`Res`] and [`ResMut`] parameters to access resources
/// - [`Comp`] and [`CompMut`] parameters to access components
/// - [`&World`][World] to access the world directly
/// - [`In`] for systems which have an input value. This must be the first argument of the function.
pub trait IntoSystem<Args, In, Out> {
    /// The type of the system that is output
    type Sys: System<In, Out>;

    /// Convert into a [`System`].
    fn system(self) -> Self::Sys;
}

impl<T, In, Out> IntoSystem<T, In, Out> for T
where
    T: System<In, Out>,
{
    type Sys = T;
    fn system(self) -> Self::Sys {
        self
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
    fn borrow<'s>(world: &'s World, state: &'s mut Self::State) -> Self::Param<'s>;
}

impl SystemParam for &'_ World {
    type State = ();
    type Param<'s> = &'s World;
    fn initialize(_world: &mut World) {}
    fn get_state(_world: &World) -> Self::State {}
    fn borrow<'s>(world: &'s World, _state: &'s mut Self::State) -> Self::Param<'s> {
        world
    }
}

/// The system input parameter.
#[derive(Deref, DerefMut)]
pub struct In<T>(pub T);

/// [`SystemParam`] for getting read access to a resource.
///
/// Use [`ResInit`] if you want to automatically initialize the resource.
pub struct Res<'a, T: HasSchema>(Ref<'a, T>);
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
pub struct ResInit<'a, T: HasSchema + FromWorld>(Ref<'a, T>);
impl<'a, T: HasSchema + FromWorld> std::ops::Deref for ResInit<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// [`SystemParam`] for getting mutable access to a resource.
///
/// Use [`ResMutInit`] if you want to automatically initialize the resource.
pub struct ResMut<'a, T: HasSchema>(RefMut<'a, T>);
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
pub struct ResMutInit<'a, T: HasSchema + FromWorld>(RefMut<'a, T>);
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

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        Res(state.borrow())
    }
}

impl<'a, T: HasSchema> SystemParam for Option<Res<'a, T>> {
    type State = Option<AtomicResource<T>>;
    type Param<'p> = Option<Res<'p, T>>;

    fn initialize(_world: &mut World) {}

    fn get_state(world: &World) -> Self::State {
        world.resources.get_cell::<T>()
    }

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        state.as_ref().map(|state| Res(state.borrow()))
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

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
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
                `ResMutInit` instead of `ResMut` to automatically initialize the \
                resource with the default value.",
                std::any::type_name::<T>()
            )
        })
    }

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        ResMut(state.borrow_mut())
    }
}

impl<'a, T: HasSchema> SystemParam for Option<ResMut<'a, T>> {
    type State = Option<AtomicResource<T>>;
    type Param<'p> = Option<ResMut<'p, T>>;

    fn initialize(_world: &mut World) {}

    fn get_state(world: &World) -> Self::State {
        world.resources.get_cell::<T>()
    }

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        state.as_mut().map(|state| ResMut(state.borrow_mut()))
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

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        ResMutInit(state.borrow_mut())
    }
}

/// [`SystemParam`] for getting read access to a [`ComponentStore`].
pub type Comp<'a, T> = Ref<'a, ComponentStore<T>>;
/// [`SystemParam`] for getting mutable access to a [`ComponentStore`].
pub type CompMut<'a, T> = RefMut<'a, ComponentStore<T>>;

impl<'a, T: HasSchema> SystemParam for Comp<'a, T> {
    type State = Arc<AtomicCell<ComponentStore<T>>>;
    type Param<'p> = Comp<'p, T>;

    fn initialize(world: &mut World) {
        world.components.init::<T>();
    }

    fn get_state(world: &World) -> Self::State {
        world.components.get_cell::<T>().unwrap()
    }

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        state.borrow()
    }
}

impl<'a, T: HasSchema> SystemParam for CompMut<'a, T> {
    type State = Arc<AtomicCell<ComponentStore<T>>>;
    type Param<'p> = CompMut<'p, T>;

    fn initialize(world: &mut World) {
        world.components.init::<T>();
    }

    fn get_state(world: &World) -> Self::State {
        world.components.get_cell::<T>().unwrap()
    }

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
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
        > IntoSystem<(F, $($args,)*), (), Out> for F
        where for<'a> F: 'static + Send + Sync +
            FnMut(
                $(
                    <$args as SystemParam>::Param<'a>,
                )*
            ) -> Out +
            FnMut(
                $(
                    $args,
                )*
            ) -> Out
        {
            type Sys = StaticSystem<(), Out>;
            fn system(mut self) -> Self::Sys {
                StaticSystem {
                    name: std::any::type_name::<F>(),
                    initialize: |_world| {
                        $(
                            $args::initialize(_world);
                        )*
                    },
                    run: Box::new(move |_world, _input| {
                        $(
                            #[allow(non_snake_case)]
                            let mut $args = $args::get_state(_world);
                        )*

                        self(
                            $(
                                $args::borrow(_world, &mut $args),
                            )*
                        )
                    })
                }
            }
        }
    };
}

macro_rules! impl_system_with_input {
    ($($args:ident,)*) => {
        #[allow(unused_parens)]
        impl<
            'input,
            F,
            InT: 'input,
            Out,
            $(
                $args: SystemParam,
            )*
        > IntoSystem<(F, InT, $($args,)*), InT, Out> for F
        where for<'a> F: 'static + Send + Sync +
            FnMut(
                In<InT>,
                $(
                    <$args as SystemParam>::Param<'a>,
                )*
            ) -> Out +
            FnMut(
                In<InT>,
                $(
                    $args,
                )*
            ) -> Out
        {
            type Sys = StaticSystem<InT, Out>;
            fn system(mut self) -> Self::Sys {
                StaticSystem {
                    name: std::any::type_name::<F>(),
                    initialize: |_world| {
                        $(
                            $args::initialize(_world);
                        )*
                    },
                    run: Box::new(move |_world, input| {
                        $(
                            #[allow(non_snake_case)]
                            let mut $args = $args::get_state(_world);
                        )*

                        self(
                            In(input),
                            $(
                                $args::borrow(_world, &mut $args),
                            )*
                        )
                    })
                }
            }
        }
    };
}

macro_rules! impl_systems {
    // base case
    () => {
        impl_system!();
        impl_system_with_input!();
    };
    // recursive call
    ($head:ident, $($idents:ident,)*) => {
        impl_system!($head, $($idents,)*);
        impl_system_with_input!($head, $($idents,)*);
        impl_systems!($($idents,)*);
    }
}

impl_systems!(A, B, C, D, E, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn convert_system() {
        fn tmp(
            _var1: Ref<ComponentStore<u32>>,
            _var2: Ref<ComponentStore<u64>>,
            _var3: Res<i32>,
            _var4: ResMut<i64>,
        ) -> u32 {
            0
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
        ) {
        }
        fn tmp3(_in: In<usize>, _comp1: Comp<i64>) {}
        let _ = tmp.system();
        let _ = tmp2.system();
        let _ = tmp3.system();
    }

    #[test]
    fn system_is_send() {
        let x = 6;
        send(
            (move |_var1: Res<u32>| {
                let _y = x;
            })
            .system(),
        );
        send((|| ()).system());
        send(sys.system());
    }

    fn sys(_var1: Res<u32>) {}
    fn send<T: Send>(_t: T) {}

    #[test]
    fn optional_resource() {
        fn access_resource(
            a: Option<Res<u8>>,
            b: Option<Res<u16>>,
            c: Option<ResMut<u32>>,
            d: Option<ResMut<u64>>,
        ) {
            assert!(a.as_deref() == None);
            assert!(b.as_deref() == Some(&1));
            assert!(c.as_deref() == None);
            assert!(d.as_deref() == Some(&mut 2));
        }

        let mut world = World::new();
        world.insert_resource(1u16);
        world.insert_resource(2u64);
        world.run_system(access_resource, ());
    }

    #[test]
    fn in_and_out() {
        fn mul_by_res(n: In<usize>, r: Res<usize>) -> usize {
            *n * *r
        }

        fn sys_with_ref_in(mut n: In<&mut usize>) {
            **n *= 3;
        }

        let mut world = World::new();
        world.insert_resource(2usize);

        let result = world.run_initialized_system(mul_by_res, 3);
        assert_eq!(result, 6);

        let mut n = 3;
        world.run_initialized_system(sys_with_ref_in, &mut n)
    }

    #[test]
    fn system_replace_resource() {
        #[derive(Default, HasSchema, Clone, PartialEq, Eq, Debug)]
        pub struct A;
        #[derive(Default, HasSchema, Clone, Debug)]
        pub struct B {
            x: u32,
        }
        let mut world = World::default();
        let mut my_system = (|_a: ResInit<A>, mut b: ResMutInit<B>| {
            let b2 = B { x: 45 };
            *b = b2;
        })
        .system();

        assert!(world.resources.get_cell::<B>().is_none());
        my_system.initialize(&mut world);

        {
            let res = world.resource::<B>();
            assert_eq!(res.x, 0);
        }

        my_system.run(&world, ());

        let res = world.resource::<B>();
        assert_eq!(res.x, 45);

        let res = world.resource::<A>();
        assert_eq!(*res, A);
    }
}
