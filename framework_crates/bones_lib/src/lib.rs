//! The core bones library.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

#[doc(inline)]
pub use bones_ecs as ecs;

/// Bones lib prelude
pub mod prelude {
    pub use crate::{
        ecs::prelude::*, instant::Instant, time::*, Game, GamePlugin, Session, SessionCommand,
        SessionOptions, SessionPlugin, SessionRunner, Sessions,
    };
}

pub use instant;
pub mod time;

use std::{collections::VecDeque, fmt::Debug, sync::Arc};

use crate::prelude::*;
/// A bones game. This includes all of the game worlds, and systems.
#[derive(Deref, DerefMut)]
pub struct Session {
    /// The ECS world for the core.
    pub world: World,
    /// The system stages.
    #[deref]
    pub stages: SystemStages,
    /// Whether or not this session should have it's systems run.
    pub active: bool,
    /// Whether or not this session should be rendered.
    pub visible: bool,
    /// The priority of this session relative to other sessions in the [`Game`].
    pub priority: i32,
    /// The session runner to use for this session.
    pub runner: Box<dyn SessionRunner>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("world", &self.world)
            .field("stages", &self.stages)
            .field("active", &self.active)
            .field("visible", &self.visible)
            .field("priority", &self.priority)
            .field("runner", &"SessionRunner")
            .finish()
    }
}

impl Session {
    /// Create an empty [`Session`].
    pub fn new() -> Self {
        Self {
            world: {
                let mut w = World::default();
                w.init_resource::<Time>();
                w
            },
            stages: default(),
            active: true,
            visible: true,
            priority: 0,
            runner: Box::<DefaultSessionRunner>::default(),
        }
    }

    /// Install a plugin.
    pub fn install_plugin(&mut self, plugin: impl SessionPlugin) -> &mut Self {
        plugin.install(self);
        self
    }

    /// Snapshot the world state.
    ///
    /// This is the same as `core.world.clone()`, but it is more explicit.
    pub fn snapshot(&self) -> World {
        self.world.clone()
    }

    /// Restore the world state.
    ///
    /// Re-sets the world state to that of the provided `world`, which may or may not have been
    /// created with [`snapshot()`][Self::snapshot].
    ///
    /// This is the same as doing an [`std::mem::swap`] on `self.world`, but it is more explicit.
    pub fn restore(&mut self, world: &mut World) {
        std::mem::swap(&mut self.world, world)
    }

    /// Set the session runner for this session.
    pub fn set_session_runner(&mut self, runner: Box<dyn SessionRunner>) {
        self.runner = runner;
    }

    /// Provides an interface for resetting various internal parts of the Session.
    /// Note this does not fully reset the entire Session.
    pub fn reset_internals(
        &mut self,
        reset_components: bool,
        reset_entities: bool,
        reset_systems: bool,
    ) {
        self.world.reset_internals(reset_components, reset_entities);
        if reset_systems {
            self.reset_remove_all_systems();
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for plugins that can be installed into a [`Session`].
pub trait SessionPlugin {
    /// Install the plugin into the [`Session`].
    fn install(self, session: &mut Session);
}
impl<F: FnOnce(&mut Session)> SessionPlugin for F {
    fn install(self, session: &mut Session) {
        (self)(session)
    }
}

/// Trait for plugins that can be installed into a [`Game`].
pub trait GamePlugin {
    /// Install the plugin into the [`Game`].
    fn install(self, game: &mut Game);
}
impl<F: FnOnce(&mut Game)> GamePlugin for F {
    fn install(self, game: &mut Game) {
        (self)(game)
    }
}

/// A session runner is in charge of advancing a [`Session`] simulation.
pub trait SessionRunner: Sync + Send + 'static {
    /// Step the simulation once.
    ///
    /// It is the responsibility of the session runner to update the [`Time`] resource if necessary.
    ///
    /// If no special behavior is desired, the simplest session runner, and the one that is
    /// implemented by [`DefaultSessionRunner`] is as follows:
    ///
    /// ```
    /// # use bones_lib::prelude::*;
    /// # struct Example;
    /// # impl SessionRunner for Example {
    /// fn step(&mut self, now: Instant, world: &mut World, stages: &mut SystemStages) {
    ///     world.resource_mut::<Time>().update_with_instant(now);
    ///     stages.run(world);
    /// }
    /// fn restart_session(&mut self) {}
    /// fn disable_local_input(&mut self, disable_input: bool) {}
    /// # }
    /// ```
    fn step(&mut self, now: Instant, world: &mut World, stages: &mut SystemStages);

    /// Restart Session Runner. This should reset accumulated time, inputs, etc.
    ///
    /// The expectation is that current players using it may continue to, so something like a network
    /// socket or player info should persist.
    fn restart_session(&mut self);

    /// Disable the capture of local input by this session.
    fn disable_local_input(&mut self, input_disabled: bool);
}

/// The default [`SessionRunner`], which just runs the systems once every time it is run.
#[derive(Default)]
pub struct DefaultSessionRunner;
impl SessionRunner for DefaultSessionRunner {
    fn step(&mut self, now: instant::Instant, world: &mut World, stages: &mut SystemStages) {
        world.resource_mut::<Time>().update_with_instant(now);
        stages.run(world)
    }

    // This is a no-op as no state, but implemented this way in case that changes later.
    #[allow(clippy::default_constructed_unit_structs)]
    fn restart_session(&mut self) {
        *self = DefaultSessionRunner::default();
    }

    // `DefaultSessionRunner` does not collect input so this impl is not relevant.
    fn disable_local_input(&mut self, _input_disabled: bool) {}
}

/// The [`Game`] encompasses a complete bones game's logic, independent of the renderer and IO
/// implementations.
///
/// Games are made up of one or more [`Session`]s, each of which contains it's own [`World`] and
/// [`SystemStages`]. These different sessions can be used for parts of the game with independent
/// states, such as the main menu and the gameplay.
#[derive(Default)]
pub struct Game {
    /// The sessions that make up the game.
    pub sessions: Sessions,
    /// The collection of systems that are associated to the game itself, and not a specific
    /// session.
    pub systems: GameSystems,
    /// List of sorted session keys.
    ///
    /// These are only guaranteed to be sorted and up-to-date immediately after calling
    /// [`Game::step()`].
    pub sorted_session_keys: Vec<Ustr>,
    /// Collection of resources that will have a shared instance of each be inserted into each
    /// session automatically.
    pub shared_resources: Vec<AtomicUntypedResource>,
}

impl Game {
    /// Create an empty game with an asset server.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a [`GamePlugin`].
    pub fn install_plugin<P: GamePlugin>(&mut self, plugin: P) -> &mut Self {
        plugin.install(self);
        self
    }
    #[track_caller]
    /// Get the shared resource of a given type out of this [`Game`]s shared resources.
    pub fn shared_resource<T: HasSchema>(&self) -> Option<Ref<T>> {
        let res = self
            .shared_resources
            .iter()
            .find(|x| x.schema() == T::schema())?;
        let borrow = res.borrow();

        if borrow.is_some() {
            // SOUND: We know the type matches T
            Some(Ref::map(borrow, |b| unsafe {
                b.as_ref().unwrap().as_ref().cast_into_unchecked()
            }))
        } else {
            None
        }
    }

    #[track_caller]
    /// Get the shared resource of a given type out of this [`Game`]s shared resources.
    pub fn shared_resource_mut<T: HasSchema>(&self) -> Option<RefMut<T>> {
        let res = self
            .shared_resources
            .iter()
            .find(|x| x.schema() == T::schema())?;
        let borrow = res.borrow_mut();

        if borrow.is_some() {
            // SOUND: We know the type matches T
            Some(RefMut::map(borrow, |b| unsafe {
                b.as_mut().unwrap().as_mut().cast_into_mut_unchecked()
            }))
        } else {
            None
        }
    }

    /// Get the shared resource cell of a given type out of this [`Game`]s shared resources.
    pub fn shared_resource_cell<T: HasSchema>(&self) -> Option<AtomicResource<T>> {
        let res = self
            .shared_resources
            .iter()
            .find(|x| x.schema() == T::schema())?;
        Some(AtomicResource::from_untyped(res.clone()).unwrap())
    }

    /// Initialize a resource that will be shared across game sessions using it's [`Default`] value
    /// if it is not already initialized, and borrow it for modification.
    pub fn init_shared_resource<T: HasSchema + Default>(&mut self) -> RefMut<T> {
        if !self
            .shared_resources
            .iter()
            .any(|x| x.schema() == T::schema())
        {
            self.insert_shared_resource(T::default());
        }
        self.shared_resource_mut::<T>().unwrap()
    }

    /// Insert a resource that will be shared across all game sessions.
    ///
    /// > **Note:** This resource will only be visible in sessions that have not already
    /// > initialized or access a resource of the same type locally.
    pub fn insert_shared_resource<T: HasSchema>(&mut self, resource: T) {
        // Update an existing resource of the same type.
        for r in &mut self.shared_resources {
            if r.schema() == T::schema() {
                let mut borrow = r.borrow_mut();

                if let Some(b) = borrow.as_mut().as_mut() {
                    *b.cast_mut() = resource;
                } else {
                    *borrow = Some(SchemaBox::new(resource))
                }
                return;
            }
        }

        // Or insert a new resource if we couldn't find one
        self.shared_resources
            .push(Arc::new(UntypedResource::new(SchemaBox::new(resource))));
    }

    // /// Remove a shared resource, if it is present in the world.
    // /// # Panics
    // /// Panics if the resource is set and it's cell has another handle to it and cannot be
    // /// unwrapped.
    // pub fn remove_shared_resource<T: HasSchema>(&mut self) -> Option<T> {
    //     self.shared_resources
    //         .iter()
    //         .position(|x| x.schema() == T::schema())
    //         .map(|idx| {
    //             self.shared_resources
    //                 .remove(idx)
    //                 .try_into_inner()
    //                 .unwrap()
    //                 .into_inner()
    //         })
    // }

    /// Step the game simulation.
    pub fn step(&mut self, now: instant::Instant) {
        // Pull out the game systems so that we can run them on the game
        let mut game_systems = std::mem::take(&mut self.systems);

        // Run game startup systems
        if !game_systems.has_run_startup {
            for system in &mut game_systems.startup {
                system(self);
            }
            game_systems.has_run_startup = true;
        }

        // Run the before systems
        for system in &mut game_systems.before {
            system(self)
        }

        // Sort session keys by priority
        self.sorted_session_keys.clear();
        self.sorted_session_keys.extend(self.sessions.map.keys());
        self.sorted_session_keys
            .sort_by_key(|name| self.sessions.map.get(name).unwrap().priority);

        // For every session
        for session_name in self.sorted_session_keys.clone() {
            // Extract the current session
            let Some(mut current_session) = self.sessions.map.remove(&session_name) else {
                // This may happen if the session was deleted by another session.
                continue;
            };

            // If this session is active
            let options = if current_session.active {
                // Run any before session game systems
                if let Some(systems) = game_systems.before_session.get_mut(&session_name) {
                    for system in systems {
                        system(self)
                    }
                }

                // Make sure session contains all of the shared resources
                for r in &self.shared_resources {
                    if !current_session
                        .world
                        .resources
                        .untyped()
                        .contains_cell(r.schema().id())
                    {
                        current_session
                            .world
                            .resources
                            .untyped()
                            .insert_cell(r.clone())
                            .unwrap();
                    }
                }

                // Insert the session options
                current_session.world.resources.insert(SessionOptions {
                    active: true,
                    delete: false,
                    visible: current_session.visible,
                });

                // Insert the other sessions into the current session's world
                {
                    let mut sessions = current_session.world.resource_mut::<Sessions>();
                    std::mem::swap(&mut *sessions, &mut self.sessions);
                }

                // Step the current session's simulation using it's session runner
                current_session.runner.step(
                    now,
                    &mut current_session.world,
                    &mut current_session.stages,
                );

                // Pull the sessions back out of the world
                {
                    let mut sessions = current_session.world.resource_mut::<Sessions>();
                    std::mem::swap(&mut *sessions, &mut self.sessions);
                }

                // Pull the current session options back out of the world.
                *current_session.world.resource::<SessionOptions>()
            } else {
                SessionOptions {
                    active: false,
                    visible: current_session.visible,
                    delete: false,
                }
            };

            // Delete the session
            if options.delete {
                let session_idx = self
                    .sorted_session_keys
                    .iter()
                    .position(|x| x == &session_name)
                    .unwrap();
                self.sorted_session_keys.remove(session_idx);

            // Update session options
            } else {
                current_session.active = options.active;
                current_session.visible = options.visible;

                // Insert the current session back into the session list
                self.sessions.map.insert(session_name, current_session);
            }

            // Run any after session game systems
            if let Some(systems) = game_systems.after_session.get_mut(&session_name) {
                for system in systems {
                    system(self)
                }
            }
        }

        // Execute Session Commands
        {
            let mut session_commands: VecDeque<Box<SessionCommand>> = default();
            std::mem::swap(&mut session_commands, &mut self.sessions.commands);
            for command in session_commands.drain(..) {
                command(&mut self.sessions);
            }
        }

        // Run after systems
        for system in &mut game_systems.after {
            system(self)
        }

        // Replace the game systems
        self.systems = game_systems;

        // Make sure sorted session keys does not include sessions that were deleted during
        // execution by other sessions.
        self.sorted_session_keys
            .retain(|x| self.sessions.iter().any(|(id, _)| id == x));
    }
}

/// A system that runs directly on a [`Game`] instead of in a specific [`Session`].
pub type GameSystem = Box<dyn FnMut(&mut Game) + Sync + Send>;

/// A collection of systems associated directly to a [`Game`] as opposed to a [`Session`].
#[derive(Default)]
pub struct GameSystems {
    /// Flag which indicates whether or not the startup systems have been run yet.
    pub has_run_startup: bool,
    /// Startup systems.
    pub startup: Vec<GameSystem>,
    /// Game systems that are run before sessions are run.
    pub before: Vec<GameSystem>,
    /// Game systems that are run after sessions are run.
    pub after: Vec<GameSystem>,
    /// Game systems that are run after a specific session is run.
    pub after_session: HashMap<Ustr, Vec<GameSystem>>,
    /// Game systems that are run before a specific session is run.
    pub before_session: HashMap<Ustr, Vec<GameSystem>>,
}

impl GameSystems {
    /// Add a system that will run only once, before all of the other non-startup systems.
    pub fn add_startup_system<F>(&mut self, system: F) -> &mut Self
    where
        F: FnMut(&mut Game) + Sync + Send + 'static,
    {
        self.startup.push(Box::new(system));
        self
    }

    /// Add a system that will run on every step, before all of the sessions are run.
    pub fn add_before_system<F>(&mut self, system: F) -> &mut Self
    where
        F: FnMut(&mut Game) + Sync + Send + 'static,
    {
        self.before.push(Box::new(system));
        self
    }

    /// Add a system that will run on every step, after all of the sessions are run.
    pub fn add_after_system<F>(&mut self, system: F) -> &mut Self
    where
        F: FnMut(&mut Game) + Sync + Send + 'static,
    {
        self.after.push(Box::new(system));
        self
    }

    /// Add a system that will run every time the named session is run, before the session is run.
    pub fn add_before_session_system<F>(&mut self, session: &str, system: F) -> &mut Self
    where
        F: FnMut(&mut Game) + Sync + Send + 'static,
    {
        self.before_session
            .entry(session.into())
            .or_default()
            .push(Box::new(system));
        self
    }

    /// Add a system that will run every time the named session is run, after the session is run.
    pub fn add_after_session_system<F>(&mut self, session: &str, system: F) -> &mut Self
    where
        F: FnMut(&mut Game) + Sync + Send + 'static,
    {
        self.after_session
            .entry(session.into())
            .or_default()
            .push(Box::new(system));
        self
    }
}

/// Type of session command
pub type SessionCommand = dyn FnOnce(&mut Sessions) + Sync + Send;

/// Container for multiple game sessions.
///
/// Each session shares the same [`Entities`].
#[derive(HasSchema, Default)]
pub struct Sessions {
    map: UstrMap<Session>,

    /// Commands that operate on [`Sessions`], called after all sessions update.
    /// These may be used to add/delete/modify sessions.
    ///
    /// Commands are useful in a situation where you want to remove / recreate
    /// a session from within it's own system. You cannot do this while the `Session` is running.
    ///
    /// Commands added inside a session command will not be executed until next frame.
    commands: VecDeque<Box<SessionCommand>>,
}

/// Resource that allows you to configure the current session.
#[derive(HasSchema, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct SessionOptions {
    /// Whether or not this session should be active after this frame.
    pub active: bool,
    /// Whether or not this session should be visible.
    pub visible: bool,
    /// Whether or not this session should be deleted.
    pub delete: bool,
}

impl Sessions {
    /// Create a new session, and borrow it mutably so it can be modified.
    #[track_caller]
    pub fn create<K: TryInto<Ustr>>(&mut self, name: K) -> &mut Session
    where
        <K as TryInto<Ustr>>::Error: Debug,
    {
        let name = name.try_into().unwrap();
        // Create a blank session
        let mut session = Session::new();

        // Initialize the sessions resource in the session so it will be available in [`Game::step()`].
        session.world.init_resource::<Sessions>();

        // Insert it into the map
        self.map.insert(name, session);

        // And borrow it for the modification
        self.map.get_mut(&name).unwrap()
    }

    /// Delete a session.
    #[track_caller]
    pub fn delete<K: TryInto<Ustr>>(&mut self, name: K)
    where
        <K as TryInto<Ustr>>::Error: Debug,
    {
        self.map.remove(&name.try_into().unwrap());
    }

    /// Borrow a session from the sessions list.
    #[track_caller]
    pub fn get<K: TryInto<Ustr>>(&self, name: K) -> Option<&Session>
    where
        <K as TryInto<Ustr>>::Error: Debug,
    {
        self.map.get(&name.try_into().unwrap())
    }

    /// Borrow a session from the sessions list.
    #[track_caller]
    pub fn get_mut<K: TryInto<Ustr>>(&mut self, name: K) -> Option<&mut Session>
    where
        <K as TryInto<Ustr>>::Error: Debug,
    {
        self.map.get_mut(&name.try_into().unwrap())
    }

    /// Mutably iterate over sessions.
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<Ustr, Session> {
        self.map.iter_mut()
    }

    /// Iterate over sessions.
    pub fn iter(&self) -> std::collections::hash_map::Iter<Ustr, Session> {
        self.map.iter()
    }

    /// Add a [`SessionCommand`] to queue.
    pub fn add_command(&mut self, command: Box<SessionCommand>) {
        self.commands.push_back(command);
    }
}

// We implement `Clone` so that the world can still be snapshot with this resouce in it, but we
// don't actually clone the sessions, since they aren't `Clone`, and the actual sessions shouldn't
// be present in the world when taking a snapshot.
impl Clone for Sessions {
    fn clone(&self) -> Self {
        Self::default()
    }
}
