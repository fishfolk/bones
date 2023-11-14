# Bones

<img src="https://raw.githubusercontent.com/fishfolk/bones_branding/main/sources/logo.svg" align="right" width="300px" />

[![Documentation](https://img.shields.io/badge/documentation-fishfolk.org-green.svg?labelColor=1e1c24&color=f3ee7a)](https://fishfolk.org/bones/overview/introduction/)
[![Crates.io](https://img.shields.io/crates/v/bones_lib?labelColor=1e1c24)](https://crates.io/crates/bones_lib)
[![docs.rs](https://img.shields.io/docsrs/bones_framework?label=API%20Docs&labelColor=1e1c24)](https://docs.rs/bones_framework)
[![Main Branch Docs](https://img.shields.io/badge/API_Docs-Main_Branch-blue?labelColor=1e1c24&color=red)](https://fishfolk.github.io/bones/rustdoc/bones_framework/index.html)
[![License](https://img.shields.io/badge/License-MIT%20or%20Apache%202-green.svg?label=license&labelColor=1e1c24&color=34925e)](./LICENSE)
[![Discord](https://img.shields.io/badge/chat-on%20discord-green.svg?logo=discord&logoColor=fff&labelColor=1e1c24&color=8d5b3f)][Discord]

A work-in-progress, simple, and easy-to-use game engine that can be rendered with [Bevy].

Used in the [Jumpy] game, and will possibly be used in other FishFolk games in the future.

Check out [Fishfolk.org] for more documentation and tutorials.

[fishfolk.org]: https://fishfolk.org
[bevy]: https://bevyengine.org
[jumpy]: https://github.com/fishfolk/jumpy
[discord]: https://discord.gg/4smxjcheE5
[revolt]: https://weird.dev/invite/ZagXxrS4

## Overview

### Bones ECS

Bones is designed around a simple, custom Entity Component System ( ECS ), designed to make it easier to get a few features that are important to us:

- **Determinism:** Bones ECS is deterministic by default, making it easier to get a re-producible and predictable gameplay.
- **Snapshot/Restore:** The Bones ECS world can be trivially snapshot and restored.
- **Modding/Scripting ( work-in-progress ):** Bones ECS is built on our [`bones_schema`] system, which allows for runtime reflection and the ability to interact with data types defined outside of Rust.

[`bones_schema`]: https://fishfolk.github.io/bones/rustdoc/bones_schema/index.html

Determinism and Snapshot/Restore are also key features for getting excellent **network play** with the rollback networking model, while requiring no changes to the core game loop implementation.

### Bones Lib

The [`bones_lib`] contains the [`bones_ecs`] and the [`bones_asset`] system. It defines the concept of a [`Game`] which contains all of your game logic and data in a collection of [`Session`]s that each have their own ECS [`World`].

Bones lib has no rendering components or even math types, it is only concerned with organizing your game logic and assets.

[`bones_lib`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/index.html
[`bones_ecs`]: https://fishfolk.github.io/bones/rustdoc/bones_ecs/index.html
[`bones_asset`]: https://fishfolk.github.io/bones/rustdoc/bones_asset/index.html
[`Game`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/struct.Game.html
[`Session`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/struct.Session.html
[`World`]: https://fishfolk.github.io/bones/rustdoc/bones_lib/ecs/struct.World.html

### Bones Framework

On top of [`bones_lib`] there is the [`bones_framework`], which defines the rendering components and math types. Right now [`bones_framework`] is focused only on 2D rendering. 3D is not a priority for us now, but there is no technical limitation preventing community developed 3D rendering components either on top of [`bones_lib`] directly or as an extension to the [`bones_framework`].

[`bones_framework`]: https://fishfolk.github.io/bones/rustdoc/bones_framework/index.html

### Bones Bevy Renderer

A game created with the [`bones_framework`] is renderer agnostic. This allows us to create rendering integrations with other engines. Our official integration is with the [Bevy] engine. The [`bones_bevy_renderer`] allows you to create a Bevy `App` for rendering a [`bones_framework`] game.

This also allows you to create custom extensions to the Bevy renderer, if you need a bones integration with a feature not supported out-of-the-box in the [`bones_framework`].

[`bones_bevy_renderer`]: https://fishfolk.github.io/bones/rustdoc/bones_bevy_renderer/index.html

## Contributing

If you would like to contribute, feel free to reach out on our [Discord] or [Revolt] server to ask questions!

We also use [TODO Issue][tdi] to automatically create issues from all of our `TODO` comments in code. You can check out the [todo issue list][tdil] to see if there's any thing you'd like to take a hack at.

[tdi]: https://github.com/DerJuulsn/todo-issue
[tdil]: https://github.com/fishfolk/bones/issues?q=is%3Aissue+is%3Aopen+label%3Acode%3Atodo

## Similar Projects

Our architecure has many things in common with these other awesome projects:

- [Gamercade](https://github.com/gamercade-io/) / wasm4
- [Ambient](https://github.com/AmbientRun/Ambient)
- [Tangle](https://github.com/kettle11/tangle)
- [Godot sandbox #5010](https://github.com/godotengine/godot-proposals/issues/5010)
