//! Implementation of stage abstraction for running collections of systems over a [`World`].

use std::collections::VecDeque;

use crate::prelude::*;

/// Resource that is automatically added to the world while a system stage is being run
/// that specifies the unique ID of the stage that being run.
///
/// If the stage is `Ulid(0)`, the default ID, then that means the startup stage is being run.
#[derive(Deref, DerefMut, Clone, Copy, HasSchema, Default)]
pub struct CurrentSystemStage(pub Ulid);

/// Builder for [`SystemStages`]. It is immutable once created,
pub struct SystemStagesBuilder {
    /// The stages in the collection, in the order that they will be run.
    stages: Vec<Box<dyn SystemStage>>,
    /// The systems that should run at startup.
    /// They will be executed next step based on if [`SessionStarted`] resource in world says session has not started, or if resource does not exist.
    startup_systems: Vec<StaticSystem<(), ()>>,

    /// Resources installed during session plugin installs. Copied to world as first step on startup of stages' execution.
    startup_resources: UntypedResourceSet,

    /// Systems that are continously run until they succeed(return Some). These run before all stages. Uses Option to allow for easy usage of `?`.
    single_success_systems: Vec<StaticSystem<(), Option<()>>>,
}

impl Default for SystemStagesBuilder {
    fn default() -> Self {
        Self::with_core_stages()
    }
}

impl SystemStagesBuilder {
    /// Create a [`SystemStagesBuilder`] for [`SystemStages`] collection, initialized with a stage for each [`CoreStage`].
    pub fn with_core_stages() -> Self {
        Self {
            stages: vec![
                Box::new(SimpleSystemStage::new(CoreStage::First)),
                Box::new(SimpleSystemStage::new(CoreStage::PreUpdate)),
                Box::new(SimpleSystemStage::new(CoreStage::Update)),
                Box::new(SimpleSystemStage::new(CoreStage::PostUpdate)),
                Box::new(SimpleSystemStage::new(CoreStage::Last)),
            ],
            startup_resources: default(),
            startup_systems: default(),
            single_success_systems: Vec::new(),
        }
    }

    /// Finish building and convert to [`SystemStages`]
    pub fn finish(self) -> SystemStages {
        SystemStages {
            stages: self.stages,
            startup_systems: self.startup_systems,
            startup_resources: self.startup_resources,
            single_success_systems: self.single_success_systems,
        }
    }

    /// Create [`SystemStagesBuilder`] by taking existing [`SystemStages`].
    pub fn from_stages(stages: SystemStages) -> Self {
        Self {
            stages: stages.stages,
            startup_systems: stages.startup_systems,
            startup_resources: stages.startup_resources,
            single_success_systems: stages.single_success_systems,
        }
    }

    /// Add a system that will run only once, before all of the other non-startup systems.
    /// If wish to reset session and run again, can modify [`SessionStarted`] resource in world.
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

    /// Insert a startup resource. On stage / session startup (first step), will be inserted into [`World`].
    ///
    /// If already exists, will be overwritten.
    pub fn insert_startup_resource<T: HasSchema>(&mut self, resource: T) {
        self.startup_resources.insert_resource(resource);
    }

    /// Init startup resource with default, and return mutable ref for modification.
    /// If already exists, returns mutable ref to existing resource.
    pub fn init_startup_resource<T: HasSchema + Default>(&mut self) -> RefMut<T> {
        self.startup_resources.init_resource::<T>()
    }

    /// Get mutable reference to startup resource if found.
    #[track_caller]
    pub fn startup_resource_mut<T: HasSchema>(&self) -> Option<RefMut<T>> {
        self.startup_resources.resource_mut()
    }
}

/// An ordered collection of [`SystemStage`]s.
pub struct SystemStages {
    /// The stages in the collection, in the order that they will be run.
    stages: Vec<Box<dyn SystemStage>>,

    /// The systems that should run at startup.
    /// They will be executed next step based on if [`SessionStarted`] resource in world says session has not started, or if resource does not exist.
    startup_systems: Vec<StaticSystem<(), ()>>,

    /// Resources installed during session plugin installs. Copied to world as first step on startup of stages' execution.
    startup_resources: UntypedResourceSet,

    /// Systems that are continously run until they succeed(return Some). These run before all stages. Uses Option to allow for easy usage of `?`.
    single_success_systems: Vec<StaticSystem<(), Option<()>>>,
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
        SystemStagesBuilder::default().finish()
    }
}

impl SystemStages {
    /// Create builder for construction of [`SystemStages`].
    pub fn builder() -> SystemStagesBuilder {
        SystemStagesBuilder::default()
    }

    /// Execute the systems on the given `world`.
    pub fn run(&mut self, world: &mut World) {
        // If we haven't run startup systems and setup resources yet, do so
        self.handle_startup(world);

        // Run single success systems
        for (index, system) in self.single_success_systems.iter_mut().enumerate() {
            let should_run = !Self::has_single_success_system_succeeded(index, world);

            if should_run && system.run(world, ()).is_some() {
                Self::mark_single_success_system_succeeded(index, world);
            }
        }

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

    /// If [`SessionStarted`] resource indicates have not yet started,
    /// perform startup tasks (insert startup resources, run startup systems).
    ///
    /// For advanced use cases in which want to only insert startup resources, or run startup systems and split this
    /// behavior, see [`SystemStages::handle_startup_systems`] and `[SystemStages::handle_startup_resources`].
    ///
    ///
    /// While this is used internally by [`SystemStages::run`], this is also used
    /// for resetting world. This allows world to immediately startup and re-initialize after reset.
    ///
    /// # Panics
    ///
    /// May panic if resources are borrowed, should not borrow resources when calling.
    pub fn handle_startup(&mut self, world: &mut World) {
        self.handle_startup_resources(world);
        self.handle_startup_systems(world);
    }

    /// If [`SessionStarted`] resource indicates startup resources have not yet been inserted, will do so and update `SessionStarted`.
    ///
    /// This function contains only half of stage's startup behavior, see [`SystemStages::handle_startup`] if not intending to split
    /// resource insertion from startup systems (Splitting these is more for advanced special cases).
    ///
    /// # Panics
    ///
    /// May panic if resources are borrowed, should not borrow resources when calling.
    pub fn handle_startup_resources(&mut self, world: &mut World) {
        let (mut resources_inserted, systems_run) = match world.get_resource_mut::<SessionStarted>()
        {
            Some(session_started) => (
                session_started.startup_resources_inserted,
                session_started.startup_systems_executed,
            ),
            None => (false, false),
        };

        if !resources_inserted {
            self.insert_startup_resources(world);
            resources_inserted = true;

            world.insert_resource(SessionStarted {
                startup_systems_executed: systems_run,
                startup_resources_inserted: resources_inserted,
            });
        }
    }

    /// If [`SessionStarted`] resource indicates startup systems have not yet been executed, will do so and update `SessionStarted`.
    ///
    /// This function contains only half of stage's startup behavior, see [`SystemStages::handle_startup`] if not intending to split
    /// startup system execution from startup resource insertion (Splitting these is more for advanced special cases).
    ///
    /// # Panics
    ///
    /// May panic if resources are borrowed, should not borrow resources when calling.
    pub fn handle_startup_systems(&mut self, world: &mut World) {
        let (resources_inserted, mut systems_run) = match world.get_resource_mut::<SessionStarted>()
        {
            Some(session_started) => (
                session_started.startup_resources_inserted,
                session_started.startup_systems_executed,
            ),
            None => (false, false),
        };

        if !systems_run {
            // Set the current stage resource
            world.insert_resource(CurrentSystemStage(Ulid(0)));

            // For each startup system
            for system in &mut self.startup_systems {
                // Run the system
                system.run(world, ());
            }
            systems_run = true;

            world.resources.remove::<CurrentSystemStage>();
            world.insert_resource(SessionStarted {
                startup_systems_executed: systems_run,
                startup_resources_inserted: resources_inserted,
            });
        }
    }

    /// Check if single success system is marked as succeeded in [`SingleSuccessSystems`] [`Resource`].
    fn has_single_success_system_succeeded(system_index: usize, world: &World) -> bool {
        if let Some(system_success) = world.get_resource::<SingleSuccessSystems>() {
            return system_success.has_system_succeeded(system_index);
        }

        false
    }

    /// Mark a single success system as succeeded in [`SingleSuccessSystems`] [`Resource`].
    fn mark_single_success_system_succeeded(system_index: usize, world: &mut World) {
        if let Some(mut system_succes) = world.get_resource_mut::<SingleSuccessSystems>() {
            system_succes.set_system_completed(system_index);
            return;
        }

        // Resource does not exist - must initialize it
        world
            .init_resource::<SingleSuccessSystems>()
            .set_system_completed(system_index);
    }

    /// Insert the startup resources that [`SystemStages`] and session were built with into [`World`].
    ///
    /// This will update [`SessionStarted`] resource to flag that startup resources are inserted.
    fn insert_startup_resources(&self, world: &mut World) {
        for resource in self.startup_resources.resources().iter() {
            // Deep copy startup resource and insert into world.
            let resource_copy = resource.clone_data().unwrap();
            let resource_cell = world.resources.untyped().get_cell(resource.schema());
            let prev_val = resource_cell.insert(resource_copy).unwrap();

            // Warn on already existing resource
            if prev_val.is_some() {
                let schema_name = resource.schema().full_name;
                tracing::warn!("SystemStages` attempted to inserted resource {schema_name} on startup that already exists in world - startup resource not inserted.
                    When building new session, startup resources should be initialized on `SessionBuilder`.");
            }
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

/// Resource tracking if Session has started (startup systems executed and resources inserted).
/// If field is set to false, that operation needs to be handled on next stage run.
/// If resource is not present, assumed to have not started (and will be initialized upon next stage execution).
#[derive(Copy, Clone, HasSchema, Default)]
pub struct SessionStarted {
    /// Have startup systems executed?
    pub startup_systems_executed: bool,

    /// Have startup resources beeen inserted into [`World`]?
    pub startup_resources_inserted: bool,
}

/// Resource tracking which of single success systems in `Session`'s [`SystemStages`] have completed.
/// Success is tracked to
#[derive(HasSchema, Clone, Default)]
pub struct SingleSuccessSystems {
    /// Set of indices of [`SystemStages`]'s single success systems that have succeeded.
    pub systems_succeeded: HashSet<usize>,
}

impl SingleSuccessSystems {
    /// Reset single success systems completion status. so they run again until success.
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.systems_succeeded.clear();
    }

    /// Check if system has completed
    pub fn has_system_succeeded(&self, index: usize) -> bool {
        self.systems_succeeded.contains(&index)
    }

    /// Mark system as completed.
    pub fn set_system_completed(&mut self, index: usize) {
        self.systems_succeeded.insert(index);
    }
}
