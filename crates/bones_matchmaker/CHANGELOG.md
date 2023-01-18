# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2023-01-18)

<csr-id-27252465ad0506ff2f8c377531fa079ec64d1750/>
<csr-id-49852b7f9d448334dfb66f4ab7c0310ec339f908/>
<csr-id-a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60/>
<csr-id-ae0a761fc9b82ba2fc639c2b6f7af09fb650cd31/>

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

 - 7 commits contributed to the release over the course of 26 calendar days.
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
    - add github workflows for ci, docs, matchmaker, and PR linter ([`a516a68`](https://github.com/fishfolk/bones/commit/a516a68902ebcd4c3e24b6a47b3ff79b92ff5f60))
    - migrate crates from the jumpy repository ([`3724c69`](https://github.com/fishfolk/bones/commit/3724c69a0bb24828d1710380bb8d139e304b7955))
</details>

