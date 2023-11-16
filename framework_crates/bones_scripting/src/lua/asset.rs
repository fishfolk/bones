use std::sync::Arc;

use bevy_tasks::ThreadExecutor;
use piccolo::{
    AnyCallback, AnyUserData, CallbackReturn, Closure, Context, Executor, StashedClosure, Table,
};
use send_wrapper::SendWrapper;

use crate::prelude::*;

/// A Lua script asset.
///
/// Lua scripts can be run easily with the [`LuaEngine`] resource.
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
#[type_data(asset_loader("lua", LuaScriptLoader))]
pub struct LuaScript {
    /// The lua source for the script.
    pub source: String,
}

/// Asset loader for [`LuaScript`].
struct LuaScriptLoader;
impl AssetLoader for LuaScriptLoader {
    fn load(
        &self,
        _ctx: AssetLoadCtx,
        bytes: &[u8],
    ) -> futures::future::Boxed<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let script = LuaScript {
                source: String::from_utf8(bytes)?,
            };
            Ok(SchemaBox::new(script))
        })
    }
}

/// A lua plugin asset.
///
/// This differs from [`LuaScript`] in that loaded [`LuaPlugin`]s will be automatically registered
/// and run by the bones framework and [`LuaScript`] must be manually triggered by your systems.
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
#[type_data(asset_loader("plugin.lua", LuaPluginLoader))]
pub struct LuaPlugin {
    /// The lua source of the script.
    pub source: String,
    /// The lua closures, registered by the script, to run in different system stages.
    pub systems: LuaPluginSystemsCell,
}
impl Drop for LuaPlugin {
    fn drop(&mut self) {
        match std::mem::take(&mut *self.systems.borrow_mut()) {
            LuaPluginSystemsState::NotLoaded => (),
            // Systems, due to the `SendWrapper` for the lua `Closures` must be dropped on
            // the lua executor thread
            LuaPluginSystemsState::Loaded { systems, executor } => {
                executor.spawn(async move { drop(systems) }).detach();
            }
            LuaPluginSystemsState::Unloaded => (),
        }
    }
}

impl LuaPlugin {
    /// Whether or not the plugin has loaded it's systems.
    pub fn has_loaded(&self) -> bool {
        matches!(*self.systems.borrow(), LuaPluginSystemsState::Loaded { .. })
    }

    /// Load the lua plugin's systems.
    pub fn load(
        &self,
        executor: Arc<ThreadExecutor<'static>>,
        lua: &mut piccolo::Lua,
    ) -> Result<(), anyhow::Error> {
        if !self.has_loaded() {
            *self.systems.borrow_mut() = LuaPluginSystemsState::Loaded {
                systems: SendWrapper::new(default()),
                executor,
            };
            self.load_impl(lua)
        } else {
            Ok(())
        }
    }
    fn load_impl(&self, lua: &mut piccolo::Lua) -> Result<(), anyhow::Error> {
        let executor = lua.try_run(|ctx| {
            let env = ctx.singletons().get(ctx, super::bindings::env);

            let session_var = AnyUserData::new_static(&ctx, self.systems.clone());
            session_var.set_metatable(&ctx, Some(ctx.singletons().get(ctx, session_metatable)));
            env.set(ctx, "session", session_var)?;

            let closure = Closure::load_with_env(ctx, self.source.as_bytes(), env)?;
            let ex = Executor::start(ctx, closure.into(), ());
            Ok(ctx.state.registry.stash(&ctx, ex))
        })?;

        lua.execute::<()>(&executor)?;

        Ok(())
    }
}

fn session_metatable(ctx: Context) -> Table {
    let metatable = Table::new(&ctx);

    metatable
        .set(
            ctx,
            "__tostring",
            AnyCallback::from_fn(&ctx, |ctx, _fuel, mut stack| {
                stack.push_front(
                    piccolo::String::from_static(&ctx, "Session { add_system_to_stage }").into(),
                );
                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();
    metatable
        .set(
            ctx,
            "__newindex",
            ctx.singletons().get(ctx, super::bindings::no_newindex),
        )
        .unwrap();

    let add_startup_system_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let (this, closure): (AnyUserData, Closure) = stack.consume(ctx)?;
            let this = this.downcast_static::<LuaPluginSystemsCell>()?;

            let mut systems = this.borrow_mut();
            systems
                .as_loaded_mut()
                .startup
                .push((false, ctx.state.registry.stash(&ctx, closure)));

            Ok(CallbackReturn::Return)
        }),
    );
    let add_system_to_stage_callback = ctx.state.registry.stash(
        &ctx,
        AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
            let (this, stage, closure): (AnyUserData, AnyUserData, Closure) = stack.consume(ctx)?;
            let this = this.downcast_static::<LuaPluginSystemsCell>()?;
            let stage = stage.downcast_static::<CoreStage>()?;

            let mut systems = this.borrow_mut();
            systems
                .as_loaded_mut()
                .core_stages
                .push((*stage, ctx.state.registry.stash(&ctx, closure)));

            Ok(CallbackReturn::Return)
        }),
    );

    metatable
        .set(
            ctx,
            "__index",
            AnyCallback::from_fn(&ctx, move |ctx, _fuel, mut stack| {
                let (_this, key): (piccolo::Value, piccolo::String) = stack.consume(ctx)?;

                #[allow(clippy::single_match)]
                match key.as_bytes() {
                    b"add_system_to_stage" => {
                        stack.push_front(
                            ctx.state
                                .registry
                                .fetch(&add_system_to_stage_callback)
                                .into(),
                        );
                    }
                    b"add_startup_system" => {
                        stack.push_front(
                            ctx.state
                                .registry
                                .fetch(&add_startup_system_callback)
                                .into(),
                        );
                    }
                    _ => (),
                }

                Ok(CallbackReturn::Return)
            }),
        )
        .unwrap();

    metatable
}

/// An atomic cell containing the [`LuaPluginSystemsState`].
pub type LuaPluginSystemsCell = Arc<AtomicCell<LuaPluginSystemsState>>;

/// The load state of the [`LuaPluginSystems
#[derive(Default)]
pub enum LuaPluginSystemsState {
    /// The systems have not been loaded yet.
    #[default]
    NotLoaded,
    /// The systems have been loaded.
    Loaded {
        systems: SendWrapper<LuaPluginSystems>,
        executor: Arc<ThreadExecutor<'static>>,
    },
    /// The [`LuaPlugin`] has been dropped and it's systems have been unloaded.
    Unloaded,
}

impl LuaPluginSystemsState {
    /// Helper to get the loaded systems.
    pub fn as_loaded(&self) -> &LuaPluginSystems {
        match self {
            LuaPluginSystemsState::NotLoaded => panic!("Not loaded"),
            LuaPluginSystemsState::Loaded { systems, .. } => systems,
            LuaPluginSystemsState::Unloaded => panic!("Not loaded"),
        }
    }

    /// Helper to get the loaded systems mutably.
    pub fn as_loaded_mut(&mut self) -> &mut LuaPluginSystems {
        match self {
            LuaPluginSystemsState::NotLoaded => panic!("Not loaded"),
            LuaPluginSystemsState::Loaded { systems, .. } => &mut *systems,
            LuaPluginSystemsState::Unloaded => panic!("Not loaded"),
        }
    }
}

/// The ID of a system stage.
pub type SystemStageId = Ulid;

/// The systems that have been registered by a lua plugin.
#[derive(Default)]
pub struct LuaPluginSystems {
    /// Startup systems. The bool indicates whether the system has been run yet.
    pub startup: Vec<(bool, StashedClosure)>,
    /// Systems that run in the core stages.
    pub core_stages: Vec<(CoreStage, StashedClosure)>,
}

struct LuaPluginLoader;
impl AssetLoader for LuaPluginLoader {
    fn load(
        &self,
        _ctx: AssetLoadCtx,
        bytes: &[u8],
    ) -> futures::future::Boxed<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let script = LuaPlugin {
                source: String::from_utf8(bytes)?,
                systems: Arc::new(AtomicCell::new(default())),
            };
            Ok(SchemaBox::new(script))
        })
    }
}
