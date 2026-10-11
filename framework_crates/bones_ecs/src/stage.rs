//! Implementation of stage abstraction for running collections of systems over a [`World`].

use std::collections::VecDeque;
use std::marker::PhantomData;

use crate::prelude::*;

/// An ordered collection of [`SystemStage`]s.
pub struct SystemStages {
    /// The stages in the collection, in the order that they will be run.
    stages: Vec<Box<dyn SystemStage>>,
}

impl Default for SystemStages {
    fn default() -> Self {
        Self::with_core_stages()
    }
}

impl SystemStages {
    /// Create a [`SystemStages`] collection, initialized with a stage for each [`CoreStage`].
    pub fn with_core_stages() -> Self {
        Self {
            stages: vec![
                Box::new(
                    StartupSystemStage::<CoreStageStartupResourceSystemsRan>::new(
                        CoreStage::StartupResource,
                    ),
                ),
                Box::new(StartupSystemStage::<CoreStageStartupSystemsRan>::new(
                    CoreStage::Startup,
                )),
                Box::new(SimpleSystemStage::new(CoreStage::First)),
                Box::new(SimpleSystemStage::new(CoreStage::PreUpdate)),
                Box::new(SimpleSystemStage::new(CoreStage::Update)),
                Box::new(SimpleSystemStage::new(CoreStage::PostUpdate)),
                Box::new(SimpleSystemStage::new(CoreStage::Last)),
            ],
        }
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

    /// Execute the systems on the given `world`.
    pub fn run(&mut self, world: &mut World) {
        for stage in &mut self.stages {
            stage.run(world);
        }
        // Cleanup killed entities
        world.maintain();
    }

    /// Add a system to the [`CoreStage::Startup`] stage which specially runs
    /// only once, before all other core stages. Inserting [`CoreStageStartupSystemsRan`]
    /// into the world prevents systems in the startup stage from running.
    #[deprecated = "use `.add_system_to_stage(CoreStage::Startup, your_system)` instead"]
    pub fn add_startup_system<Args, S>(&mut self, system: S) -> &mut Self
    where
        S: IntoSystem<Args, (), (), Sys = StaticSystem<(), ()>>,
    {
        self.add_system_to_stage(CoreStage::Startup, system.system());
        self
    }

    /// Convenience method. Adds a system that will run each frame until it succeeds (returns Some).
    /// Uses Option to allow for easy usage of `?`. System is added to
    /// [`CoreStage::First`].
    ///
    /// See also the wrapper function used inside [`single_success_system`].
    #[deprecated = "use `.add_system_to_stage(First, single_success_system(Ulid::new(), your_system)"]
    pub fn add_single_success_system<Args, S>(&mut self, system: S) -> &mut Self
    where
        S: IntoSystem<Args, (), Option<()>, Sys = StaticSystem<(), Option<()>>>,
    {
        self.add_system_to_stage(First, single_success_system(Ulid::new(), system));
        self
    }

    /// Adds a system to [`CoreStage::Startup`] that inserts the default value
    /// of a resource into the [`World`] if it doesn't already exist.
    pub fn init_startup_resource<R: HasSchema + FromWorld>(&mut self) -> &mut Self {
        self.add_system_to_stage(StartupResource, |world: &World| {
            world.init_resource::<R>();
        });
        self
    }

    /// Adds a system to [`CoreStage::StartupResource`] that inserts the given
    /// resource into the [`World`] if it doesn't already exist. The resource
    /// type is required to have a [`HasSchema`] clone implementation to
    /// re-insert the resource into the world after the first startup.
    ///
    /// # Clone Requirement Details
    /// If the type being added implements [`HasSchema`] with the derive like
    /// so:
    /// ```ignore
    /// #[derive(HasSchema, Default)]
    /// #[schema(no_clone)] // This bypasses HasSchema clone requirements.
    /// struct ResourceBeingInserted;
    /// ```
    /// Then the system being added to the [`StartupResource`] system stage will
    /// panic when inserting a second time. The startup resource stage only runs
    /// once so it is therefore safe to use a type like the example above as
    /// long as you don't plan on removing from the [`World`] the
    /// [`StartupSystemStage`] tag which in this case is
    /// [`CoreStageStartupResourceSystemsRan`].
    pub fn add_startup_resource<R: HasSchema>(&mut self, resource: R) -> &mut Self {
        let mut once = Some(SchemaBox::new(resource));

        self.add_system_to_stage(StartupResource, move |world: &World| {
            if !world.resources.contains::<R>() {
                let schema_box = if R::schema().clone_fn.is_some() {
                    once.clone()
                } else {
                    once.take()
                };

                if let Some(schema_box) = schema_box {
                    world
                        .resources
                        .untyped()
                        .get(R::schema())
                        .insert(schema_box)
                        .unwrap();
                } else {
                    panic!(
                        "The schema for this type does not allow cloning it.\nSchema: {:#?}; \
                    You may need to add the resource to the `ResetWorld::reset_resources` \
                    to avoid cloning on startup.",
                        R::schema()
                    )
                };
            }
        });
        self
    }
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

/// Resource for tracking whether or not the game has run the startup systems in
/// [`CoreStage::Startup`].
#[derive(HasSchema, Clone, Default, Debug)]
pub struct CoreStageStartupSystemsRan;

/// Resource for tracking whether or not the game has run the startup systems in
/// [`CoreStage::StartupResource`].
#[derive(HasSchema, Clone, Default, Debug)]
pub struct CoreStageStartupResourceSystemsRan;

/// [`SystemStage`] for running systems once on startup.
pub struct StartupSystemStage<T: HasSchema + Default + Clone> {
    /// The unique identifier for the stage.
    pub id: Ulid,
    /// The human-readable name for the stage, used for error messages when something goes wrong.
    pub name: String,
    /// The list of systems in the stage.
    ///
    /// Each system will be run in the order that they are in in this list.
    pub systems: Vec<StaticSystem<(), ()>>,
    _phantom_data: PhantomData<T>,
}
impl<T: HasSchema + Clone + Default> StartupSystemStage<T> {
    /// Create a new, empty stage, for the given label.
    pub fn new<L: StageLabel>(label: L) -> Self {
        Self {
            id: label.id(),
            name: label.name(),
            systems: Default::default(),
            _phantom_data: PhantomData,
        }
    }
}
impl<T: HasSchema + Clone + Default> SystemStage for StartupSystemStage<T> {
    fn id(&self) -> Ulid {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn run(&mut self, world: &World) {
        if world.get_resource_mut::<T>().is_none() {
            // Run the systems
            for system in &mut self.systems {
                system.run(world, ());
            }
            world.insert_resource(T::default());
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

/// Wrapper system for single success systems.
///
/// Runs the given system each frame until it succeeds (returns Some). Uses
/// `Option<()>` instead of a `bool` to allow for easy usage of `?`.
///
/// Takes a [`Ulid`] that is used to identify whether or not the system has
/// succeeded from the [`SingleSuccessSystems`] resource.
pub fn single_success_system<Args, S>(id: Ulid, system: S) -> StaticSystem<(), ()>
where
    S: IntoSystem<Args, (), Option<()>, Sys = StaticSystem<(), Option<()>>>,
{
    let mut system = system.system();
    (move |world: &World| {
        let succeeded = world
            .init_resource::<SingleSuccessSystems>()
            .has_system_succeeded(id);
        if !succeeded && system.run(world, ()).is_some() {
            world
                .init_resource::<SingleSuccessSystems>()
                .set_system_completed(id);
        }
    })
    .system()
}

// TODO: Figure out a name for a function that creates a single success system
// with the Ulid created automatically internally. Maybe rename the current
// function to something else and have the auto-generated id function as
// standard since there is probably only a slim possibility that you will need
// to re-use a Ulid for these kind of systems.

/// Resource tracking which of single success systems have completed with a
/// [`HashSet`] of [`Ulid`] for the succeeded.
#[derive(HasSchema, Clone, Default, Deref, DerefMut)]
pub struct SingleSuccessSystems(pub HashSet<Ulid>);

impl SingleSuccessSystems {
    /// Reset single success systems completion status. so they run again until success.
    #[inline]
    pub fn reset(&mut self) {
        self.clear();
    }

    /// Check if system has completed
    #[inline]
    pub fn has_system_succeeded(&self, ulid: Ulid) -> bool {
        self.contains(&ulid)
    }

    /// Mark system as completed.
    #[inline]
    pub fn set_system_completed(&mut self, ulid: Ulid) {
        self.insert(ulid);
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
    /// The startup resource insertion stage
    StartupResource,
    /// The startup stage
    Startup,
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
            CoreStage::StartupResource => Ulid(2162940536129995539438442124983465554),
            CoreStage::Startup => Ulid(2149923225027836290977818907354065512),
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
