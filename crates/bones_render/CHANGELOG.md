# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2023-01-18)

<csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/>
<csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/>

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

 - <csr-id-e76de9db7fa7116b9e1237c301e939f22de5e370/> implement `Default` for sprite components.
 - <csr-id-a16443a0860e46bf44fed534648af85d540f975a/> add modules to the prelude.
 - <csr-id-c61b84553b8e4322a96de377b1b8f132894166db/> add audio module.
 - <csr-id-2a52b688bb9b8920c9b0001fe536c4f82c86b127/> add a helper method for creating an `AtlasSprite`.
 - <csr-id-6d813ccca3ea98f61fed0bdeaa2f15997c071b12/> add utility `Key` datatype.
   The `Key` datatype is a small, stack-allocated identifier,
   similar to a string, but avoiding the heap allocation.
   
   This type might better be moved to a utility crate,
   but since one doesn't exist yet for bones alone we
   put it in bones_render for now.
 - <csr-id-34c5ecc7b2f37b99fa3b415558a858ec26ec1bba/> add resource for controlling the clear color.
 - <csr-id-0a7fec655cd951f18bb7e8e134a534d3e79999c1/> implement tilemap rendering.
 - <csr-id-f11fc28734a7bb402fe5485aca3e1b0aab13cfc2/> add helper methods for creating `Transform`s.
   Adds helpers for creating transforms from either a translation, a rotation, or a scale.
 - <csr-id-d43b6ec3aa5ef9fc587b4463d00445f43acec2ce/> implement atlas sprite rendering.
   Adds the `bones_render` types for atlas sprites,
   and renders them in `bones_bevy_renderer`.
   
   This also adds an asset loader for `.atlas.yaml`/`.atlas.json` files
   which can be used when you need to load a `Handle<Atlas>`
   in a `BonesBevyAsset` struct.

### New Features (BREAKING)

 - <csr-id-2c7d5f4372291a9c6e81bdc19a511e4fb0a45e8c/> make `Key::new()` const and add `key!` macro for const construction.
 - <csr-id-89b44d7b4f64ec266eb0ea674c220e07376a03b7/> add asset integration with bevy.
   This is a big overall change that adds ways to integrate Bones with bevy assets.
 - <csr-id-d7b5711832f6834644fc41ff011af118ce8a9f56/> draft bones_lib architecture.
   Renames `bones` to `bones_lib` ( mostly because `bones` was already taken )
   and adds the `bones_asset`, `bones_bevy_renderer`, `bones_input`, and
   `bones_render` crates.
   
   This sets up the overall structure for the bones library,
   though changes to some aspects of the design are likely to change.

### Bug Fixes (BREAKING)

 - <csr-id-6419a8cc1652c10d94192816cbd2f5199624faa5/> fix panics in `TileLayer` by returning an `Option<Tile>` from `get()`.

### Refactor (BREAKING)

 - <csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/> prepare for release.
   - Remove `bones_has_load_progress`: for now we don't use it, and if we
     want something similar we will work it into `bones_bevy_asset`.
   - Remove `bones_camera_shake`: it was moved into `bones_lib::camera`.
   - Add version numbers for all local dependencies.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release over the course of 16 calendar days.
 - 17 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#26](https://github.com/fishfolk/bones/issues/26), [#29](https://github.com/fishfolk/bones/issues/29), [#31](https://github.com/fishfolk/bones/issues/31), [#34](https://github.com/fishfolk/bones/issues/34), [#35](https://github.com/fishfolk/bones/issues/35), [#37](https://github.com/fishfolk/bones/issues/37), [#43](https://github.com/fishfolk/bones/issues/43), [#44](https://github.com/fishfolk/bones/issues/44), [#54](https://github.com/fishfolk/bones/issues/54), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#26](https://github.com/fishfolk/bones/issues/26)**
    - draft bones_lib architecture. ([`d7b5711`](https://github.com/fishfolk/bones/commit/d7b5711832f6834644fc41ff011af118ce8a9f56))
 * **[#29](https://github.com/fishfolk/bones/issues/29)**
    - add asset integration with bevy. ([`89b44d7`](https://github.com/fishfolk/bones/commit/89b44d7b4f64ec266eb0ea674c220e07376a03b7))
 * **[#31](https://github.com/fishfolk/bones/issues/31)**
    - implement atlas sprite rendering. ([`d43b6ec`](https://github.com/fishfolk/bones/commit/d43b6ec3aa5ef9fc587b4463d00445f43acec2ce))
 * **[#34](https://github.com/fishfolk/bones/issues/34)**
    - add helper methods for creating `Transform`s. ([`f11fc28`](https://github.com/fishfolk/bones/commit/f11fc28734a7bb402fe5485aca3e1b0aab13cfc2))
 * **[#35](https://github.com/fishfolk/bones/issues/35)**
    - implement tilemap rendering. ([`0a7fec6`](https://github.com/fishfolk/bones/commit/0a7fec655cd951f18bb7e8e134a534d3e79999c1))
 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#43](https://github.com/fishfolk/bones/issues/43)**
    - add resource for controlling the clear color. ([`34c5ecc`](https://github.com/fishfolk/bones/commit/34c5ecc7b2f37b99fa3b415558a858ec26ec1bba))
 * **[#44](https://github.com/fishfolk/bones/issues/44)**
    - add utility `Key` datatype. ([`6d813cc`](https://github.com/fishfolk/bones/commit/6d813ccca3ea98f61fed0bdeaa2f15997c071b12))
 * **[#54](https://github.com/fishfolk/bones/issues/54)**
    - implement `Default` for sprite components. ([`e76de9d`](https://github.com/fishfolk/bones/commit/e76de9db7fa7116b9e1237c301e939f22de5e370))
 * **[#63](https://github.com/fishfolk/bones/issues/63)**
    - prepare for release. ([`ae0a761`](https://github.com/fishfolk/bones/commit/ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31))
 * **[#65](https://github.com/fishfolk/bones/issues/65)**
    - add missing crate descriptions. ([`2725246`](https://github.com/fishfolk/bones/commit/27252465ad0506ff2f8c377531fa079ec64d1750))
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
 * **Uncategorized**
    - add modules to the prelude. ([`a16443a`](https://github.com/fishfolk/bones/commit/a16443a0860e46bf44fed534648af85d540f975a))
    - add audio module. ([`c61b845`](https://github.com/fishfolk/bones/commit/c61b84553b8e4322a96de377b1b8f132894166db))
    - add a helper method for creating an `AtlasSprite`. ([`2a52b68`](https://github.com/fishfolk/bones/commit/2a52b688bb9b8920c9b0001fe536c4f82c86b127))
    - fix panics in `TileLayer` by returning an `Option<Tile>` from `get()`. ([`6419a8c`](https://github.com/fishfolk/bones/commit/6419a8cc1652c10d94192816cbd2f5199624faa5))
    - make `Key::new()` const and add `key!` macro for const construction. ([`2c7d5f4`](https://github.com/fishfolk/bones/commit/2c7d5f4372291a9c6e81bdc19a511e4fb0a45e8c))
</details>

