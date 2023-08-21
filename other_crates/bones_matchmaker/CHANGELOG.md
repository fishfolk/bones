# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 (2023-06-01)

### New Features

 - <csr-id-3f2e3485f9556cc68eb4c04df34d3aa2c6087330/> upgrade to Bevy 0.10.

### Style

 - <csr-id-92d0a58c1cb41485a023f396aa9e1a88544d69b3/> remove unnecessary set of `BLOCKING_MAX_THREADS` ENV var.
   This used to be used, but isn't applicable anymore.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 43 calendar days.
 - 134 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#117](https://github.com/fishfolk/bones/issues/117), [#122](https://github.com/fishfolk/bones/issues/122)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#117](https://github.com/fishfolk/bones/issues/117)**
    - remove unnecessary set of `BLOCKING_MAX_THREADS` ENV var. ([`92d0a58`](https://github.com/fishfolk/bones/commit/92d0a58c1cb41485a023f396aa9e1a88544d69b3))
 * **[#122](https://github.com/fishfolk/bones/issues/122)**
    - upgrade to Bevy 0.10. ([`3f2e348`](https://github.com/fishfolk/bones/commit/3f2e3485f9556cc68eb4c04df34d3aa2c6087330))
</details>

## 0.1.0 (2023-01-18)

<csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/>
<csr-id-49852b7f9d448334dfb66f4ab7c0310ec339f908/>
<csr-id-a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60/>
<csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/>
<csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/>

### Chore

 - <csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/> add missing crate descriptions.
 - <csr-id-49852b7f9d448334dfb66f4ab7c0310ec339f908/> update dependencies

### Chore

 - <csr-id-a68cb79e6b7d3774c53c0236edf3a12175f297b5/> generate changelogs for all crates.

### Documentation

 - <csr-id-a69389412d22b8cb48bab0ed96d739b0fee35348/> document source repository in cargo manifest.
   The `repository` key under `bones_ecs` previously pointed to https://github.com/fishfolk/jumpy.
   
   This updates this to point to the bones repo, and also adds the `repository` key to the other
   crates in the repository.

### New Features

 - <csr-id-3724c69a0bb24828d1710380bb8d139e304b7955/> migrate crates from the jumpy repository

### Other

 - <csr-id-a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60/> add github workflows for ci, docs, matchmaker, and PR linter

### Refactor (BREAKING)

 - <csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/> prepare for release.
   - Remove `bones_has_load_progress`: for now we don't use it, and if we
     want something similar we will work it into `bones_bevy_asset`.
   - Remove `bones_camera_shake`: it was moved into `bones_lib::camera`.
   - Add version numbers for all local dependencies.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 26 calendar days.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#37](https://github.com/fishfolk/bones/issues/37), [#63](https://github.com/fishfolk/bones/issues/63), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67), [#7](https://github.com/fishfolk/bones/issues/7)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#63](https://github.com/fishfolk/bones/issues/63)**
    - prepare for release. ([`ae0a761`](https://github.com/fishfolk/bones/commit/ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31))
 * **[#65](https://github.com/fishfolk/bones/issues/65)**
    - add missing crate descriptions. ([`2725246`](https://github.com/fishfolk/bones/commit/27252465ad0506ff2f8c377531fa079ec64d1750))
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
 * **[#7](https://github.com/fishfolk/bones/issues/7)**
    - update dependencies ([`49852b7`](https://github.com/fishfolk/bones/commit/49852b7f9d448334dfb66f4ab7c0310ec339f908))
 * **Uncategorized**
    - Release bones_matchmaker_proto v0.1.0, quinn_runtime_bevy v0.1.0, bones_matchmaker v0.1.0 ([`c6d682f`](https://github.com/fishfolk/bones/commit/c6d682fa4f428f9cb9c963c93061bd477f1d281e))
    - add github workflows for ci, docs, matchmaker, and PR linter ([`a516a68`](https://github.com/fishfolk/bones/commit/a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60))
    - migrate crates from the jumpy repository ([`3724c69`](https://github.com/fishfolk/bones/commit/3724c69a0bb24828d1710380bb8d139e304b7955))
</details>

