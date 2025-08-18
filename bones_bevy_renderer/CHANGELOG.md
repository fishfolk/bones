# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 (2023-06-01)

<csr-id-6825d579672fa508a4c67aa40efa970909f5ff54/>

### Chore

 - <csr-id-6825d579672fa508a4c67aa40efa970909f5ff54/> update bones lib  versioning.

### Documentation

 - <csr-id-3f18051e023a4deb676a5f895f1478beda513f04/> update changelogs.

### New Features

<csr-id-8751bdb1f2f403761e792bf489216aad02beaa92/>
<csr-id-822fe58511e956c91a9c3b1fe338d25799696411/>
<csr-id-605345bd3d4fa2f8f540ae106b114d52c45b904a/>
<csr-id-6abe6ee3587f737966bddb5ab0f003e62aea3291/>

 - <csr-id-3f2e3485f9556cc68eb4c04df34d3aa2c6087330/> upgrade to Bevy 0.10.
 - <csr-id-ad6d073a33dc342d5aed1155488e4681cf1bc782/> add color to atlas sprite.
 - <csr-id-b96133fec89330e3837575c110e587f7e11bf3a6/> add color and sync with bevy
   - Add color type to bones

### Bug Fixes

 - <csr-id-29fd36e25797749b73094b0324389d9777394552/> fix crash while rendering tile maps with out-of-bounds tile indexes.
   This clamps the tile indexes of the tiles to be within
   their atlas's bounds so that the game doesn't crash if a
   tile ends up out-of-bounds while switching tilemaps.

### New Features (BREAKING)

 - <csr-id-00110c27b0aa76ed597c7e4d62bec70cfd1b2a23/> add `from_world` implementation similar to Bevy.
   Allows resources to be added with either a `Default` implementation,
   or a custom `FromWorld` implementation that allows them to derive their,
   value from any other data currently in the world.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 128 calendar days.
 - 128 days passed between releases.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 11 unique issues were worked on: [#100](https://github.com/fishfolk/bones/issues/100), [#105](https://github.com/fishfolk/bones/issues/105), [#110](https://github.com/fishfolk/bones/issues/110), [#111](https://github.com/fishfolk/bones/issues/111), [#114](https://github.com/fishfolk/bones/issues/114), [#122](https://github.com/fishfolk/bones/issues/122), [#124](https://github.com/fishfolk/bones/issues/124), [#76](https://github.com/fishfolk/bones/issues/76), [#92](https://github.com/fishfolk/bones/issues/92), [#95](https://github.com/fishfolk/bones/issues/95), [#97](https://github.com/fishfolk/bones/issues/97)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#100](https://github.com/fishfolk/bones/issues/100)**
    - add custom camera viewport support. ([`8751bdb`](https://github.com/fishfolk/bones/commit/8751bdb1f2f403761e792bf489216aad02beaa92))
 * **[#105](https://github.com/fishfolk/bones/issues/105)**
    - fix crash while rendering tile maps with out-of-bounds tile indexes. ([`29fd36e`](https://github.com/fishfolk/bones/commit/29fd36e25797749b73094b0324389d9777394552))
 * **[#110](https://github.com/fishfolk/bones/issues/110)**
    - add color and sync with bevy ([`b96133f`](https://github.com/fishfolk/bones/commit/b96133fec89330e3837575c110e587f7e11bf3a6))
 * **[#111](https://github.com/fishfolk/bones/issues/111)**
    - update bones lib  versioning. ([`6825d57`](https://github.com/fishfolk/bones/commit/6825d579672fa508a4c67aa40efa970909f5ff54))
 * **[#114](https://github.com/fishfolk/bones/issues/114)**
    - add color to atlas sprite. ([`ad6d073`](https://github.com/fishfolk/bones/commit/ad6d073a33dc342d5aed1155488e4681cf1bc782))
 * **[#122](https://github.com/fishfolk/bones/issues/122)**
    - upgrade to Bevy 0.10. ([`3f2e348`](https://github.com/fishfolk/bones/commit/3f2e3485f9556cc68eb4c04df34d3aa2c6087330))
 * **[#124](https://github.com/fishfolk/bones/issues/124)**
    - update changelogs. ([`3f18051`](https://github.com/fishfolk/bones/commit/3f18051e023a4deb676a5f895f1478beda513f04))
 * **[#76](https://github.com/fishfolk/bones/issues/76)**
    - add 2D line path rendering. ([`6abe6ee`](https://github.com/fishfolk/bones/commit/6abe6ee3587f737966bddb5ab0f003e62aea3291))
 * **[#92](https://github.com/fishfolk/bones/issues/92)**
    - add `from_world` implementation similar to Bevy. ([`00110c2`](https://github.com/fishfolk/bones/commit/00110c27b0aa76ed597c7e4d62bec70cfd1b2a23))
 * **[#95](https://github.com/fishfolk/bones/issues/95)**
    - add time resource + sync system ([`605345b`](https://github.com/fishfolk/bones/commit/605345bd3d4fa2f8f540ae106b114d52c45b904a))
 * **[#97](https://github.com/fishfolk/bones/issues/97)**
    - add helper for advancing the Time a fixed timestep. ([`822fe58`](https://github.com/fishfolk/bones/commit/822fe58511e956c91a9c3b1fe338d25799696411))
 * **Uncategorized**
    - Release bones_render v0.1.1, bones_bevy_renderer v0.1.1 ([`5b33433`](https://github.com/fishfolk/bones/commit/5b3343305a0871914085eb1b98702ef82b84d98f))
</details>

<csr-unknown>
Add color type to Bones SpriteAdd color type to clear colorAdd color type to Path2dSync with Bevy<csr-unknown/>

## 0.1.1 (2023-01-24)

### New Features

 - <csr-id-6abe6ee3587f737966bddb5ab0f003e62aea3291/> add 2D line path rendering.

## 0.1.0 (2023-01-24)

<csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/>
<csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/>
<csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/>
<csr-id-248f80ae2aeea109b1ab14426319af194a64c3d1/>

### Chore

 - <csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/> add missing crate descriptions.

### Other

 - <csr-id-248f80ae2aeea109b1ab14426319af194a64c3d1/> switch to released version of `bevy_simple_tilemap`.
   This temporarily increases our list of Bevy feature dependencies as we wait for the
   [PR](https://github.com/forbjok/bevy_simple_tilemap/pull/9) to reduce the required
   bevy features, but it allows us to publish the crate to crates.io.

### Chore

 - <csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/> generate changelogs for all crates.

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

 - 18 commits contributed to the release over the course of 21 calendar days.
 - 15 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 14 unique issues were worked on: [#26](https://github.com/fishfolk/bones/issues/26), [#29](https://github.com/fishfolk/bones/issues/29), [#30](https://github.com/fishfolk/bones/issues/30), [#31](https://github.com/fishfolk/bones/issues/31), [#35](https://github.com/fishfolk/bones/issues/35), [#37](https://github.com/fishfolk/bones/issues/37), [#40](https://github.com/fishfolk/bones/issues/40), [#43](https://github.com/fishfolk/bones/issues/43), [#45](https://github.com/fishfolk/bones/issues/45), [#51](https://github.com/fishfolk/bones/issues/51), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67), [#71](https://github.com/fishfolk/bones/issues/71)

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
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
 * **[#71](https://github.com/fishfolk/bones/issues/71)**
    - switch to released version of `bevy_simple_tilemap`. ([`248f80a`](https://github.com/fishfolk/bones/commit/248f80ae2aeea109b1ab14426319af194a64c3d1))
 * **Uncategorized**
    - Release bones_bevy_renderer v0.1.0 ([`fd5c4f2`](https://github.com/fishfolk/bones/commit/fd5c4f2b295dafa90d8aa235645ef9aba68b2f70))
    - Release bones_bevy_asset_macros v0.2.0, bones_bevy_asset v0.1.0, bones_bevy_renderer v0.1.0, safety bump 2 crates ([`7f7bb38`](https://github.com/fishfolk/bones/commit/7f7bb38fca7b54fd1ad408bd63f63515d07ef2ab))
    - Release type_ulid_macros v0.1.0, type_ulid v0.1.0, bones_bevy_utils v0.1.0, bones_ecs v0.1.0, bones_asset v0.1.0, bones_input v0.1.0, bones_render v0.1.0, bones_lib v0.1.0 ([`db0333d`](https://github.com/fishfolk/bones/commit/db0333ddacb6f29aed8664db67973e72ea586dce))
    - move entity sync to stage before `CoreStage::PostUpdate`. ([`5116014`](https://github.com/fishfolk/bones/commit/5116014e0fd7f886ba208dd161f567ce021f3f8e))
</details>

