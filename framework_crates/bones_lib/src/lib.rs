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
        ecs::prelude::*, Game, Plugin, Session, SessionOptions, SessionRunner, Sessions,
    };
}

use std::fmt::Debug;

use crate::prelude::*;

/// A bones game. This includes all of the game worlds, and systems.
#[derive(Deref, DerefMut)]
pub struct Session {
    /// The ECS world for the core.
    pub world: World,
    /// The system
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
        Self::default()
    }

    /// Install a plugin.
    pub fn install_plugin(&mut self, plugin: impl Plugin) -> &mut Self {
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
}

impl Default for Session {
    fn default() -> Self {
        Self {
            world: default(),
            stages: default(),
            active: true,
            visible: true,
            priority: 0,
            runner: Box::<DefaultSessionRunner>::default(),
        }
    }
}

/// Trait for plugins that can be installed into a [`Session`].
pub trait Plugin {
    /// Install the plugin into the [`Session`].
    fn install(self, session: &mut Session);
}
impl<F: FnOnce(&mut Session)> Plugin for F {
    fn install(self, core: &mut Session) {
        (self)(core)
    }
}

/// A session runner is in charge of advancing a [`Session`] simulation.
pub trait SessionRunner: Sync + Send + 'static {
    /// Step the simulation once.
    fn step(&mut self, world: &mut World, stages: &mut SystemStages) -> SystemResult;
}

/// The default [`SessionRunner`], which just runs the systems once every time it is run.
#[derive(Default)]
pub struct DefaultSessionRunner {
    /// Whether or not the systems have been initialized yet.
    pub has_init: bool,
}
impl SessionRunner for DefaultSessionRunner {
    fn step(&mut self, world: &mut World, stages: &mut SystemStages) -> SystemResult {
        // Initialize systems if they have not been initialized yet.
        if unlikely(!self.has_init) {
            self.has_init = true;
            stages.initialize_systems(world);
        }
        stages.run(world)
    }
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
    /// List of sorted session keys.
    ///
    /// These are only guaranteed to be sorted and up-to-date immediately after calling
    /// [`Game::step()`].
    pub sorted_session_keys: Vec<Key>,
    /// Collection of resources that will have a shared instance of each be inserted into each
    /// session automatically.
    pub shared_resources: Vec<UntypedAtomicResource>,
}

impl Game {
    /// Create an empty game with an asset server.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the shared resource of a given type out of this [`Game`]s shared resources.
    pub fn shared_resource<T: HasSchema>(&self) -> Option<RefMut<T>> {
        self.shared_resources
            .iter()
            .find(|x| x.schema() == T::schema())
            .map(|x| x.borrow_mut().typed())
    }

    /// Get the shared resource cell of a given type out of this [`Game`]s shared resources.
    pub fn shared_resource_cell<T: HasSchema>(&self) -> Option<AtomicResource<T>> {
        self.shared_resources
            .iter()
            .find(|x| x.schema() == T::schema())
            .map(|x| AtomicResource::from_untyped(x.clone()))
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
        self.shared_resource::<T>().unwrap()
    }

    /// Insert a resource that will be shared across all game sessions.
    pub fn insert_shared_resource<T: HasSchema + Default>(&mut self, resource: T) {
        let resource = UntypedAtomicResource::new(SchemaBox::new(resource));

        // Replace an existing resource of the same type.
        for r in &mut self.shared_resources {
            if r.schema() == T::schema() {
                *r = resource;
                return;
            }
        }

        // Or insert a new resource if we couldn't find one
        self.shared_resources.push(resource);
    }

    /// Step the game simulation.
    ///
    /// `apply_input` is a function that will be called once for every active [`Session`], allowing
    /// you to update the world with the current frame's input, whatever form that may come in.
    ///
    /// Usually this will be used to:
    /// - assign the player input to a resource so that the game can respond to player controls.
    /// - assign the window information to a resource, so that the game can respond to the window
    ///   size.
    /// - setup other important resources such as the UI context and the asset server, if
    ///   applicable.
    pub fn step<F: FnMut(&mut World)>(&mut self, mut apply_input: F) {
        // Sort session keys by priority
        self.sorted_session_keys.clear();
        self.sorted_session_keys.extend(self.sessions.map.keys());
        self.sorted_session_keys
            .sort_by_key(|name| self.sessions.map.get(name).unwrap().priority);

        // For every session
        for session_name in self.sorted_session_keys.clone() {
            // Extract the current session
            let mut current_session = self.sessions.map.remove(&session_name).unwrap();

            // If this session is active
            let options = if current_session.active {
                // Make sure session contains all of the shared resources
                for r in &self.shared_resources {
                    if !current_session
                        .world
                        .resources
                        .untyped()
                        .contains(r.schema().id())
                    {
                        current_session
                            .world
                            .resources
                            .untyped_mut()
                            .insert_cell(r.clone());
                    }
                }

                // Insert the session options
                current_session.world.resources.insert(SessionOptions {
                    active: true,
                    delete: false,
                    visible: current_session.visible,
                });

                // Apply the game input
                apply_input(&mut current_session.world);

                // Insert the other sessions into the current session's world
                {
                    let mut sessions = current_session.world.resource_mut::<Sessions>();
                    std::mem::swap(&mut *sessions, &mut self.sessions);
                }

                // Step the current session's simulation using it's session runner
                current_session
                    .runner
                    .step(&mut current_session.world, &mut current_session.stages)
                    .unwrap_or_else(|_| panic!("Error running session: {session_name}"));

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
        }
    }
}

/// Container for multiple game sessions.
///
/// Each session shares the same [`Entities`].
#[derive(HasSchema, Default, Debug)]
pub struct Sessions {
    entities: AtomicResource<Entities>,
    map: HashMap<Key, Session>,
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
    pub fn create<K: TryInto<Key>>(&mut self, name: K) -> &mut Session
    where
        <K as TryInto<Key>>::Error: Debug,
    {
        let name = name.try_into().unwrap();
        // Create a blank session
        let mut session = Session::new();

        // Make sure the new session has the same entities as the other sessions.
        session.world.resources.insert_cell(self.entities.clone());

        // Initialize the sessions resource in the session so it will be available in [`Game::step()`].
        session.world.init_resource::<Sessions>();

        // Insert it into the map
        self.map.insert(name, session);

        // And borrow it for the modification
        self.map.get_mut(&name).unwrap()
    }

    /// Delete a session.
    #[track_caller]
    pub fn delete<K: TryInto<Key>>(&mut self, name: K)
    where
        <K as TryInto<Key>>::Error: Debug,
    {
        self.map.remove(&name.try_into().unwrap());
    }

    /// Borrow a session from the sessions list.
    #[track_caller]
    pub fn get<K: TryInto<Key>>(&self, name: K) -> Option<&Session>
    where
        <K as TryInto<Key>>::Error: Debug,
    {
        self.map.get(&name.try_into().unwrap())
    }

    /// Borrow a session from the sessions list.
    #[track_caller]
    pub fn get_mut<K: TryInto<Key>>(&mut self, name: K) -> Option<&mut Session>
    where
        <K as TryInto<Key>>::Error: Debug,
    {
        self.map.get_mut(&name.try_into().unwrap())
    }

    /// Mutably iterate over sessions.
    pub fn iter_mut(&mut self) -> hashbrown::hash_map::IterMut<Key, Session> {
        self.map.iter_mut()
    }

    /// Iterate over sessions.
    pub fn iter(&self) -> hashbrown::hash_map::Iter<Key, Session> {
        self.map.iter()
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
