# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Chore

 - <csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/> add missing crate descriptions.

### Documentation

 - <csr-id-a69389412d22b8cb48bab0ed96d739b0fee35348/> document source repository in cargo manifest.
   The `repository` key under `bones_ecs` previously pointed to https://github.com/fishfolk/jumpy.
   
   This updates this to point to the bones repo, and also adds the `repository` key to the other
   crates in the repository.

### New Features

 - <csr-id-34c5ecc7b2f37b99fa3b415558a858ec26ec1bba/> add resource for controlling the clear color.
 - <csr-id-0a7fec655cd951f18bb7e8e134a534d3e79999c1/> implement tilemap rendering.
 - <csr-id-d43b6ec3aa5ef9fc587b4463d00445f43acec2ce/> implement atlas sprite rendering.
   Adds the `bones_render` types for atlas sprites,
   and renders them in `bones_bevy_renderer`.
   
   This also adds an asset loader for `.atlas.yaml`/`.atlas.json` files
   which can be used when you need to load a `Handle<Atlas>`
   in a `BonesBevyAsset` struct.

### Bug Fixes

 - <csr-id-e3d70fa9cf2bb6f1346750dbb7f7b968d4fd8387/> fix Error When Re-Creating World Resource
   This fixes a panic that would happen if you added a bones world resource,
   removed it, and re-added it.
 - <csr-id-1f826dd939dfcb1fd7045f634b8008fa3ce3acff/> fix tile rendering offset.
   The previous tile rendering fix fixed some of the issue,
   but it used the wrong dimensions when off-setting the tile
   placement so that the tiles render from their bottom-left corner.
 - <csr-id-f8f41ede20fa921f10404be22c24062fafef5eae/> fix bugs in tilemap renderer.
   - Fix issue where tiles were being rendered off into the far right side
     of the map.
   - Fix issue where tiles were not being cleared from the previous frame
     before updating them for the current frame.

### New Features (BREAKING)

 - <csr-id-b80cf486bd66a160031072ba1a616bac0195052a/> remove join! macro and improve iteration API.
   We will add a more ergonomic replacement for the `join!` macro later,
   but for now we make it easier to use the raw bitset iteration APIs,
   which previously required a frustrating use of `Rc`.
   
   Removing the `Rc` broke the `join!` macro, but I think there is a better way to
   create a join API, so it isn't worth fixing yet.
   
   This also improves the ergonomics of the bitset iterators by having them
   yield their items directly instead of yielding `Option`s that must be filtered out.
 - <csr-id-89b44d7b4f64ec266eb0ea674c220e07376a03b7/> add asset integration with bevy.
   This is a big overall change that adds ways to integrate Bones with bevy assets.
 - <csr-id-d7b5711832f6834644fc41ff011af118ce8a9f56/> draft bones_lib architecture.
   Renames `bones` to `bones_lib` ( mostly because `bones` was already taken )
   and adds the `bones_asset`, `bones_bevy_renderer`, `bones_input`, and
   `bones_render` crates.
   
   This sets up the overall structure for the bones library,
   though changes to some aspects of the design are likely to change.

### Bug Fixes (BREAKING)

 - <csr-id-5116014e0fd7f886ba208dd161f567ce021f3f8e/> move entity sync to stage before `CoreStage::PostUpdate`.
   This fixes problems where a sprite is moved and it's global transform
   doesn't update until the next frame, causing flickering.

### Refactor (BREAKING)

 - <csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/> prepare for release.
   - Remove `bones_has_load_progress`: for now we don't use it, and if we
     want something similar we will work it into `bones_bevy_asset`.
   - Remove `bones_camera_shake`: it was moved into `bones_lib::camera`.
   - Add version numbers for all local dependencies.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release over the course of 16 calendar days.
 - 13 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#26](https://github.com/fishfolk/bones/issues/26), [#29](https://github.com/fishfolk/bones/issues/29), [#30](https://github.com/fishfolk/bones/issues/30), [#31](https://github.com/fishfolk/bones/issues/31), [#35](https://github.com/fishfolk/bones/issues/35), [#37](https://github.com/fishfolk/bones/issues/37), [#40](https://github.com/fishfolk/bones/issues/40), [#43](https://github.com/fishfolk/bones/issues/43), [#45](https://github.com/fishfolk/bones/issues/45), [#51](https://github.com/fishfolk/bones/issues/51), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#26](https://github.com/fishfolk/bones/issues/26)**
    - draft bones_lib architecture. ([`d7b5711`](https://github.com/fishfolk/bones/commit/d7b5711832f6834644fc41ff011af118ce8a9f56))
 * **[#29](https://github.com/fishfolk/bones/issues/29)**
    - add asset integration with bevy. ([`89b44d7`](https://github.com/fishfolk/bones/commit/89b44d7b4f64ec266eb0ea674c220e07376a03b7))
 * **[#30](https://github.com/fishfolk/bones/issues/30)**
    - remove join! macro and improve iteration API. ([`b80cf48`](https://github.com/fishfolk/bones/commit/b80cf486bd66a160031072ba1a616bac0195052a))
 * **[#31](https://github.com/fishfolk/bones/issues/31)**
    - implement atlas sprite rendering. ([`d43b6ec`](https://github.com/fishfolk/bones/commit/d43b6ec3aa5ef9fc587b4463d00445f43acec2ce))
 * **[#35](https://github.com/fishfolk/bones/issues/35)**
    - implement tilemap rendering. ([`0a7fec6`](https://github.com/fishfolk/bones/commit/0a7fec655cd951f18bb7e8e134a534d3e79999c1))
 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#40](https://github.com/fishfolk/bones/issues/40)**
    - fix bugs in tilemap renderer. ([`f8f41ed`](https://github.com/fishfolk/bones/commit/f8f41ede20fa921f10404be22c24062fafef5eae))
 * **[#43](https://github.com/fishfolk/bones/issues/43)**
    - add resource for controlling the clear color. ([`34c5ecc`](https://github.com/fishfolk/bones/commit/34c5ecc7b2f37b99fa3b415558a858ec26ec1bba))
 * **[#45](https://github.com/fishfolk/bones/issues/45)**
    - fix tile rendering offset. ([`1f826dd`](https://github.com/fishfolk/bones/commit/1f826dd939dfcb1fd7045f634b8008fa3ce3acff))
 * **[#51](https://github.com/fishfolk/bones/issues/51)**
    - fix Error When Re-Creating World Resource ([`e3d70fa`](https://github.com/fishfolk/bones/commit/e3d70fa9cf2bb6f1346750dbb7f7b968d4fd8387))
 * **[#63](https://github.com/fishfolk/bones/issues/63)**
    - prepare for release. ([`ae0a761`](https://github.com/fishfolk/bones/commit/ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31))
 * **[#65](https://github.com/fishfolk/bones/issues/65)**
    - add missing crate descriptions. ([`2725246`](https://github.com/fishfolk/bones/commit/27252465ad0506ff2f8c377531fa079ec64d1750))
 * **Uncategorized**
    - move entity sync to stage before `CoreStage::PostUpdate`. ([`5116014`](https://github.com/fishfolk/bones/commit/5116014e0fd7f886ba208dd161f567ce021f3f8e))
</details>

