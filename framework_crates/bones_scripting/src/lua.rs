use crate::prelude::*;

use bevy_tasks::{ComputeTaskPool, TaskPool, ThreadExecutor};
use bones_asset::dashmap::mapref::one::{MappedRef, MappedRefMut};
use bones_lib::ecs::utils::*;

use parking_lot::Mutex;
pub use piccolo;
use piccolo::{
    compiler::{LineNumber, ParseError},
    registry::{Fetchable, Stashable},
    Closure, Context, Executor, FromValue, Lua, PrototypeError, StashedClosure, Table, UserData,
    Value,
};
use send_wrapper::SendWrapper;
use std::{any::Any, rc::Rc, sync::Arc};

#[macro_use]
mod freeze;
use freeze::*;

mod asset;
pub use asset::*;

mod ext;
pub use ext::*;

pub mod bindings;

/// Install the lua scripting plugin.
pub fn lua_game_plugin(game: &mut Game) {
    // Register asset type.
    LuaScript::register_schema();

    // Add `SchemaLuaMetatable` type data for common types.
    bindings::register_lua_typedata();

    // Initialize the lua engine resource.
    game.init_shared_resource::<LuaEngine>();
}

/// A [`SessionPlugin] that will run the provided lua plugins
pub struct LuaPluginLoaderSessionPlugin(pub Arc<Vec<Handle<LuaPlugin>>>);

/// Resource containing the lua plugins that have been installed in this session.
#[derive(HasSchema, Deref, DerefMut, Default, Clone)]
pub struct LuaPlugins(pub Arc<Vec<Handle<LuaPlugin>>>);

impl SessionPlugin for LuaPluginLoaderSessionPlugin {
    fn install(self, session: &mut Session) {
        session.world.insert_resource(LuaPlugins(self.0));

        for lua_stage in [
            CoreStage::First,
            CoreStage::PreUpdate,
            CoreStage::Update,
            CoreStage::PostUpdate,
            CoreStage::Last,
        ] {
            session.stages.add_system_to_stage(
                lua_stage,
                move |engine: Res<LuaEngine>,
                      asset_server: Res<AssetServer>,
                      lua_plugins: Res<LuaPlugins>,
                      world: &World| {
                    engine.exec(|lua| {
                        Frozen::<Freeze![&'freeze World]>::in_scope(world, |world| {
                            lua.enter(|ctx| {
                                let env = ctx.singletons().get(ctx, bindings::env);
                                let worldref = WorldRef(world);
                                worldref.add_to_env(ctx, env);
                            });

                            for plugin_handle in lua_plugins.iter() {
                                let Some(plugin) = asset_server.try_get(*plugin_handle) else {
                                    return;
                                };
                                let plugin = plugin.unwrap();

                                // Load the plugin if necessary
                                if !plugin.has_loaded() {
                                    if let Err(e) = plugin.load(engine.executor.clone(), lua) {
                                        tracing::error!("Error loading lua plugin: {e}");
                                    }
                                }

                                let mut systems = plugin.systems.borrow_mut();
                                let systems = systems.as_loaded_mut();

                                for (has_run, closure) in &mut systems.startup {
                                    if !*has_run {
                                        let executor = lua.enter(|ctx| {
                                            let closure = ctx.registry().fetch(closure);
                                            let ex = Executor::start(ctx, closure.into(), ());
                                            ctx.registry().stash(&ctx, ex)
                                        });
                                        if let Err(e) = lua.execute::<()>(&executor) {
                                            tracing::error!("Error running lua plugin system: {e}");
                                        }

                                        *has_run = true;
                                    }
                                }

                                for (stage, closure) in &systems.core_stages {
                                    if stage == &lua_stage {
                                        let executor = lua.enter(|ctx| {
                                            let closure = ctx.registry().fetch(closure);
                                            let ex = Executor::start(ctx, closure.into(), ());
                                            ctx.registry().stash(&ctx, ex)
                                        });
                                        if let Err(e) = lua.execute::<()>(&executor) {
                                            tracing::error!("Error running lua plugin system: {e}");
                                        }
                                    }
                                }
                            }
                        })
                    });
                },
            );
        }
    }
}

/// A frozen reference to the ECS [`World`].
///
// This type can be converted into lua userdata for accessing the world from lua.
#[derive(Deref, DerefMut, Clone)]
pub struct WorldRef(Frozen<Freeze![&'freeze World]>);
impl Default for WorldRef {
    fn default() -> Self {
        Self(Frozen::new())
    }
}
impl<'gc> FromValue<'gc> for &'gc WorldRef {
    fn from_value(_ctx: Context<'gc>, value: Value<'gc>) -> Result<Self, piccolo::TypeError> {
        value.as_static_user_data::<WorldRef>()
    }
}

impl WorldRef {
    /// Convert this [`WorldRef`] into a Lua userdata.
    pub fn into_userdata(self, ctx: Context<'_>) -> UserData<'_> {
        let data = UserData::new_static(&ctx, self);
        data.set_metatable(
            &ctx,
            Some(ctx.singletons().get(ctx, bindings::world::metatable)),
        );
        data
    }

    /// Add this world
    fn add_to_env<'gc>(&self, ctx: Context<'gc>, env: Table<'gc>) {
        ctx.globals()
            .set(ctx, "world", self.clone().into_userdata(ctx))
            .unwrap();
        for (name, metatable) in [
            ("world", bindings::world::metatable as fn(Context) -> Table),
            ("components", bindings::components::metatable),
            ("resources", bindings::resources::metatable),
            ("assets", bindings::assets::metatable),
        ] {
            let data = UserData::new_static(&ctx, self.clone());
            data.set_metatable(&ctx, Some(ctx.singletons().get(ctx, metatable)));
            env.set(ctx, name, data).unwrap();
        }
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

/// Internal state for [`LuaEngine`].
struct EngineState {
    /// The Lua engine.
    lua: Mutex<Lua>,
    /// Persisted lua data we need stored in Rust, such as the environment table, world
    /// metatable, etc.
    data: LuaSingletons,
    /// Cache of the content IDs of loaded scripts, and their compiled lua closures.
    compiled_scripts: Mutex<HashMap<Cid, StashedClosure>>,
}

// TODO: Don't Use Function Pointers to Index Lua Singletons.
// Unfortunately, function pointers with different signatures may be unified by
// LLVM when activating, for intance, LTO, and that can cause unexpected behavior
// when using them as indexes into a HashMap for singletons.
/// Struct for accessing and initializing lua singletons.
///
/// This is stored in a lua global and accessed conveniently through our [`CtxExt`] trait
/// so that we can easily initialize lua tables and callbacks throughout our lua bindings.
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
            ctx.registry().fetch(stashed)
        } else {
            drop(map); // Make sure we don't deadlock
            let v = singleton(ctx);
            let stashed = ctx.registry().stash(&ctx, v);
            self.singletons.borrow_mut().insert(id, Box::new(stashed));
            v
        }
    }
}

impl Default for EngineState {
    fn default() -> Self {
        // Initialize an empty lua engine and our lua data.
        let mut lua = Lua::core();
        lua.try_enter(|ctx| {
            // Insert our lua singletons.
            ctx.globals().set(
                ctx,
                "luasingletons",
                UserData::new_static(&ctx, LuaSingletons::default()),
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

                let ticker = (*executor).ticker().unwrap();
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
                let worldref = WorldRef(world);

                let executor = lua.try_enter(|ctx| {
                    // Fetch the env table
                    let env = self.state.data.get(ctx, bindings::env);

                    // Compile the script
                    let closure = worldref.with(|world| {
                        let asset_server = world.resource::<AssetServer>();
                        let cid = *asset_server
                            .store
                            .asset_ids
                            .get(&script.untyped())
                            .ok_or_else(|| {
                                tracing::warn!("Script asset not loaded.");
                                PrototypeError::Parser(ParseError {
                                    kind: piccolo::compiler::ParseErrorKind::EndOfStream {
                                        expected: None,
                                    },
                                    line_number: LineNumber(0),
                                })
                            })?;

                        let mut compiled_scripts = self.state.compiled_scripts.lock();
                        let closure = compiled_scripts.get(&cid);

                        Ok::<_, PrototypeError>(match closure {
                            Some(closure) => ctx.registry().fetch(closure),
                            None => {
                                let asset = asset_server.store.assets.get(&cid).unwrap();
                                let source = &asset.data.cast_ref::<LuaScript>().source;
                                // TODO: Provide a meaningfull name to loaded scripts.
                                let closure =
                                    Closure::load_with_env(ctx, None, source.as_bytes(), env)?;
                                compiled_scripts.insert(cid, ctx.registry().stash(&ctx, closure));

                                closure
                            }
                        })
                    })?;

                    // Insert the world ref into the global scope
                    worldref.add_to_env(ctx, env);

                    let ex = Executor::start(ctx, closure.into(), ());
                    let ex = ctx.registry().stash(&ctx, ex);
                    Ok(ex)
                });

                if let Err(e) = executor.and_then(|ex| lua.execute::<()>(&ex)) {
                    tracing::error!("{e}");
                }
            });
        });
    }
}
