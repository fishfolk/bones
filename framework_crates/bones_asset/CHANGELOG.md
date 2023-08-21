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

 - <csr-id-3f2e3485f9556cc68eb4c04df34d3aa2c6087330/> upgrade to Bevy 0.10.
 - <csr-id-7e00c6e7b6300054ffeeebd186b5adf96b8aa10b/> implement `Serialize` for `Handle<T>` and `UntypedHandle`.

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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 103 calendar days.
 - 133 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#101](https://github.com/fishfolk/bones/issues/101), [#106](https://github.com/fishfolk/bones/issues/106), [#111](https://github.com/fishfolk/bones/issues/111), [#122](https://github.com/fishfolk/bones/issues/122), [#124](https://github.com/fishfolk/bones/issues/124)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#101](https://github.com/fishfolk/bones/issues/101)**
    - implement `Serialize` for `Handle<T>` and `UntypedHandle`. ([`7e00c6e`](https://github.com/fishfolk/bones/commit/7e00c6e7b6300054ffeeebd186b5adf96b8aa10b))
 * **[#106](https://github.com/fishfolk/bones/issues/106)**
    - makes bones asset path representation more consistent. ([`632ef4e`](https://github.com/fishfolk/bones/commit/632ef4e2d7647f6cb704a1b5eaeb2fbba9562314))
 * **[#111](https://github.com/fishfolk/bones/issues/111)**
    - update bones lib  versioning. ([`6825d57`](https://github.com/fishfolk/bones/commit/6825d579672fa508a4c67aa40efa970909f5ff54))
 * **[#122](https://github.com/fishfolk/bones/issues/122)**
    - upgrade to Bevy 0.10. ([`3f2e348`](https://github.com/fishfolk/bones/commit/3f2e3485f9556cc68eb4c04df34d3aa2c6087330))
 * **[#124](https://github.com/fishfolk/bones/issues/124)**
    - update changelogs. ([`3f18051`](https://github.com/fishfolk/bones/commit/3f18051e023a4deb676a5f895f1478beda513f04))
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

 - <csr-id-604aa8a5d0c98930a6ccd64d27f5e76c55da451c/> add optional bones_has_load_progress integration.

### New Features (BREAKING)

 - <csr-id-89b44d7b4f64ec266eb0ea674c220e07376a03b7/> add asset integration with bevy.
   This is a big overall change that adds ways to integrate Bones with bevy assets.
 - <csr-id-59f5e67d42de57a33dd302443a8a04427126a5be/> have `TypeUlid` trait require an associated constant instead of a function.
   This makes it possible to access the type's Ulid at compile time,
   possibly in const functions.
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

 - 10 commits contributed to the release over the course of 16 calendar days.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#26](https://github.com/fishfolk/bones/issues/26), [#28](https://github.com/fishfolk/bones/issues/28), [#29](https://github.com/fishfolk/bones/issues/29), [#37](https://github.com/fishfolk/bones/issues/37), [#38](https://github.com/fishfolk/bones/issues/38), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#26](https://github.com/fishfolk/bones/issues/26)**
    - draft bones_lib architecture. ([`d7b5711`](https://github.com/fishfolk/bones/commit/d7b5711832f6834644fc41ff011af118ce8a9f56))
 * **[#28](https://github.com/fishfolk/bones/issues/28)**
    - have `TypeUlid` trait require an associated constant instead of a function. ([`59f5e67`](https://github.com/fishfolk/bones/commit/59f5e67d42de57a33dd302443a8a04427126a5be))
 * **[#29](https://github.com/fishfolk/bones/issues/29)**
    - add asset integration with bevy. ([`89b44d7`](https://github.com/fishfolk/bones/commit/89b44d7b4f64ec266eb0ea674c220e07376a03b7))
 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#38](https://github.com/fishfolk/bones/issues/38)**
    - add optional bones_has_load_progress integration. ([`604aa8a`](https://github.com/fishfolk/bones/commit/604aa8a5d0c98930a6ccd64d27f5e76c55da451c))
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

