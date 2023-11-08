use crate::prelude::*;

use bevy_tasks::{ComputeTaskPool, TaskPool, ThreadExecutor};
use bones_asset::dashmap::mapref::one::{MappedRef, MappedRefMut};
use bones_lib::ecs::utils::*;

use parking_lot::Mutex;
use piccolo::{
    registry::{Fetchable, Stashable},
    AnyUserData, Closure, Context, Fuel, Lua, ProtoCompileError, StaticClosure, Table, Thread,
    ThreadMode, Value,
};
use send_wrapper::SendWrapper;
use std::{any::Any, rc::Rc, sync::Arc};

#[macro_use]
mod freeze;
use freeze::*;

mod asset;
pub use asset::*;

pub mod bindings;

/// Install the scripting plugin.
pub fn lua_game_plugin(game: &mut Game) {
    // Register asset type.
    LuaScript::register_schema();

    // Add `SchemaLuaMetatable` type data for common types.
    bindings::register_lua_typedata();

    // Initialize the lua engine resource.
    game.init_shared_resource::<LuaEngine>();
}

/// A frozen reference to the ECS [`World`].
///
// This type can be converted into lua userdata for accessing the world from lua.
#[derive(Deref, DerefMut, Clone)]
pub struct WorldRef(Frozen<Freeze![&'freeze World]>);

impl WorldRef {
    /// Convert this [`WorldRef`] into a Lua userdata.
    pub fn into_userdata<'gc>(
        self,
        ctx: Context<'gc>,
        world_metatable: Table<'gc>,
    ) -> AnyUserData<'gc> {
        let data = AnyUserData::new_static(&ctx, self);
        data.set_metatable(&ctx, Some(world_metatable));
        data
    }
}

/// Resource used to access the lua scripting engine.
#[derive(HasSchema, Clone)]
#[schema(no_default)]
pub struct LuaEngine {
    /// The thread-local task executor that is used to spawn any tasks that need access to the
    /// lua engine which can only be accessed on it's own thread.
    executor: Arc<ThreadExecutor<'static>>,
    /// The lua engine state container.
    state: Arc<SendWrapper<EngineState>>,
}

/// Internal state for [`LuaEngine`]
struct EngineState {
    /// The Lua engine.
    lua: Mutex<Lua>,
    /// Persisted lua data we need stored in Rust, such as the environment table, world
    /// metatable, etc.
    data: LuaSingletons,
    /// Cache of the content IDs of loaded scripts, and their compiled lua closures.
    compiled_scripts: Mutex<HashMap<Cid, StaticClosure>>,
}

trait CtxExt {
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

impl Default for EngineState {
    fn default() -> Self {
        // Initialize an empty lua engine and our lua data.
        let mut lua = Lua::core();
        lua.try_run(|ctx| {
            ctx.state.globals.set(
                ctx,
                "luasingletons",
                AnyUserData::new_static(&ctx, LuaSingletons::default()),
            )?;
            Ok(())
        })
        .unwrap();
        Self {
            lua: Mutex::new(lua),
            data: default(),
            compiled_scripts: default(),
        }
    }
}

impl Default for LuaEngine {
    /// Initialize the Lua engine.
    fn default() -> Self {
        // Make sure the compute task pool is initialized
        ComputeTaskPool::init(TaskPool::new);

        #[cfg(not(target_arch = "wasm32"))]
        let executor = {
            let (send, recv) = async_channel::bounded(1);

            // Spawn the executor task that will be used for the lua engine.
            let pool = ComputeTaskPool::get();
            pool.spawn_local(async move {
                let executor = Arc::new(ThreadExecutor::new());
                send.try_send(executor.clone()).unwrap();

                let ticker = executor.ticker().unwrap();
                loop {
                    ticker.tick().await;
                }
            })
            .detach();
            pool.with_local_executor(|local| while local.try_tick() {});

            recv.try_recv().unwrap()
        };

        #[cfg(target_arch = "wasm32")]
        let executor = Arc::new(ThreadExecutor::new());

        LuaEngine {
            executor,
            state: Arc::new(SendWrapper::new(default())),
        }
    }
}

impl LuaEngine {
    /// Access the lua engine to run code on it.
    pub fn exec<'a, F: FnOnce(&mut Lua) + Send + 'a>(&self, f: F) {
        let pool = ComputeTaskPool::get();

        // Create a new scope spawned on the lua engine thread.
        pool.scope_with_executor(false, Some(&self.executor), |scope| {
            scope.spawn_on_external(async {
                f(&mut self.state.lua.lock());
            });
        });
    }

    /// Run a lua script as a system on the given world.
    pub fn run_script_system(&self, world: &World, script: Handle<LuaScript>) {
        self.exec(|lua| {
            Frozen::<Freeze![&'freeze World]>::in_scope(world, |world| {
                // Wrap world reference so that it can be converted to lua userdata.
                let world = WorldRef(world);

                let result = lua.try_run(|ctx| {
                    // Create a thread
                    let thread = Thread::new(&ctx);

                    // Fetch the env table
                    let env = self.state.data.get(ctx, bindings::env);

                    // Compile the script
                    let closure = world.with(|world| {
                        let asset_server = world.resource::<AssetServer>();
                        let cid = *asset_server
                            .store
                            .asset_ids
                            .get(&script.untyped())
                            .ok_or_else(|| {
                                tracing::warn!("Script asset not loaded.");
                                ProtoCompileError::Parser(
                                    piccolo::compiler::ParserError::EndOfStream { expected: None },
                                )
                            })?;

                        let mut compiled_scripts = self.state.compiled_scripts.lock();
                        let closure = compiled_scripts.get(&cid);

                        Ok::<_, ProtoCompileError>(match closure {
                            Some(closure) => ctx.state.registry.fetch(closure),
                            None => {
                                let asset = asset_server.store.assets.get(&cid).unwrap();
                                let source = &asset.data.cast_ref::<LuaScript>().source;
                                let closure = Closure::load_with_env(ctx, source.as_bytes(), env)?;
                                compiled_scripts
                                    .insert(cid, ctx.state.registry.stash(&ctx, closure));

                                closure
                            }
                        })
                    })?;

                    // Insert the world ref into the global scope
                    let world = world
                        .into_userdata(ctx, self.state.data.get(ctx, bindings::world::metatable));
                    env.set(ctx, "world", world)?;

                    // Start the thread
                    thread.start(ctx, closure.into(), ())?;

                    // Run the thread to completion
                    let mut fuel = Fuel::with_fuel(i32::MAX);
                    loop {
                        // If the thread is ready
                        if matches!(thread.mode(), ThreadMode::Normal) {
                            // Step it
                            thread.step(ctx, &mut fuel)?;
                        } else {
                            break;
                        }

                        // Handle fuel interruptions
                        if fuel.is_interrupted() {
                            break;
                        }
                    }

                    // Take the thread result and print any errors
                    thread.take_return::<()>(ctx)??;

                    Ok(())
                });
                if let Err(e) = result {
                    tracing::error!("{e}");
                }
            });
        });
    }
}

pub struct LuaSingletons {
    singletons: Rc<AtomicCell<HashMap<usize, Box<dyn Any>>>>,
}
impl Default for LuaSingletons {
    fn default() -> Self {
        Self {
            singletons: Rc::new(AtomicCell::new(HashMap::default())),
        }
    }
}
impl LuaSingletons {
    /// Fetch a lua singleton, initializing it if it has not yet been created.
    ///
    /// The singleton is defined by a function pointer that returns a stashable value.
    fn get<
        'gc,
        S: Fetchable<'gc, Fetched = T> + 'static,
        T: Stashable<'gc, Stashed = S> + Clone + Copy + 'gc,
    >(
        &self,
        ctx: Context<'gc>,
        singleton: fn(Context<'gc>) -> T,
    ) -> T {
        let map = self.singletons.borrow_mut();
        let id = singleton as usize;
        if let Some(entry) = map.get(&id) {
            let stashed = entry.downcast_ref::<S>().expect(
                "Encountered two functions with different return types and \
                the same function pointer.",
            );
            ctx.state.registry.fetch(stashed)
        } else {
            drop(map); // Make sure we don't deadlock
            let v = singleton(ctx);
            let stashed = ctx.state.registry.stash(&ctx, v);
            self.singletons.borrow_mut().insert(id, Box::new(stashed));
            v
        }
    }
}

/// A type data that can be used to specify a custom metatable to use for the type when it is
/// used in an [`EcsRef`] in the lua API.
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
struct SchemaLuaEcsRefMetatable(pub fn(piccolo::Context) -> piccolo::Table);

/// A reference to an ECS-compatible value.
#[derive(Clone)]
pub struct EcsRef {
    /// The kind of reference.
    pub data: EcsRefData,
    /// The path to the desired field.
    pub path: Ustr,
}

impl EcsRef {
    /// Get the function that may be used to retrieve the metatable to use for this [`EcsRef`].
    pub fn metatable_fn(&self) -> fn(piccolo::Context) -> piccolo::Table {
        (|| {
            let data = self.data.borrow();
            let field = data
                .schema_ref()?
                .access()
                .field_path(FieldPath(self.path))?;
            let metatable_fn = field
                .into_schema_ref()
                .schema()
                .type_data
                .get::<SchemaLuaEcsRefMetatable>()?;
            Some(metatable_fn.0)
        })()
        .unwrap_or(bindings::ecsref::metatable)
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

pub enum EcsRefBorrowKind<'a> {
    Resource(AtomicSchemaRef<'a>),
    Component(ComponentBorrow<'a>),
    Free(Ref<'a, SchemaBox>),
    Asset(Option<MappedRef<'a, Cid, LoadedAsset, SchemaBox>>),
}

impl EcsRefBorrowKind<'_> {
    /// Will return none if the value does not exist, such as an unloaded asset or a component
    /// that is not set for a given entity.
    pub fn schema_ref(&self) -> Option<SchemaRef> {
        match self {
            EcsRefBorrowKind::Resource(r) => Some(r.schema_ref()),
            EcsRefBorrowKind::Component(c) => c.borrow.get_ref(c.entity),
            EcsRefBorrowKind::Free(f) => Some(f.as_ref()),
            EcsRefBorrowKind::Asset(a) => a.as_ref().map(|x| x.as_ref()),
        }
    }
}

pub struct ComponentBorrow<'a> {
    pub borrow: Ref<'a, UntypedComponentStore>,
    pub entity: Entity,
}

pub struct ComponentBorrowMut<'a> {
    pub borrow: RefMut<'a, UntypedComponentStore>,
    pub entity: Entity,
}

pub enum EcsRefBorrowMutKind<'a> {
    Resource(AtomicSchemaRefMut<'a>),
    Component(ComponentBorrowMut<'a>),
    Free(RefMut<'a, SchemaBox>),
    Asset(Option<MappedRefMut<'a, Cid, LoadedAsset, SchemaBox>>),
}

impl EcsRefBorrowMutKind<'_> {
    pub fn schema_ref_mut(&mut self) -> Option<SchemaRefMut> {
        match self {
            EcsRefBorrowMutKind::Resource(r) => Some(r.schema_ref_mut()),
            EcsRefBorrowMutKind::Component(c) => c.borrow.get_ref_mut(c.entity),
            EcsRefBorrowMutKind::Free(f) => Some(f.as_mut()),
            EcsRefBorrowMutKind::Asset(a) => a.as_mut().map(|x| x.as_mut()),
        }
    }
}

impl EcsRefData {
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

    /// Mutably borrow the ref.
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

/// A resource ref.
#[derive(Clone)]
pub struct ComponentRef {
    /// The component store.
    pub store: UntypedAtomicComponentStore,
    /// The entity to get the component data for.
    pub entity: Entity,
}

/// An asset ref.
#[derive(Clone)]
pub struct AssetRef {
    /// The asset server handle.
    pub server: AssetServer,
    /// The kind of asset we are referencing.
    pub handle: UntypedHandle,
}
