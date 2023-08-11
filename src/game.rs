//! [`Game`], [`Session`], and related types.

use crate::prelude::*;

/// A bones game. This includes all of the game worlds, and systems.
pub struct Session {
    /// The ECS world for the core.
    pub world: World,
    /// The system
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

    /// Run the session's stages on it's world once.
    pub fn step(&mut self) -> SystemResult {
        self.stages.run(&mut self.world)
    }

    /// Install a plugin.
    pub fn install_plugin(&mut self, plugin: impl BonesPlugin) -> &mut Self {
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
            runner: Box::new(DefaultSessionRunner),
        }
    }
}

/// Trait for plugins that can be installed into a [`Session`].
pub trait BonesPlugin {
    /// Install the plugin into the [`Session`].
    fn install(self, core: &mut Session);
}
impl<F: FnOnce(&mut Session)> BonesPlugin for F {
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
pub struct DefaultSessionRunner;
impl SessionRunner for DefaultSessionRunner {
    fn step(&mut self, world: &mut World, stages: &mut SystemStages) -> SystemResult {
        stages.run(world)
    }
}

/// The [`Game`] encompasses a complete bones game's logic, independent of the renderer and IO
/// implementations.
///
/// Games are made up of one or more [`Session`]s, each of which contains it's own [`World`] and
/// [`SystemStages`]. These different sessions can be used for parts of the game with independent
/// states, such as the main menu and the gameplay.
#[derive(Default, Debug)]
pub struct Game {
    /// The sessions that make up the game.
    pub sessions: HashMap<Key, Session>,
    session_keys_cache: Vec<Key>,
}

impl Game {
    /// Create an empty game.
    pub fn new() -> Self {
        Self::default()
    }

    /// Step the game simulation.
    pub fn step(&mut self) {
        // Sort session keys by priority
        self.session_keys_cache.extend(self.sessions.keys());
        self.session_keys_cache
            .sort_by_key(|name| self.sessions.get(name).unwrap().priority);

        // For every session
        for session_name in self.session_keys_cache.drain(..) {
            // Extract the current session
            let mut current_session = self.sessions.remove(&session_name).unwrap();

            // If we are supposed to advance this session.
            if current_session.active {
                // Insert the other sessions into the current session's world
                {
                    current_session.world.init_resource::<Sessions>();
                    let mut sessions = current_session.world.resource_mut::<Sessions>();
                    std::mem::swap(&mut sessions.0, &mut self.sessions);
                }

                // Step the session simulation using it's session runner
                current_session
                    .runner
                    .step(&mut current_session.world, &mut current_session.stages)
                    .unwrap_or_else(|_| panic!("Error running session: {session_name}"));

                // Pull the sessions back out of the world
                {
                    let mut sessions = current_session.world.resource_mut::<Sessions>();
                    std::mem::swap(&mut sessions.0, &mut self.sessions);
                }
            }

            // Insert the current session back into the session list
            self.sessions.insert(session_name, current_session);
        }
    }
}

/// Resource that contains the other sessions in the [`Game`] while one session is being run.
#[derive(HasSchema, Default, Deref, DerefMut)]
#[schema(opaque)]
pub struct Sessions(pub HashMap<Key, Session>);

// We implement `Clone` so that the world can still be snapshot with this resouce in it, but we
// don't actually clone the sessions, since they aren't `Clone`, and the actual sessions shouldn't
// be present in the world when taking a snapshot.
impl Clone for Sessions {
    fn clone(&self) -> Self {
        Self::default()
    }
}
