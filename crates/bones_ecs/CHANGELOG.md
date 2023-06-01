# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### New Features

 - <csr-id-3f2e3485f9556cc68eb4c04df34d3aa2c6087330/> upgrade to Bevy 0.10.
 - <csr-id-db98f76c5871b5fb989c85a3d1375aca145c8941/> make `Entity::new()` public.
   This is important for use cases where you need to manually
   serialize/load an entity ID.
 - <csr-id-7c578b47f5046251528e996ff00f997637385761/> make `insert_stage_before/after()` chainable.
 - <csr-id-147ebc86744a90196dbbbde1ad0563117b3c0414/> add `get_many_mut()` method to `CompMut`.
   This allows you to mutably borrow the component for multiple entities
   at the same time, which was otherwise difficult, unsafe, or inefficient
   to do previously.

### Bug Fixes

 - <csr-id-3a3f05ac6b1418784a404f5070e6346122600ee1/> change type bound for `Res` from `Default` to `FromWorld`.
 - <csr-id-1335457adaf6300d166f24a175378993e9bacb75/> export `FromWorld` publicly and make compatible with `Res`/`ResMut` system parameters.
 - <csr-id-3f061167a3f8e13a2cda7e81703d4abe42587aa6/> fix `insert_stage_before/after` always inserting before/after `PreUpdate`.
 - <csr-id-7bfcf5ddb1ed2f42f6a34bfbbde96f0240ce7fb3/> fix returned component order in `get_many_mut()`.
   `get_many_mut()` was previously not returning the components
   in the same order as the entities list.

### New Features (BREAKING)

 - <csr-id-00110c27b0aa76ed597c7e4d62bec70cfd1b2a23/> add `from_world` implementation similar to Bevy.
   Allows resources to be added with either a `Default` implementation,
   or a custom `FromWorld` implementation that allows them to derive their,
   value from any other data currently in the world.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 126 calendar days.
 - 133 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 9 unique issues were worked on: [#115](https://github.com/fishfolk/bones/issues/115), [#122](https://github.com/fishfolk/bones/issues/122), [#78](https://github.com/fishfolk/bones/issues/78), [#79](https://github.com/fishfolk/bones/issues/79), [#88](https://github.com/fishfolk/bones/issues/88), [#90](https://github.com/fishfolk/bones/issues/90), [#92](https://github.com/fishfolk/bones/issues/92), [#93](https://github.com/fishfolk/bones/issues/93), [#94](https://github.com/fishfolk/bones/issues/94)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#115](https://github.com/fishfolk/bones/issues/115)**
    - change type bound for `Res` from `Default` to `FromWorld`. ([`3a3f05a`](https://github.com/fishfolk/bones/commit/3a3f05ac6b1418784a404f5070e6346122600ee1))
 * **[#122](https://github.com/fishfolk/bones/issues/122)**
    - upgrade to Bevy 0.10. ([`3f2e348`](https://github.com/fishfolk/bones/commit/3f2e3485f9556cc68eb4c04df34d3aa2c6087330))
 * **[#78](https://github.com/fishfolk/bones/issues/78)**
    - add `get_many_mut()` method to `CompMut`. ([`147ebc8`](https://github.com/fishfolk/bones/commit/147ebc86744a90196dbbbde1ad0563117b3c0414))
 * **[#79](https://github.com/fishfolk/bones/issues/79)**
    - fix returned component order in `get_many_mut()`. ([`7bfcf5d`](https://github.com/fishfolk/bones/commit/7bfcf5ddb1ed2f42f6a34bfbbde96f0240ce7fb3))
 * **[#88](https://github.com/fishfolk/bones/issues/88)**
    - fix `insert_stage_before/after` always inserting before/after `PreUpdate`. ([`3f06116`](https://github.com/fishfolk/bones/commit/3f061167a3f8e13a2cda7e81703d4abe42587aa6))
 * **[#90](https://github.com/fishfolk/bones/issues/90)**
    - make `insert_stage_before/after()` chainable. ([`7c578b4`](https://github.com/fishfolk/bones/commit/7c578b47f5046251528e996ff00f997637385761))
 * **[#92](https://github.com/fishfolk/bones/issues/92)**
    - add `from_world` implementation similar to Bevy. ([`00110c2`](https://github.com/fishfolk/bones/commit/00110c27b0aa76ed597c7e4d62bec70cfd1b2a23))
 * **[#93](https://github.com/fishfolk/bones/issues/93)**
    - make `Entity::new()` public. ([`db98f76`](https://github.com/fishfolk/bones/commit/db98f76c5871b5fb989c85a3d1375aca145c8941))
 * **[#94](https://github.com/fishfolk/bones/issues/94)**
    - export `FromWorld` publicly and make compatible with `Res`/`ResMut` system parameters. ([`1335457`](https://github.com/fishfolk/bones/commit/1335457adaf6300d166f24a175378993e9bacb75))
</details>

## 0.1.0 (2023-01-18)

<csr-id-0b424b93d127618b7ecf6b831cc71d317e28af97/>
<csr-id-a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60/>
<csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/>
<csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/>

### Chore

 - <csr-id-0b424b93d127618b7ecf6b831cc71d317e28af97/> update bevy_derive dependency version
   Updates to 0.9.1 just to keep up with the latest Bevy version.

### Chore

 - <csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/> generate changelogs for all crates.

### Documentation

 - <csr-id-1891e2d17f0a1bd6876ffdcbe1b2d90b7fbd6571/> update docs and add tutorial.
 - <csr-id-a69389412d22b8cb48bab0ed96d739b0fee35348/> document source repository in cargo manifest.
   The `repository` key under `bones_ecs` previously pointed to https://github.com/fishfolk/jumpy.
   
   This updates this to point to the bones repo, and also adds the `repository` key to the other
   crates in the repository.
 - <csr-id-5f0ea6441b32575e613a2e3af2f2c46a4afec223/> fix doc links/errors for `ulid` module.

### New Features

 - <csr-id-2f5ff59d2ac0a924362846d1d78c827a98deacde/> add utility function for running already initialized systems.
 - <csr-id-a11fd1b610b79b5e9bc0d0d477bd56342da66d30/> add utility functions for adding stages to `SystemStages`.
 - <csr-id-7a9920687cb0e05a0e237ed882c3ab8ebe7624b8/> add debug implementation for `System`.
   Also simplifies the debug implementation for the `CoreStage` label.
 - <csr-id-3a3c6536b94a6fa8c1a0d5f53c436302092eb112/> add `default()` function.
   Makes a free-standing `default()` function equivalent to `Default::default()`
   and puts it in the ECS prelude.
 - <csr-id-1487dead42231e2ab870debb7db42375d21e6062/> add `contains()` helper method to component store.
 - <csr-id-3724c69a0bb24828d1710380bb8d139e304b7955/> migrate crates from the jumpy repository

### Bug Fixes

 - <csr-id-efe8744f44b5090a2eaebfc9e27cce56a84a73a6/> fix soundness issues revealed in code-review
   - Fixes safety documentation for some public, unsafe functions.

### Other

 - <csr-id-a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60/> add github workflows for ci, docs, matchmaker, and PR linter

### New Features (BREAKING)

 - <csr-id-81ca6548c96ad9b6bdd23c9ed45d7c887950b8b9/> implement `Commands` system parameter.
   The `Commands` parameter can be used to schedule systems that
   should run at the end of the current stage.
 - <csr-id-0a43a5a48fe0c26cb926555ef15384907871a9e1/> add return value parameter to systems.
   This allows systems to return values,
   which is most useful in the context of the world's
   `run_system()` function which can now extract the return value.
   
   The systems in `SystemStages` are still required to
   return `()` or `SystemResult<()>`.
 - <csr-id-dcd252a819a8f8bc8cdbc33278740dd76feb2ffa/> use a trait for SystemStages.
   This allows you to create custom stage implementations.
   
   The plan is to use this functionality in Jumpy to create a special,
   looping stage to use for the player state machine.
 - <csr-id-c29c96dff380f10438e955adf3a1919479294ef2/> add improved iteration API.
   Added a more convenient replacement for the old `join!` macro.
 - <csr-id-b80cf486bd66a160031072ba1a616bac0195052a/> remove join! macro and improve iteration API.
   We will add a more ergonomic replacement for the `join!` macro later,
   but for now we make it easier to use the raw bitset iteration APIs,
   which previously required a frustrating use of `Rc`.
   
   Removing the `Rc` broke the `join!` macro, but I think there is a better way to
   create a join API, so it isn't worth fixing yet.
   
   This also improves the ergonomics of the bitset iterators by having them
   yield their items directly instead of yielding `Option`s that must be filtered out.
 - <csr-id-59f5e67d42de57a33dd302443a8a04427126a5be/> have `TypeUlid` trait require an associated constant instead of a function.
   This makes it possible to access the type's Ulid at compile time,
   possibly in const functions.
 - <csr-id-0c259d455b1eaa6c612c893a4878903d0c6ce783/> replace `Dispatcher` with `SystemStages`.
   This replaces the `Dispatcher` with a similar `SystemStages` utility
   suitable for running multiple systems in a row.
   
   `SystemStages` improves on `Dispatcher` by making it easier to add
   systems to specific stages, similar to Bevy stages.
 - <csr-id-85212abbfda810cb093076b5701c37911365b5c3/> require systems to impl `Sync`.
   This makes things easier when integrating with Bevy right now.
   If this becomes too restrictive in the future we can re-visit.
 - <csr-id-60b850a07e32d1eaee8ea910300de11dc299bf02/> add `Default` bound to `Res` and `ResMut`.
   This makes the `Default` bound on `IntoSystem` easier to find
   by failing directly in the generic argument
   instead of failing at the `IntoSystem` implementation.
 - <csr-id-d74cec66c8e4db5f8d287f4e619d172d0f9c8b91/> use `TypeUlid`s instead of `TypeUuid`s.
   Creates a new type_ulid crate and uses it's `TypeUlid` trait instead of
   `TypeUuid` in bones_ecs.

### Bug Fixes (BREAKING)

 - <csr-id-5fc8009211db205e493cff92076e4e8089904d41/> fix unsound component iterators.
   The component iterators were casting pointers to byte slices,
   but they should have been returning raw pointers instead.
   
   This also simplified the mutable iterator implementation.
   
   This fixed strange behavior in a non-minimal test repository,
   where mutating the transform component of one entity somehow
   applied to the transform of another entity at the same time.

### Refactor (BREAKING)

 - <csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/> prepare for release.
   - Remove `bones_has_load_progress`: for now we don't use it, and if we
     want something similar we will work it into `bones_bevy_asset`.
   - Remove `bones_camera_shake`: it was moved into `bones_lib::camera`.
   - Add version numbers for all local dependencies.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 27 commits contributed to the release over the course of 26 calendar days.
 - 25 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 19 unique issues were worked on: [#13](https://github.com/fishfolk/bones/issues/13), [#17](https://github.com/fishfolk/bones/issues/17), [#19](https://github.com/fishfolk/bones/issues/19), [#20](https://github.com/fishfolk/bones/issues/20), [#21](https://github.com/fishfolk/bones/issues/21), [#23](https://github.com/fishfolk/bones/issues/23), [#24](https://github.com/fishfolk/bones/issues/24), [#28](https://github.com/fishfolk/bones/issues/28), [#30](https://github.com/fishfolk/bones/issues/30), [#32](https://github.com/fishfolk/bones/issues/32), [#36](https://github.com/fishfolk/bones/issues/36), [#37](https://github.com/fishfolk/bones/issues/37), [#42](https://github.com/fishfolk/bones/issues/42), [#5](https://github.com/fishfolk/bones/issues/5), [#57](https://github.com/fishfolk/bones/issues/57), [#59](https://github.com/fishfolk/bones/issues/59), [#6](https://github.com/fishfolk/bones/issues/6), [#63](https://github.com/fishfolk/bones/issues/63), [#67](https://github.com/fishfolk/bones/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#13](https://github.com/fishfolk/bones/issues/13)**
    - use `TypeUlid`s instead of `TypeUuid`s. ([`d74cec6`](https://github.com/fishfolk/bones/commit/d74cec66c8e4db5f8d287f4e619d172d0f9c8b91))
 * **[#17](https://github.com/fishfolk/bones/issues/17)**
    - add `contains()` helper method to component store. ([`1487dea`](https://github.com/fishfolk/bones/commit/1487dead42231e2ab870debb7db42375d21e6062))
 * **[#19](https://github.com/fishfolk/bones/issues/19)**
    - add `Default` bound to `Res` and `ResMut`. ([`60b850a`](https://github.com/fishfolk/bones/commit/60b850a07e32d1eaee8ea910300de11dc299bf02))
 * **[#20](https://github.com/fishfolk/bones/issues/20)**
    - require systems to impl `Sync`. ([`85212ab`](https://github.com/fishfolk/bones/commit/85212abbfda810cb093076b5701c37911365b5c3))
 * **[#21](https://github.com/fishfolk/bones/issues/21)**
    - fix doc links/errors for `ulid` module. ([`5f0ea64`](https://github.com/fishfolk/bones/commit/5f0ea6441b32575e613a2e3af2f2c46a4afec223))
 * **[#23](https://github.com/fishfolk/bones/issues/23)**
    - replace `Dispatcher` with `SystemStages`. ([`0c259d4`](https://github.com/fishfolk/bones/commit/0c259d455b1eaa6c612c893a4878903d0c6ce783))
 * **[#24](https://github.com/fishfolk/bones/issues/24)**
    - add `default()` function. ([`3a3c653`](https://github.com/fishfolk/bones/commit/3a3c6536b94a6fa8c1a0d5f53c436302092eb112))
 * **[#28](https://github.com/fishfolk/bones/issues/28)**
    - have `TypeUlid` trait require an associated constant instead of a function. ([`59f5e67`](https://github.com/fishfolk/bones/commit/59f5e67d42de57a33dd302443a8a04427126a5be))
 * **[#30](https://github.com/fishfolk/bones/issues/30)**
    - remove join! macro and improve iteration API. ([`b80cf48`](https://github.com/fishfolk/bones/commit/b80cf486bd66a160031072ba1a616bac0195052a))
 * **[#32](https://github.com/fishfolk/bones/issues/32)**
    - fix unsound component iterators. ([`5fc8009`](https://github.com/fishfolk/bones/commit/5fc8009211db205e493cff92076e4e8089904d41))
 * **[#36](https://github.com/fishfolk/bones/issues/36)**
    - add improved iteration API. ([`c29c96d`](https://github.com/fishfolk/bones/commit/c29c96dff380f10438e955adf3a1919479294ef2))
 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#42](https://github.com/fishfolk/bones/issues/42)**
    - use a trait for SystemStages. ([`dcd252a`](https://github.com/fishfolk/bones/commit/dcd252a819a8f8bc8cdbc33278740dd76feb2ffa))
 * **[#5](https://github.com/fishfolk/bones/issues/5)**
    - fix soundness issues revealed in code-review ([`efe8744`](https://github.com/fishfolk/bones/commit/efe8744f44b5090a2eaebfc9e27cce56a84a73a6))
 * **[#57](https://github.com/fishfolk/bones/issues/57)**
    - implement `Commands` system parameter. ([`81ca654`](https://github.com/fishfolk/bones/commit/81ca6548c96ad9b6bdd23c9ed45d7c887950b8b9))
 * **[#59](https://github.com/fishfolk/bones/issues/59)**
    - update docs and add tutorial. ([`1891e2d`](https://github.com/fishfolk/bones/commit/1891e2d17f0a1bd6876ffdcbe1b2d90b7fbd6571))
 * **[#6](https://github.com/fishfolk/bones/issues/6)**
    - update bevy_derive dependency version ([`0b424b9`](https://github.com/fishfolk/bones/commit/0b424b93d127618b7ecf6b831cc71d317e28af97))
 * **[#63](https://github.com/fishfolk/bones/issues/63)**
    - prepare for release. ([`ae0a761`](https://github.com/fishfolk/bones/commit/ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31))
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
 * **Uncategorized**
    - Release type_ulid v0.1.0, bones_bevy_utils v0.1.0, bones_ecs v0.1.0, bones_asset v0.1.0, bones_input v0.1.0, bones_render v0.1.0, bones_lib v0.1.0 ([`69713d7`](https://github.com/fishfolk/bones/commit/69713d7da8024ee4b3017b563f031880009c90ee))
    - Release type_ulid_macros v0.1.0, type_ulid v0.1.0, bones_bevy_utils v0.1.0, bones_ecs v0.1.0, bones_asset v0.1.0, bones_input v0.1.0, bones_render v0.1.0, bones_lib v0.1.0 ([`db0333d`](https://github.com/fishfolk/bones/commit/db0333ddacb6f29aed8664db67973e72ea586dce))
    - add utility function for running already initialized systems. ([`2f5ff59`](https://github.com/fishfolk/bones/commit/2f5ff59d2ac0a924362846d1d78c827a98deacde))
    - add utility functions for adding stages to `SystemStages`. ([`a11fd1b`](https://github.com/fishfolk/bones/commit/a11fd1b610b79b5e9bc0d0d477bd56342da66d30))
    - add debug implementation for `System`. ([`7a99206`](https://github.com/fishfolk/bones/commit/7a9920687cb0e05a0e237ed882c3ab8ebe7624b8))
    - add return value parameter to systems. ([`0a43a5a`](https://github.com/fishfolk/bones/commit/0a43a5a48fe0c26cb926555ef15384907871a9e1))
    - add github workflows for ci, docs, matchmaker, and PR linter ([`a516a68`](https://github.com/fishfolk/bones/commit/a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60))
    - migrate crates from the jumpy repository ([`3724c69`](https://github.com/fishfolk/bones/commit/3724c69a0bb24828d1710380bb8d139e304b7955))
</details>

