<div align="center">
<img src="https://raw.githubusercontent.com/fishfolk/bones_branding/main/renders/logo-rect.svg" align="center" width="250px" />
<h1>Bones</h1>

A 'meta-engine' framework made to facilitate the development of moddable, multiplayer 2D games.

[![Documentation](https://img.shields.io/badge/documentation-fishfolk.org-green.svg?labelColor=1e1c24&color=f3ee7a)](https://fishfolk.org/bones/overview/introduction/)
[![Crates.io](https://img.shields.io/crates/v/bones_lib?labelColor=1e1c24)](https://crates.io/crates/bones_lib)
[![docs.rs](https://img.shields.io/docsrs/bones_framework?label=API%20Docs&labelColor=1e1c24)](https://docs.rs/bones_framework)
[![Main Branch Docs](https://img.shields.io/badge/API_Docs-Main_Branch-blue?labelColor=1e1c24&color=red)](https://fishfolk.github.io/bones/rustdoc/bones_framework/index.html)
[![License](https://img.shields.io/badge/License-MIT%20or%20Apache%202-green.svg?label=license&labelColor=1e1c24&color=34925e)](./LICENSE)
[![Discord](https://img.shields.io/discord/865004050357682246?logo=discord&logoColor=white)](https://discord.gg/4smxjcheE5)

<hr />

</div>

Initially borne out of [Jumpy](https://github.com/fishfolk/jumpy), Bones will eventually be the engine of choice for all [Fish Folk](https://github.com/fishfolk) games. It is suitable for any other games with similar requirements.

By default Bones is rendered with [Bevy](https://bevyengine.org), but it is fundamentally engine-agnostic and comes with its own lightweight ECS, asset server and user experience. Bones is officially focused on 2D games and models itself after the likes of [Corgi](https://corgi-engine.moremountains.com/). It can however be used for 3D games as well, if you are willing to create custom rendering integrations.

## Overview

### Bones ECS

Bones is designed around a simple, custom Entity Component System (ECS), designed to make it easier to get a few features that are important to us:

- **Determinism:** Bones ECS is deterministic by default, making it easier to get a re-producible and predictable gameplay.
- **Snapshot/Restore:** The Bones ECS world can be trivially snapshot and restored.
- **Modding/Scripting:** Bones ECS is built on our [`bones_schema`] system, which allows for runtime reflection and the ability to interact with data types defined outside of Rust.

[`bones_schema`]: https://fishfolk.github.io/bones/rustdoc/bones_schema/index.html

Determinism and Snapshot/Restore are also key features for getting excellent **network play** with the rollback networking model, while requiring no changes to the core game loop implementation.

### Bones Lib

The [`bones_lib`] contains the [`bones_ecs`] and the [`bones_asset`] system. It defines the concept
of a [`Game`] which contains all of your game logic and data in a collection of [`Session`]s that
each have their own ECS [`World`].

Bones lib has no rendering components or even math types, it is only concerned with organizing your game logic and assets.

[`bones_lib`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/index.html
[`bones_ecs`]: https://fishfolk.github.io/bones/rustdoc/bones_ecs/index.html
[`bones_asset`]: https://fishfolk.github.io/bones/rustdoc/bones_asset/index.html
[`Game`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/struct.Game.html
[`Session`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/struct.Session.html
[`World`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/ecs/struct.World.html

### Bones Framework

On top of [`bones_lib`] there is the [`bones_framework`], which defines the rendering components and
math types. Right now [`bones_framework`] is focused only on 2D rendering. 3D is not a priority for
us now, but there is no technical limitation preventing community developed 3D rendering components
either on top of [`bones_lib`] directly or as an extension to the [`bones_framework`].

[`bones_framework`]: https://fishfolk.github.io/bones/rustdoc/bones_framework/index.html

### Bones Bevy Renderer

A game created with only the [`bones_framework`] is renderer agnostic, allowing us to create
rendering integrations with other engines. Our official integration is with the [Bevy] engine.

Rendering in the [`bones_framework`] is intentionally simple, and some games may need more advanced
features that aren't supported out of the box. Bones, and it's Bevy integration, are designed so
that you can create custom rendering specific to your needs. That means you can still take advantage
of any fancy new Bevy plugins, or maybe use something other than Bevy entirely!

### Bones Scripting

[`bones_ecs`] is built to be scripted. Effort has also been made to avoid putting unnecessary
performance limitations into the scripting system. Bones comes with an integration with the
[`piccolo`] VM to enable Lua scripting out-of-the-box.

This integration allows Lua scripts to access the ECS world in a way very similar to the
Rust API. Rust components and resources can be annotated with `#[repr(C)]` to enable direct
access by Lua scripts, and if a type cannot be `#[repr(C)]`, you can still manually
create your own Lua bindings for that type.

Allowing both Rust and the scripting language to talk to the _same_ ECS world allows you to easily
blend both languages in your game, and have them interact quite easily in many circumstances. If
a portion of your game needs extra high performance or low-level access, you can use Rust, but
if you want hot reloaded and moddable elements of your game, you can use Lua.

The scripting system is not limited to Lua. Using the simple dynamic API to [`bones_ecs`], you can
create your own integrations to any language or system you desire.

The scripting system is new and work-in-progress, but all of the major things have been
successfully implemented, and it is going to be actively used in Jumpy.

[`piccolo`]: https://github.com/kyren/piccolo/

## Contributing

If you would like to contribute, feel free to reach out on our [Discord](https://discord.gg/4smxjcheE5) server to ask questions!

We also use [TODO Issue][tdi] to automatically create issues from all of our `TODO` comments in code. You can check out the [todo issue list][tdil] to see if there's any thing you'd like to take a hack at.

[tdi]: https://github.com/DerJuulsn/todo-issue
[tdil]: https://github.com/fishfolk/bones/issues?q=is%3Aissue+is%3Aopen+label%3Acode%3Atodo

## Similar Projects

Our architecure has many things in common with these other awesome projects:

- [Gamercade](https://github.com/gamercade-io/) / wasm4
- [Ambient](https://github.com/AmbientRun/Ambient)
- [flecs-polyglot](https://github.com/flecs-hub/flecs-polyglot)
- [Tangle](https://github.com/kettle11/tangle)
- [Godot sandbox #5010](https://github.com/godotengine/godot-proposals/issues/5010)
