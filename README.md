# Bones

A work-in-progress, opinionated game meta-engine built on [Bevy].

Under development for future use in the [Jumpy] game, and possibly other FishFolk games.

Check out the [Bones Book][book] for more in-depth explanations and tutorials.

[book]: https://fishfolk.github.io/bones/
[bevy]: https://bevyengine.org
[jumpy]: https://github.com/fishfolk/jumpy

## Overview

### Bones ECS

Bones is designed around a simple, custom Entity Component System ( ECS ), designed to make it easier to get a few features that are important to us:

- **Determinism:** Bones ECS is deterministic by default, making it easier to get a re-producible and predictable gameplay.
- **Snapshot/Restore:** The Bones ECS world can be trivially snapshot and restored.
- **Modding/Scripting ( future ):** Bones ECS is simple enough that we can feasibly provide a C API for integration with other languages and scripting.

Determinism and Snapshot/Restore are also key features for getting excellent **network play** with the rollback networking model, while requiring no changes to the core game loop implementation.

### Game Core

Using `bones_lib`, which includes `bones_ecs` and other useful utilities, you write your "game core".

This game core is mostly isolated from anything outside of the ECS world. This is important so that the entire game core can be snapshot/restored.

This means that to collect input, you must read those inputs from an ECS resource, and to render sprites, map tiles, etc., you must create entities with specific components that tell Bones how to render them.

### Bevy Renderer

Once you have your game core, you can render the core in a Bevy game using the `bones_bevy_renderer` crate.

This lets you utilize the power and plugin ecosystem of Bevy to interact with rendering, input, etc. while keeping your core game deterministic, snapshot-able, and moddable.

You are also free to create your own rendering components that you synchronize with Bevy to support any custom rendering/audio use-cases, etc.

> **Note:** Bones ECS as well as `bones_lib` can technically be used without Bevy. Right now we are focusing on Bevy, and support for alternative uses may not be well polished/complete yet, but it's still within the realm of possibility to render Bones cores with any framework you want.

### Bones App ( Future )

As we get a feel for how things fit together with the use of Bones in [Jumpy], we hope to be able to create a standardized game runner around Bones game cores. This would allow the Bones app to handle common things like localization, asset loading, input mapping, settings menu, networking, etc., allowing you to focus on writing what makes your game unique.
