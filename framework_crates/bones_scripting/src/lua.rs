use crate::prelude::*;
use append_only_vec::AppendOnlyVec;
use bevy_tasks::{ComputeTaskPool, TaskPool, ThreadExecutor};
use bones_asset::dashmap::mapref::one::{MappedRef, MappedRefMut};
use bones_lib::ecs::utils::*;
use parking_lot::Mutex;
use piccolo::{
    AnyUserData, Closure, Context, Fuel, Lua, ProtoCompileError, StaticCallback, StaticClosure,
    StaticTable, Table, Thread, ThreadMode, Value,
};
use send_wrapper::SendWrapper;
use std::{rc::Rc, sync::Arc};

#[macro_use]
mod freeze;
use freeze::*;

mod asset;
pub use asset::*;

pub mod bindings;

/// Install the scripting plugin.
pub fn lua_game_plugin(game: &mut Game) {
    // Register asset type.
    LuaScript::schema();

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
    data: LuaData,
    /// Cache of the content IDs of loaded scripts, and their compiled lua closures.
    compiled_scripts: Mutex<HashMap<Cid, StaticClosure>>,
}

trait CtxExt {
    fn luadata(&self) -> &LuaData;
}
impl CtxExt for piccolo::Context<'_> {
    fn luadata(&self) -> &LuaData {
        let Value::UserData(data) = self.state.globals.get(*self, "luadata") else {
            unreachable!();
        };
        data.downcast_static::<LuaData>().unwrap()
    }
}

impl Default for EngineState {
    fn default() -> Self {
        // Initialize an empty lua engine and our lua data.
        let mut lua = Lua::core();
        lua.try_run(|ctx| {
            ctx.state.globals.set(
                ctx,
                "luadata",
                AnyUserData::new_static(&ctx, LuaData::default()),
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
                    let env = ctx
                        .state
                        .registry
                        .fetch(&self.state.data.table(ctx, bindings::env));

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
                    let world = world.into_userdata(
                        ctx,
                        ctx.state
                            .registry
                            .fetch(&self.state.data.table(ctx, bindings::world_metatable)),
                    );
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

/// Static lua tables and callbacks
pub struct LuaData {
    callbacks: AppendOnlyVec<(fn(Context) -> StaticCallback, StaticCallback)>,
    tables: AppendOnlyVec<(fn(Context) -> StaticTable, StaticTable)>,
}
impl Default for LuaData {
    fn default() -> Self {
        Self {
            callbacks: AppendOnlyVec::new(),
            tables: AppendOnlyVec::new(),
        }
    }
}

impl LuaData {
    /// Get a table from the store, initializing it if necessary.
    pub fn table(&self, ctx: Context, f: fn(Context) -> StaticTable) -> StaticTable {
        for (other_f, table) in self.tables.iter() {
            if *other_f == f {
                return table.clone();
            }
        }
        let new_table = f(ctx);
        self.tables.push((f, new_table.clone()));
        new_table
    }

    /// Get a callback from the store, initializing if necessary.
    pub fn callback(&self, ctx: Context, f: fn(Context) -> StaticCallback) -> StaticCallback {
        for (other_f, callback) in self.callbacks.iter() {
            if *other_f == f {
                return callback.clone();
            }
        }
        let new_callback = f(ctx);
        self.callbacks.push((f, new_callback.clone()));
        new_callback
    }
}

/// A reference to an ECS-compatible value.
#[derive(Clone)]
pub struct EcsRef {
    /// The kind of reference.
    pub data: EcsRefData,
    /// The path to the desired field.
    pub path: Ustr,
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
    pub fn access(&self) -> Option<SchemaRefAccess> {
        match self {
            EcsRefBorrowKind::Resource(r) => Some(r.as_ref().access()),
            EcsRefBorrowKind::Component(c) => c.borrow.get_ref(c.entity).map(|x| x.access()),
            EcsRefBorrowKind::Free(f) => Some(f.as_ref().access()),
            EcsRefBorrowKind::Asset(a) => a.as_ref().map(|x| x.as_ref().access()),
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
    pub fn access_mut(&mut self) -> Option<SchemaRefMutAccess> {
        match self {
            EcsRefBorrowMutKind::Resource(r) => Some(r.access_mut()),
            EcsRefBorrowMutKind::Component(c) => {
                c.borrow.get_ref_mut(c.entity).map(|x| x.into_access_mut())
            }
            EcsRefBorrowMutKind::Free(f) => Some(f.as_mut().into_access_mut()),
            EcsRefBorrowMutKind::Asset(a) => a.as_mut().map(|x| x.as_mut().into_access_mut()),
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
