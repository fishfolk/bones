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

 - <csr-id-c0a14c5681a82d8e2db725a678b3dbccfa8a80b4/> implement `BonesBevyAssetLoad` for more types.
   Added implementations for `Option`, `HashMap`,
   and `bevy_utils::HashMap` when the values implement
   `BonesBevyAssetLoad`.
 - <csr-id-7fd1c592c61e3032d803b8f70364b826b4a9ebaf/> add extra derive support & type implementations.
   - The derive macro for `BonesBevyAssetLoad` can now be used on enums.
   - Added more implementations of `BonesBevyAssetLoad` for primitive types.
 - <csr-id-3206a4d9559df5e9aafdc22e7c464308e3a9eac7/> add derive macro for `BonesBevyAssetLoad`.
   This makes it easier to nest asset structs that have handles that need loading.

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

 - 9 commits contributed to the release over the course of 14 calendar days.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#29](https://github.com/fishfolk/bones/issues/29), [#33](https://github.com/fishfolk/bones/issues/33), [#37](https://github.com/fishfolk/bones/issues/37), [#39](https://github.com/fishfolk/bones/issues/39), [#41](https://github.com/fishfolk/bones/issues/41), [#52](https://github.com/fishfolk/bones/issues/52), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65)

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
 * **Uncategorized**
    - implement `BonesBevyAssetLoad` for more types. ([`c0a14c5`](https://github.com/fishfolk/bones/commit/c0a14c5681a82d8e2db725a678b3dbccfa8a80b4))
</details>

