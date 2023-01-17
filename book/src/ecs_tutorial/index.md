# ECS Tutorial

`bones_ecs` is the core of the Bones framework, and all of the other crates depend on it, but it can also be used independently of the rest of Bones.

Here we'll give an overview of Bones ECS and how to use it.

Throughout this tutorial we assume that you have imported the `bones_ecs` prelude:

```rust
# extern crate bones_ecs;
use bones_ecs::prelude::*;
```

Or if you're using `bones_lib`:

```rust,ignore
use bones_lib::ecs::prelude::*;
```

You must also add the `type_ulid` crate to your `Cargo.toml` ( this won't be necessary in the future ).

## Components

Let's start with making some components. We'll need a `Pos` component and a `Vel` component, to represent positions and velocities:

```rust
# extern crate bones_ecs;
# extern crate type_ulid;
# use bones_ecs::prelude::*;
/// Our position component.
#[derive(Clone, TypeUlid, Debug)]
#[ulid = "01GPYNA7R38W826BG79QG0X590"]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

/// Our velocity component.
#[derive(Clone, TypeUlid, Debug)]
#[ulid = "01GPYN9961WQAJ19Q6NEQ6AFPH"]
pub struct Vel {
    pub x: f32,
    pub y: f32,
}
```

Components are just rust structs, but notice that we are required to derive at least two traits on all our components: `Clone` and `TypeUlid`.

- `Clone` is required to allow snapshotting the world state.
- `TypeUlid` is required to provide unique component identifiers across mods.

For every different component you create, it has to have a different ULID. You can generate a ULID by going to [webassembly.sh](https://webassembly.sh) and typing the command `ulid`.

> **Note:** We are considering removing the `TypeUlid` requirement. See this [issue](https://github.com/fishfolk/bones/issues/49) for details.

## Systems

Now that we have components, we can create our systems.

### Setup

Let's start off with a `setup_system` that will spawn some entities into our world.

Systems in Bones ECS are just functions or closures where all the arguments are `SystemParam`s.

```rust
# extern crate bones_ecs;
# extern crate type_ulid;
# use bones_ecs::prelude::*;
# #[derive(Clone, TypeUlid, Debug)]
# #[ulid = "01GPYNA7R38W826BG79QG0X590"]
# pub struct Pos {
#     pub x: f32,
#     pub y: f32,
# }
# #[derive(Clone, TypeUlid, Debug)]
# #[ulid = "01GPYN9961WQAJ19Q6NEQ6AFPH"]
# pub struct Vel {
#     pub x: f32,
#     pub y: f32,
# }
// Spawn our initial entities.
fn setup_system(
    mut entities: ResMut<Entities>,
    mut positions: CompMut<Pos>,
    mut velocities: CompMut<Vel>,
) {
    // Create our first entity
    let ent1 = entities.create();
    // Add add Pos and a Vel component to it.
    positions.insert(ent1, Pos { x: 0., y: 0.});
    velocities.insert(ent1, Vel { x: 3.0, y: 1.0 });

    // And do the same with another entity
    let ent2 = entities.create();
    positions.insert(ent2, Pos { x: 0., y: 100. });
    velocities.insert(ent2, Vel { x: 0.0, y: -1.0 });
}
```

Notice here how we use two different kinds of system parameters: `ResMut` and `CompMut`.

- `ResMut` gives us mutable access to a resource. In this case, we specifically access the `Entities` resource, which is always present in the `World`. The `Entities` resource allows us to create and kill entities, as well as iterate over entities.
- `CompMut` gives us mutable access to a component store. `CompMut<Vel>` can almost be though of as a `HashMap<Entity, Vel>`. Similarly, you can get the position of an entity with `positions.get(ent)`.

For both resources and components there are non-mutable variants, `Res` and `Comp`, that may be used of you only need to read from it.

### Position-Velocity System

Now that we've got our `setup_system` made, let's add a system that will update all the entity's positions based on their velocities.

```rust
# extern crate bones_ecs;
# extern crate type_ulid;
# use bones_ecs::prelude::*;
# #[derive(Clone, TypeUlid, Debug)]
# #[ulid = "01GPYNA7R38W826BG79QG0X590"]
# pub struct Pos {
#     pub x: f32,
#     pub y: f32,
# }
# #[derive(Clone, TypeUlid, Debug)]
# #[ulid = "01GPYN9961WQAJ19Q6NEQ6AFPH"]
# pub struct Vel {
#     pub x: f32,
#     pub y: f32,
# }
fn pos_vel_system(
    entities: Res<Entities>,
    mut positions: CompMut<Pos>,
    velocities: Comp<Vel>,
) {
    for (entity, (pos, vel)) in entities.iter_with((&mut positions, &velocities)) {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

Here we see a new feature of the `Entities` resource. It allows us to iterate over all of our entities that have specific components. In this case, want to iterate over all our entities that have a position and a velocity, with mutable access to the position and read access to the velocity.

### Print System

Finally, we'll want to be able to see what our system is doing, so let's add one more system to print out the velocities and positions of our entities:

```rust
# extern crate bones_ecs;
# extern crate type_ulid;
# use bones_ecs::prelude::*;
# #[derive(Clone, TypeUlid, Debug)]
# #[ulid = "01GPYNA7R38W826BG79QG0X590"]
# pub struct Pos {
#     pub x: f32,
#     pub y: f32,
# }
# #[derive(Clone, TypeUlid, Debug)]
# #[ulid = "01GPYN9961WQAJ19Q6NEQ6AFPH"]
# pub struct Vel {
#     pub x: f32,
#     pub y: f32,
# }
fn print_system(
    entities: Res<Entities>,
    positions: Comp<Pos>,
    velocities: Comp<Vel>,
) {
    println!("========");

    for (entity, (pos, vel)) in entities.iter_with((&positions, &velocities)) {
        println!("{entity:?}: {pos:?} - {vel:?}");
    }
}
```

## `World` and `SystemStages`

Now that we've defined our components and our systems, it's time to put them together.

The first step is to create a `World` to store our components and resources in:

```rust
# extern crate bones_ecs;
# use bones_ecs::prelude::*;
let mut world = World::new();
```

And now we can run our setup system, to create our entities:

```rust
# extern crate bones_ecs;
# use bones_ecs::prelude::*;
# fn setup_system() {}
# let mut world = World::new();
world.run_system(setup_system).unwrap();
```

The `run_system()` function will run a system one time. Though we haven't done so in any of our systems so far, systems are allowed to return a `SystemResult`, so we have to unwrap the possible error when running the system.

While we only want to run our setup system once, we will need to run our other systems multiple times, and for that we use `SystemStages`.

```rust
# extern crate bones_ecs;
# use bones_ecs::prelude::*;
let mut stages = SystemStages::with_core_stages();
```

This creates a new `SystemStages` collection, pre-populated with the `CoreStage`s. There are five core stages, run in order:

- `CoreStage::First`
- `CoreStage::PreUpdate`
- `CoreStage::Update`
- `CoreStage::PostUpdate`
- `CoreStage::Last`

These groupings make it easier to control which order your systems run, when you have a lot of different modules all adding systems to your `SystemStages`.

```rust
# extern crate bones_ecs;
# use bones_ecs::prelude::*;
# fn pos_vel_system() {}
# fn print_system() {}
# let mut stages = SystemStages::with_core_stages();
stages
    .add_system_to_stage(CoreStage::Update, pos_vel_system)
    .add_system_to_stage(CoreStage::PostUpdate, print_system);
```

Once we have added all our systems we have to initialize them. This will make sure that any component store or resources that they access are created registered with the world.

```rust
# extern crate bones_ecs;
# use bones_ecs::prelude::*;
# let mut world = World::new();
# let mut stages = SystemStages::with_core_stages();
stages.initialize_systems(&mut world);
```

Finally, we can run our stages against the world, and it will be sure to execute all of our systems in all of their stages every time we call `run()`. Let's run it ten times, so we can see our positions changing according to their velocity over a few frames:

```rust
# extern crate bones_ecs;
# use bones_ecs::prelude::*;
# let mut world = World::new();
# let mut stages = SystemStages::with_core_stages();
# stages.initialize_systems(&mut world);
for _ in 0..10 {
    stages.run(&mut world).unwrap();
}
```

## Full Example

And that's it for the intro! Here's the full example:

```rust,ignore
{{#include ../../../crates/bones_ecs/examples/pos_vel.rs}}
```
