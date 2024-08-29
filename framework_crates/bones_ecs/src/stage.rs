//! Implementation of stage abstraction for running collections of systems over a [`World`].

use std::collections::VecDeque;

use crate::prelude::*;

/// Resource that is automatically added to the world while a system stage is being run
/// that specifies the unique ID of the stage that being run.
///
/// If the stage is `Ulid(0)`, the default ID, then that means the startup stage is being run.
#[derive(Deref, DerefMut, Clone, Copy, HasSchema, Default)]
pub struct CurrentSystemStage(pub Ulid);

/// An ordered collection of [`SystemStage`]s.
pub struct SystemStages {
    /// The stages in the collection, in the order that they will be run.
    pub stages: Vec<Box<dyn SystemStage>>,
    /// Whether or not the startup systems have been run yet.
    pub has_started: bool,
    /// The systems that should run at startup.
    pub startup_systems: Vec<StaticSystem<(), ()>>,
    /// Systems that are continously run until they succeed(return Some). These run before all stages. Uses Option to allow for easy usage of `?`.
    pub single_success_systems: Vec<StaticSystem<(), Option<()>>>,
}

impl std::fmt::Debug for SystemStages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemStages")
            // TODO: Add list of stages to the debug render for `SystemStages`.
            // We can at least list the names of each stage for `SystemStages` debug
            // implementation.
            .finish()
    }
}

impl Default for SystemStages {
    fn default() -> Self {
        Self::with_core_stages()
    }
}

impl SystemStages {
    /// Execute the systems on the given `world`.
    pub fn run(&mut self, world: &mut World) {
        // If we haven't run our startup systems yet
        if !self.has_started {
            // Set the current stage resource
            world.insert_resource(CurrentSystemStage(Ulid(0)));

            // For each startup system
            for system in &mut self.startup_systems {
                // Run the system
                system.run(world, ());
            }

            // Don't run startup systems again
            self.has_started = true;
        }

        // Run single success systems
        self.single_success_systems.retain_mut(|system| {
            let result = system.run(world, ());
            result.is_none() // Keep the system if it didn't succeed (returned None)
        });

        // Run each stage
        for stage in &mut self.stages {
            // Set the current stage resource
            world.insert_resource(CurrentSystemStage(stage.id()));

            // Run the stage
            stage.run(world);
        }

        // Cleanup killed entities
        world.maintain();

        // Remove the current system stage resource
        world.resources.remove::<CurrentSystemStage>();
    }

    /// Create a [`SystemStages`] collection, initialized with a stage for each [`CoreStage`].
    pub fn with_core_stages() -> Self {
        Self {
            stages: vec![
                Box::new(SimpleSystemStage::new(CoreStage::First)),
                Box::new(SimpleSystemStage::new(CoreStage::PreUpdate)),
                Box::new(SimpleSystemStage::new(CoreStage::Update)),
                Box::new(SimpleSystemStage::new(CoreStage::PostUpdate)),
                Box::new(SimpleSystemStage::new(CoreStage::Last)),
            ],
            has_started: false,
            startup_systems: default(),
            single_success_systems: Vec::new(),
        }
    }

    /// Add a system that will run only once, before all of the other non-startup systems.
    pub fn add_startup_system<Args, S>(&mut self, system: S) -> &mut Self
    where
        S: IntoSystem<Args, (), (), Sys = StaticSystem<(), ()>>,
    {
        self.startup_systems.push(system.system());
        self
    }

    /// Add a system that will run each frame until it succeeds (returns Some). Runs before all stages. Uses Option to allow for easy usage of `?`.
    pub fn add_single_success_system<Args, S>(&mut self, system: S) -> &mut Self
    where
        S: IntoSystem<Args, (), Option<()>, Sys = StaticSystem<(), Option<()>>>,
    {
        self.single_success_systems.push(system.system());
        self
    }

    /// Add a [`System`] to the stage with the given label.
    pub fn add_system_to_stage<Args, S>(&mut self, label: impl StageLabel, system: S) -> &mut Self
    where
        S: IntoSystem<Args, (), (), Sys = StaticSystem<(), ()>>,
    {
        let name = label.name();
        let id = label.id();
        let mut stage = None;

        for st in &mut self.stages {
            if st.id() == id {
                stage = Some(st);
            }
        }

        let Some(stage) = stage else {
            panic!("Stage with label `{}` ( {} ) doesn't exist.", name, id);
        };

        stage.add_system(system.system());

        self
    }

    /// Insert a new stage, before another existing stage
    #[track_caller]
    pub fn insert_stage_before<L: StageLabel, S: SystemStage + 'static>(
        &mut self,
        label: L,
        stage: S,
    ) -> &mut Self {
        let stage_idx = self
            .stages
            .iter()
            .position(|x| x.id() == label.id())
            .unwrap_or_else(|| panic!("Could not find stage with label `{}`", label.name()));
        self.stages.insert(stage_idx, Box::new(stage));

        self
    }

    /// Insert a new stage, after another existing stage
    #[track_caller]
    pub fn insert_stage_after<L: StageLabel, S: SystemStage + 'static>(
        &mut self,
        label: L,
        stage: S,
    ) -> &mut Self {
        let stage_idx = self
            .stages
            .iter()
            .position(|x| x.id() == label.id())
            .unwrap_or_else(|| panic!("Could not find stage with label `{}`", label.name()));
        self.stages.insert(stage_idx + 1, Box::new(stage));

        self
    }

    /// Remove all systems from all stages, including startup and single success systems. Resets has_started as well, allowing for startup systems to run once again.
    pub fn reset_remove_all_systems(&mut self) {
        // Reset the has_started flag
        self.has_started = false;
        self.remove_all_systems();
    }

    /// Remove all systems from all stages, including startup and single success systems. Does not reset has_started.
    pub fn remove_all_systems(&mut self) {
        // Clear startup systems
        self.startup_systems.clear();

        // Clear single success systems
        self.single_success_systems.clear();

        // Clear systems from each stage
        for stage in &mut self.stages {
            stage.remove_all_systems();
        }
    }
}

/// Trait for system stages. A stage is a
pub trait SystemStage: Sync + Send {
    /// The unique identifier for the stage.
    fn id(&self) -> Ulid;
    /// The human-readable name for the stage, used for error messages when something goes wrong.
    fn name(&self) -> String;
    /// Execute the systems on the given `world`.
    fn run(&mut self, world: &World);

    /// Add a system to this stage.
    fn add_system(&mut self, system: StaticSystem<(), ()>);
    /// Remove all systems from this stage.
    fn remove_all_systems(&mut self);
}

/// A collection of systems that will be run in order.
pub struct SimpleSystemStage {
    /// The unique identifier for the stage.
    pub id: Ulid,
    /// The human-readable name for the stage, used for error messages when something goes wrong.
    pub name: String,
    /// The list of systems in the stage.
    ///
    /// Each system will be run in the order that they are in in this list.
    pub systems: Vec<StaticSystem<(), ()>>,
}

impl SimpleSystemStage {
    /// Create a new, empty stage, for the given label.
    pub fn new<L: StageLabel>(label: L) -> Self {
        Self {
            id: label.id(),
            name: label.name(),
            systems: Default::default(),
        }
    }
}

impl SystemStage for SimpleSystemStage {
    fn id(&self) -> Ulid {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn run(&mut self, world: &World) {
        // Run the systems
        for system in &mut self.systems {
            system.run(world, ());
        }

        // Drain the command queue
        let queue = world.resources.get_mut::<CommandQueue>();
        if let Some(mut command_queue) = queue {
            for mut system in command_queue.queue.drain(..) {
                system.run(world, ());
            }
        }
    }

    fn add_system(&mut self, system: StaticSystem<(), ()>) {
        self.systems.push(system);
    }

    fn remove_all_systems(&mut self) {
        self.systems.clear();
    }
}

/// Trait for things that may be used to identify a system stage.
pub trait StageLabel {
    /// Returns the human-readable name of the label, used in error messages.
    fn name(&self) -> String;
    /// Returns a unique identifier for the stage.
    fn id(&self) -> Ulid;
}

/// A [`StageLabel`] for the 5 core stages.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CoreStage {
    /// The first stage
    First,
    /// The second stage
    PreUpdate,
    /// The third stage
    Update,
    /// The fourth stage
    PostUpdate,
    /// The fifth stage
    Last,
}

impl StageLabel for CoreStage {
    fn name(&self) -> String {
        format!("{:?}", self)
    }

    fn id(&self) -> Ulid {
        match self {
            CoreStage::First => Ulid(2021715391084198804812356024998495966),
            CoreStage::PreUpdate => Ulid(2021715401330719559452824437611089988),
            CoreStage::Update => Ulid(2021715410160177201728645950400543948),
            CoreStage::PostUpdate => Ulid(2021715423103233646561968734173322317),
            CoreStage::Last => Ulid(2021715433398666914977687392909851554),
        }
    }
}

/// A resource containing the [`Commands`] command queue.
///
/// You can use [`Commands`] as a [`SystemParam`] as a shortcut to [`ResMut<CommandQueue>`].
#[derive(HasSchema, Default)]
pub struct CommandQueue {
    /// The system queue that will be run at the end of the stage
    pub queue: VecDeque<StaticSystem<(), ()>>,
}

impl Clone for CommandQueue {
    fn clone(&self) -> Self {
        if self.queue.is_empty() {
            Self {
                queue: VecDeque::with_capacity(self.queue.capacity()),
            }
        } else {
            panic!(
                "Cannot clone CommandQueue. This probably happened because you are \
                trying to clone a World while a system stage is still executing."
            )
        }
    }
}

impl CommandQueue {
    /// Add a system to be run at the end of the stage.
    pub fn add<Args, S>(&mut self, system: S)
    where
        S: IntoSystem<Args, (), (), Sys = StaticSystem<(), ()>>,
    {
        self.queue.push_back(system.system());
    }
}

/// A [`SystemParam`] that can be used to schedule systems that will be run at the end of the
/// current [`SystemStage`].
///
/// This is a shortcut for [`ResMut<CommandQueue>`].
#[derive(Deref, DerefMut)]
pub struct Commands<'a>(RefMut<'a, CommandQueue>);

impl<'a> SystemParam for Commands<'a> {
    type State = AtomicResource<CommandQueue>;
    type Param<'s> = Commands<'s>;

    fn get_state(world: &World) -> Self::State {
        let cell = world.resources.get_cell::<CommandQueue>();
        cell.init(world);
        cell
    }

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        Commands(state.borrow_mut().unwrap())
    }
}
