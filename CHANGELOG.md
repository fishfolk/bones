# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Documentation

 - <csr-id-baa617bef8918683b0993f3ff2faa60e826afb6f/> add missing Rust API documentation.

### New Features

 - <csr-id-88b47965fb59d4ee2c1748de7d839e08072ae0b2/> add camera shake.
   Adds systems and components for easily adding camera shake.
   
   Ported from the Bevy implementation in `bones_camera_shake`.
 - <csr-id-020c1244cbd27f0a32b8fad6a314bea81ef0e449/> add animation module.
 - <csr-id-ec30508e66dbc4c436a052754f1168419ad20c1a/> add `bones_camera_shake` crate
   Adds the camera shake functionality from [Bomby](https://github.com/fishfolk/bomby).
   
   For the time being it uses `bevy_ecs` and not `bones_ecs`.
 - <csr-id-3724c69a0bb24828d1710380bb8d139e304b7955/> migrate crates from the jumpy repository

### Bug Fixes

 - <csr-id-9de77ff7c9ddcb5af5737553384becbb9483b665/> fix sprite animation bug.
   Fixes the behavior when an atlas sprite's current index is less
   than the starting index of an animated sprite.
   
   Previously it would play the animation from wherever the current
   index happened to be, but it was supposed to skip to the animation
   start frame.

### Refactor

 - <csr-id-db6ad44986098e98b7117aca3b3150749bc5f90a/> temporarily vendor 1D perlin noise.
   We're waiting on the noise crate to publish a release supporting
   1D perlin noise, so in the meantime we vendor the `perlin_1d` function
   and use it directly.

### New Features (BREAKING)

 - <csr-id-e78ed38715945aa180eeb390a20fc08cc19872af/> make `bones_bevy_utils` an optional dependency.
   This reduces dependencies if you want to use `bones_lib` without Bevy.
 - <csr-id-89b44d7b4f64ec266eb0ea674c220e07376a03b7/> add asset integration with bevy.
   This is a big overall change that adds ways to integrate Bones with bevy assets.
 - <csr-id-d7b5711832f6834644fc41ff011af118ce8a9f56/> draft bones_lib architecture.
   Renames `bones` to `bones_lib` ( mostly because `bones` was already taken )
   and adds the `bones_asset`, `bones_bevy_renderer`, `bones_input`, and
   `bones_render` crates.
   
   This sets up the overall structure for the bones library,
   though changes to some aspects of the design are likely to change.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 26 calendar days.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#26](https://github.com/fishfolk/bones/issues/26), [#29](https://github.com/fishfolk/bones/issues/29), [#4](https://github.com/fishfolk/bones/issues/4), [#53](https://github.com/fishfolk/bones/issues/53), [#55](https://github.com/fishfolk/bones/issues/55), [#56](https://github.com/fishfolk/bones/issues/56), [#58](https://github.com/fishfolk/bones/issues/58), [#61](https://github.com/fishfolk/bones/issues/61)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#26](https://github.com/fishfolk/bones/issues/26)**
    - draft bones_lib architecture. ([`d7b5711`](https://github.com/fishfolk/bones/commit/d7b5711832f6834644fc41ff011af118ce8a9f56))
 * **[#29](https://github.com/fishfolk/bones/issues/29)**
    - add asset integration with bevy. ([`89b44d7`](https://github.com/fishfolk/bones/commit/89b44d7b4f64ec266eb0ea674c220e07376a03b7))
 * **[#4](https://github.com/fishfolk/bones/issues/4)**
    - add `bones_camera_shake` crate ([`ec30508`](https://github.com/fishfolk/bones/commit/ec30508e66dbc4c436a052754f1168419ad20c1a))
 * **[#53](https://github.com/fishfolk/bones/issues/53)**
    - make `bones_bevy_utils` an optional dependency. ([`e78ed38`](https://github.com/fishfolk/bones/commit/e78ed38715945aa180eeb390a20fc08cc19872af))
 * **[#55](https://github.com/fishfolk/bones/issues/55)**
    - add missing Rust API documentation. ([`baa617b`](https://github.com/fishfolk/bones/commit/baa617bef8918683b0993f3ff2faa60e826afb6f))
 * **[#56](https://github.com/fishfolk/bones/issues/56)**
    - add camera shake. ([`88b4796`](https://github.com/fishfolk/bones/commit/88b47965fb59d4ee2c1748de7d839e08072ae0b2))
 * **[#58](https://github.com/fishfolk/bones/issues/58)**
    - fix sprite animation bug. ([`9de77ff`](https://github.com/fishfolk/bones/commit/9de77ff7c9ddcb5af5737553384becbb9483b665))
 * **[#61](https://github.com/fishfolk/bones/issues/61)**
    - temporarily vendor 1D perlin noise. ([`db6ad44`](https://github.com/fishfolk/bones/commit/db6ad44986098e98b7117aca3b3150749bc5f90a))
 * **Uncategorized**
    - add animation module. ([`020c124`](https://github.com/fishfolk/bones/commit/020c1244cbd27f0a32b8fad6a314bea81ef0e449))
    - migrate crates from the jumpy repository ([`3724c69`](https://github.com/fishfolk/bones/commit/3724c69a0bb24828d1710380bb8d139e304b7955))
</details>

