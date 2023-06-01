# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 (2023-06-01)

### Documentation

 - <csr-id-3f18051e023a4deb676a5f895f1478beda513f04/> update changelogs.

### New Features

 - <csr-id-3f2e3485f9556cc68eb4c04df34d3aa2c6087330/> upgrade to Bevy 0.10.
 - <csr-id-822fe58511e956c91a9c3b1fe338d25799696411/> add helper for advancing the Time a fixed timestep.
   Adds a configurable `sync_time` option to the `BonesRendererPlugin`
   so that you can disable the automatic time synchronization in favor of a
   custom implementation.
   
   It also moves the time synchronization to a new stage that happens after
   `CoreStage::First` so that the time will be in sync during the
   `PreUpdate` and `Update` stages.
 - <csr-id-605345bd3d4fa2f8f540ae106b114d52c45b904a/> add time resource + sync system

### Bug Fixes (BREAKING)

 - <csr-id-5849caf064259df0530bf15f3a1985170875e225/> use `instant` crate for WASM compatibility in `bones_input`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 106 calendar days.
 - 133 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#122](https://github.com/fishfolk/bones/issues/122), [#124](https://github.com/fishfolk/bones/issues/124), [#95](https://github.com/fishfolk/bones/issues/95), [#97](https://github.com/fishfolk/bones/issues/97), [#98](https://github.com/fishfolk/bones/issues/98)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#122](https://github.com/fishfolk/bones/issues/122)**
    - upgrade to Bevy 0.10. ([`3f2e348`](https://github.com/fishfolk/bones/commit/3f2e3485f9556cc68eb4c04df34d3aa2c6087330))
 * **[#124](https://github.com/fishfolk/bones/issues/124)**
    - update changelogs. ([`3f18051`](https://github.com/fishfolk/bones/commit/3f18051e023a4deb676a5f895f1478beda513f04))
 * **[#95](https://github.com/fishfolk/bones/issues/95)**
    - add time resource + sync system ([`605345b`](https://github.com/fishfolk/bones/commit/605345bd3d4fa2f8f540ae106b114d52c45b904a))
 * **[#97](https://github.com/fishfolk/bones/issues/97)**
    - add helper for advancing the Time a fixed timestep. ([`822fe58`](https://github.com/fishfolk/bones/commit/822fe58511e956c91a9c3b1fe338d25799696411))
 * **[#98](https://github.com/fishfolk/bones/issues/98)**
    - use `instant` crate for WASM compatibility in `bones_input`. ([`5849caf`](https://github.com/fishfolk/bones/commit/5849caf064259df0530bf15f3a1985170875e225))
</details>

## 0.1.0 (2023-01-18)

<csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/>
<csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/>
<csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/>

### Chore

 - <csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/> add missing crate descriptions.

### Chore

 - <csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/> generate changelogs for all crates.

### Documentation

 - <csr-id-a69389412d22b8cb48bab0ed96d739b0fee35348/> document source repository in cargo manifest.
   The `repository` key under `bones_ecs` previously pointed to https://github.com/fishfolk/jumpy.
   
   This updates this to point to the bones repo, and also adds the `repository` key to the other
   crates in the repository.

### New Features

 - <csr-id-a85d2828c10a044524f01b0938ead015c530986f/> add `Window` input containing window size.

### New Features (BREAKING)

 - <csr-id-d7b5711832f6834644fc41ff011af118ce8a9f56/> draft bones_lib architecture.
   Renames `bones` to `bones_lib` ( mostly because `bones` was already taken )
   and adds the `bones_asset`, `bones_bevy_renderer`, `bones_input`, and
   `bones_render` crates.
   
   This sets up the overall structure for the bones library,
   though changes to some aspects of the design are likely to change.

### Refactor (BREAKING)

 - <csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/> prepare for release.
   - Remove `bones_has_load_progress`: for now we don't use it, and if we
     want something similar we will work it into `bones_bevy_asset`.
   - Remove `bones_camera_shake`: it was moved into `bones_lib::camera`.
   - Add version numbers for all local dependencies.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 16 calendar days.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#26](https://github.com/fishfolk/bones/issues/26), [#37](https://github.com/fishfolk/bones/issues/37), [#48](https://github.com/fishfolk/bones/issues/48), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#26](https://github.com/fishfolk/bones/issues/26)**
    - draft bones_lib architecture. ([`d7b5711`](https://github.com/fishfolk/bones/commit/d7b5711832f6834644fc41ff011af118ce8a9f56))
 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#48](https://github.com/fishfolk/bones/issues/48)**
    - add `Window` input containing window size. ([`a85d282`](https://github.com/fishfolk/bones/commit/a85d2828c10a044524f01b0938ead015c530986f))
 * **[#63](https://github.com/fishfolk/bones/issues/63)**
    - prepare for release. ([`ae0a761`](https://github.com/fishfolk/bones/commit/ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31))
 * **[#65](https://github.com/fishfolk/bones/issues/65)**
    - add missing crate descriptions. ([`2725246`](https://github.com/fishfolk/bones/commit/27252465ad0506ff2f8c377531fa079ec64d1750))
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
 * **Uncategorized**
    - Release type_ulid v0.1.0, bones_bevy_utils v0.1.0, bones_ecs v0.1.0, bones_asset v0.1.0, bones_input v0.1.0, bones_render v0.1.0, bones_lib v0.1.0 ([`69713d7`](https://github.com/fishfolk/bones/commit/69713d7da8024ee4b3017b563f031880009c90ee))
    - Release type_ulid_macros v0.1.0, type_ulid v0.1.0, bones_bevy_utils v0.1.0, bones_ecs v0.1.0, bones_asset v0.1.0, bones_input v0.1.0, bones_render v0.1.0, bones_lib v0.1.0 ([`db0333d`](https://github.com/fishfolk/bones/commit/db0333ddacb6f29aed8664db67973e72ea586dce))
</details>

