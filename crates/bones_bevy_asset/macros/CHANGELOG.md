# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 (2023-01-18)

<csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/>

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

 - <csr-id-7fd1c592c61e3032d803b8f70364b826b4a9ebaf/> add extra derive support & type implementations.
   - The derive macro for `BonesBevyAssetLoad` can now be used on enums.
- Added more implementations of `BonesBevyAssetLoad` for primitive types.

### New Features (BREAKING)

 - <csr-id-89b44d7b4f64ec266eb0ea674c220e07376a03b7/> add asset integration with bevy.
   This is a big overall change that adds ways to integrate Bones with bevy assets.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 14 calendar days.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#29](https://github.com/fishfolk/bones/issues/29), [#33](https://github.com/fishfolk/bones/issues/33), [#37](https://github.com/fishfolk/bones/issues/37), [#39](https://github.com/fishfolk/bones/issues/39), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67)

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
 * **[#65](https://github.com/fishfolk/bones/issues/65)**
    - add missing crate descriptions. ([`2725246`](https://github.com/fishfolk/bones/commit/27252465ad0506ff2f8c377531fa079ec64d1750))
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
</details>

<csr-unknown>
 add derive macro for BonesBevyAssetLoad.This makes it easier to nest asset structs that have handles that need loading.<csr-unknown/>

