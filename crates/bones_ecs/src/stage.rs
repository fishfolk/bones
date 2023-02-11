//! Implementation of stage abstraction for running collections of systems over a [`World`].

use std::collections::VecDeque;

use crate::prelude::*;

/// An ordered collection of [`SystemStage`]s.
pub struct SystemStages {
    /// The stages in the collection, in the order that they will be run.
    pub stages: Vec<Box<dyn SystemStage>>,
}

impl SystemStages {
    /// Initialize the systems in the stages agains the [`World`].
    ///
    /// This must be called once before calling [`run()`][Self::run].
    pub fn initialize_systems(&mut self, world: &mut World) {
        for stage in &mut self.stages {
            stage.initialize(world);
        }
    }

    /// Execute the systems on the given `world`.
    ///
    /// > **Note:** You must call [`initialize_systems()`][Self::initialize_systems] once before
    /// > calling `run()` one or more times.
    pub fn run(&mut self, world: &mut World) -> SystemResult {
        for stage in &mut self.stages {
            stage.run(world)?;
        }

        Ok(())
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
        }
    }

    /// Add a [`System`] to the stage with the given label.
    pub fn add_system_to_stage<Args, S: IntoSystem<Args, ()>, L: StageLabel>(
        &mut self,
        label: L,
        system: S,
    ) -> &mut Self {
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
}

/// Trait for system stages. A stage is a
pub trait SystemStage: Sync + Send {
    /// The unique identifier for the stage.
    fn id(&self) -> Ulid;
    /// The human-readable name for the stage, used for error messages when something goes wrong.
    fn name(&self) -> String;
    /// Execute the systems on the given `world`.
    ///
    /// > **Note:** You must call [`initialize()`][Self::initialize] once before calling `run()` one
    /// > or more times.
    fn run(&mut self, world: &mut World) -> SystemResult;
    /// Initialize the contained systems for the given `world`.
    ///
    /// Must be called once before calling [`run()`][Self::run].
    fn initialize(&mut self, world: &mut World);

    /// Add a system to this stage.
    fn add_system(&mut self, system: System<()>);
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
    pub systems: Vec<System<()>>,
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

    fn run(&mut self, world: &mut World) -> SystemResult {
        // Run the systems
        for system in &mut self.systems {
            system.run(world)?;
        }

        // Drain the command queue
        {
            let command_queue = world.resource::<CommandQueue>();
            let mut command_queue = command_queue.borrow_mut();

            for mut system in command_queue.queue.drain(..) {
                system.initialize(world);
                system.run(world).unwrap();
            }
        }

        Ok(())
    }

    fn initialize(&mut self, world: &mut World) {
        world.init_resource::<CommandQueue>();
        for system in &mut self.systems {
            system.initialize(world);
        }
    }

    fn add_system(&mut self, system: System<()>) {
        self.systems.push(system);
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
#[derive(Copy, Clone, Debug)]
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
#[derive(Debug, TypeUlid, Default)]
#[ulid = "01GPY3KPT0CDNCM23HTKAKN0NJ"]
pub struct CommandQueue {
    /// The system queue that will be run at the end of the stage
    pub queue: VecDeque<System>,
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
    pub fn add<Args, S: IntoSystem<Args, ()>>(&mut self, system: S) {
        self.queue.push_back(system.system());
    }
}

/// A [`SystemParam`] that can be used to schedule systems that will be run at the end of the
/// current [`SystemStage`].
///
/// This is a shortcut for [`ResMut<CommandQueue>`].
#[derive(Deref, DerefMut)]
pub struct Commands<'a>(AtomicRefMut<'a, CommandQueue>);

impl<'a> SystemParam for Commands<'a> {
    type State = AtomicResource<CommandQueue>;
    type Param<'s> = Commands<'s>;

    fn initialize(_world: &mut World) {}

    fn get_state(world: &World) -> Self::State {
        world.resource::<CommandQueue>()
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        Commands(state.borrow_mut())
    }
}
