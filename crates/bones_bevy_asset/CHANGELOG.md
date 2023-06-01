# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 (2023-06-01)

<csr-id-c57a2089f4dcf6bd63e8f0e0609cf6ff3506084f/>

### Chore

 - <csr-id-c57a2089f4dcf6bd63e8f0e0609cf6ff3506084f/> add serde to bones color.
   Add serde Serialize / Deserailize to bones color.

### Documentation

 - <csr-id-3f18051e023a4deb676a5f895f1478beda513f04/> update changelogs.

### New Features

 - <csr-id-3f2e3485f9556cc68eb4c04df34d3aa2c6087330/> upgrade to Bevy 0.10.
 - <csr-id-e7330d9cdb590564c3c01255401d8530425e18f0/> implement `BonesBevyAssetLoad` for `Duration`.
 - <csr-id-605345bd3d4fa2f8f540ae106b114d52c45b904a/> add time resource + sync system
 - <csr-id-a699f5d9254037d6127becae77f09527759fd408/> implement BonesBevyAssetLoad for `Key`.
   This makes it easier to deserialize `Key`s in bevy assets.

### Bug Fixes

 - <csr-id-632ef4e2d7647f6cb704a1b5eaeb2fbba9562314/> makes bones asset path representation more consistent.
   Previously the normalize method on a bones path would remove the leading
   `/` to make it support Bevy paths, which can't start with a `/`, but
   this was not consistent with the way that the handle was serialized.
   
   Now, the bones path representations always maintain the leading `/` to
   indicate a root path, and the leading `/` is removed when converting to
   a Bevy handle.
   
   This fixes issues run into when trying to read serialized bones handles
   during map saving in Jumpy.

### New Features (BREAKING)

 - <csr-id-00110c27b0aa76ed597c7e4d62bec70cfd1b2a23/> add `from_world` implementation similar to Bevy.
   Allows resources to be added with either a `Default` implementation,
   or a custom `FromWorld` implementation that allows them to derive their,
   value from any other data currently in the world.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 119 calendar days.
 - 133 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#102](https://github.com/fishfolk/bones/issues/102), [#106](https://github.com/fishfolk/bones/issues/106), [#112](https://github.com/fishfolk/bones/issues/112), [#122](https://github.com/fishfolk/bones/issues/122), [#124](https://github.com/fishfolk/bones/issues/124), [#83](https://github.com/fishfolk/bones/issues/83), [#92](https://github.com/fishfolk/bones/issues/92), [#95](https://github.com/fishfolk/bones/issues/95)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#102](https://github.com/fishfolk/bones/issues/102)**
    - implement `BonesBevyAssetLoad` for `Duration`. ([`e7330d9`](https://github.com/fishfolk/bones/commit/e7330d9cdb590564c3c01255401d8530425e18f0))
 * **[#106](https://github.com/fishfolk/bones/issues/106)**
    - makes bones asset path representation more consistent. ([`632ef4e`](https://github.com/fishfolk/bones/commit/632ef4e2d7647f6cb704a1b5eaeb2fbba9562314))
 * **[#112](https://github.com/fishfolk/bones/issues/112)**
    - add serde to bones color. ([`c57a208`](https://github.com/fishfolk/bones/commit/c57a2089f4dcf6bd63e8f0e0609cf6ff3506084f))
 * **[#122](https://github.com/fishfolk/bones/issues/122)**
    - upgrade to Bevy 0.10. ([`3f2e348`](https://github.com/fishfolk/bones/commit/3f2e3485f9556cc68eb4c04df34d3aa2c6087330))
 * **[#124](https://github.com/fishfolk/bones/issues/124)**
    - update changelogs. ([`3f18051`](https://github.com/fishfolk/bones/commit/3f18051e023a4deb676a5f895f1478beda513f04))
 * **[#83](https://github.com/fishfolk/bones/issues/83)**
    - implement BonesBevyAssetLoad for `Key`. ([`a699f5d`](https://github.com/fishfolk/bones/commit/a699f5d9254037d6127becae77f09527759fd408))
 * **[#92](https://github.com/fishfolk/bones/issues/92)**
    - add `from_world` implementation similar to Bevy. ([`00110c2`](https://github.com/fishfolk/bones/commit/00110c27b0aa76ed597c7e4d62bec70cfd1b2a23))
 * **[#95](https://github.com/fishfolk/bones/issues/95)**
    - add time resource + sync system ([`605345b`](https://github.com/fishfolk/bones/commit/605345bd3d4fa2f8f540ae106b114d52c45b904a))
</details>

## 0.1.0 (2023-01-18)

<csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/>
<csr-id-de43e3cf45b9108bebecd4196aa7524c87758e35/>
<csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/>
<csr-id-ef12c3fb681cc826199b1564e1a033a56a5ce2d4/>
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

<csr-id-3206a4d9559df5e9aafdc22e7c464308e3a9eac7/>

 - <csr-id-c0a14c5681a82d8e2db725a678b3dbccfa8a80b4/> implement `BonesBevyAssetLoad` for more types.
   Added implementations for `Option`, `HashMap`,
   and `bevy_utils::HashMap` when the values implement
   `BonesBevyAssetLoad`.
 - <csr-id-7fd1c592c61e3032d803b8f70364b826b4a9ebaf/> add extra derive support & type implementations.
   - The derive macro for `BonesBevyAssetLoad` can now be used on enums.

### Style

 - <csr-id-de43e3cf45b9108bebecd4196aa7524c87758e35/> use `if let` statement instead of `Option::map()`

### New Features (BREAKING)

 - <csr-id-89b44d7b4f64ec266eb0ea674c220e07376a03b7/> add asset integration with bevy.
   This is a big overall change that adds ways to integrate Bones with bevy assets.

### Refactor (BREAKING)

 - <csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/> prepare for release.
   - Remove `bones_has_load_progress`: for now we don't use it, and if we
     want something similar we will work it into `bones_bevy_asset`.
   - Remove `bones_camera_shake`: it was moved into `bones_lib::camera`.
   - Add version numbers for all local dependencies.
 - <csr-id-ef12c3fb681cc826199b1564e1a033a56a5ce2d4/> make world in `BevyWorld` resource optional.
   Since the bevy world can't be cloned, we previously had it in
   an Arc, but that didn't play nicely with world snapshots.
   
   Now the bevy world inside the `BevyWorld` resource is an
   option, with the `BevyAssets` system param panicking if it
   doesn't find the world when it needs it.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 14 calendar days.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 9 unique issues were worked on: [#29](https://github.com/fishfolk/bones/issues/29), [#33](https://github.com/fishfolk/bones/issues/33), [#37](https://github.com/fishfolk/bones/issues/37), [#39](https://github.com/fishfolk/bones/issues/39), [#41](https://github.com/fishfolk/bones/issues/41), [#52](https://github.com/fishfolk/bones/issues/52), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#29](https://github.com/fishfolk/bones/issues/29)**
    - add asset integration with bevy. ([`89b44d7`](https://github.com/fishfolk/bones/commit/89b44d7b4f64ec266eb0ea674c220e07376a03b7))
 * **[#33](https://github.com/fishfolk/bones/issues/33)**
    - add derive macro for `BonesBevyAssetLoad`. ([`3206a4d`](https://github.com/fishfolk/bones/commit/3206a4d9559df5e9aafdc22e7c464308e3a9eac7))
 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#39](https://github.com/fishfolk/bones/issues/39)**
    - add extra derive support & type implementations. ([`7fd1c59`](https://github.com/fishfolk/bones/commit/7fd1c592c61e3032d803b8f70364b826b4a9ebaf))
 * **[#41](https://github.com/fishfolk/bones/issues/41)**
    - make world in `BevyWorld` resource optional. ([`ef12c3f`](https://github.com/fishfolk/bones/commit/ef12c3fb681cc826199b1564e1a033a56a5ce2d4))
 * **[#52](https://github.com/fishfolk/bones/issues/52)**
    - use `if let` statement instead of `Option::map()` ([`de43e3c`](https://github.com/fishfolk/bones/commit/de43e3cf45b9108bebecd4196aa7524c87758e35))
 * **[#63](https://github.com/fishfolk/bones/issues/63)**
    - prepare for release. ([`ae0a761`](https://github.com/fishfolk/bones/commit/ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31))
 * **[#65](https://github.com/fishfolk/bones/issues/65)**
    - add missing crate descriptions. ([`2725246`](https://github.com/fishfolk/bones/commit/27252465ad0506ff2f8c377531fa079ec64d1750))
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
 * **Uncategorized**
    - Release bones_bevy_asset_macros v0.2.0, bones_bevy_asset v0.1.0, bones_bevy_renderer v0.1.0, safety bump 2 crates ([`7f7bb38`](https://github.com/fishfolk/bones/commit/7f7bb38fca7b54fd1ad408bd63f63515d07ef2ab))
    - Release type_ulid_macros v0.1.0, type_ulid v0.1.0, bones_bevy_utils v0.1.0, bones_ecs v0.1.0, bones_asset v0.1.0, bones_input v0.1.0, bones_render v0.1.0, bones_lib v0.1.0 ([`db0333d`](https://github.com/fishfolk/bones/commit/db0333ddacb6f29aed8664db67973e72ea586dce))
    - implement `BonesBevyAssetLoad` for more types. ([`c0a14c5`](https://github.com/fishfolk/bones/commit/c0a14c5681a82d8e2db725a678b3dbccfa8a80b4))
</details>

