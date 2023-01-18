# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2023-01-18)

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

### New Features (BREAKING)

 - <csr-id-59f5e67d42de57a33dd302443a8a04427126a5be/> have `TypeUlid` trait require an associated constant instead of a function.
   This makes it possible to access the type's Ulid at compile time,
   possibly in const functions.
 - <csr-id-d74cec66c8e4db5f8d287f4e619d172d0f9c8b91/> use `TypeUlid`s instead of `TypeUuid`s.
   Creates a new type_ulid crate and uses it's `TypeUlid` trait instead of
   `TypeUuid` in bones_ecs.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 20 calendar days.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#13](https://github.com/fishfolk/bones/issues/13), [#28](https://github.com/fishfolk/bones/issues/28), [#37](https://github.com/fishfolk/bones/issues/37), [#65](https://github.com/fishfolk/bones/issues/65), [#67](https://github.com/fishfolk/bones/issues/67)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#13](https://github.com/fishfolk/bones/issues/13)**
    - use `TypeUlid`s instead of `TypeUuid`s. ([`d74cec6`](https://github.com/fishfolk/bones/commit/d74cec66c8e4db5f8d287f4e619d172d0f9c8b91))
 * **[#28](https://github.com/fishfolk/bones/issues/28)**
    - have `TypeUlid` trait require an associated constant instead of a function. ([`59f5e67`](https://github.com/fishfolk/bones/commit/59f5e67d42de57a33dd302443a8a04427126a5be))
 * **[#37](https://github.com/fishfolk/bones/issues/37)**
    - document source repository in cargo manifest. ([`a693894`](https://github.com/fishfolk/bones/commit/a69389412d22b8cb48bab0ed96d739b0fee35348))
 * **[#65](https://github.com/fishfolk/bones/issues/65)**
    - add missing crate descriptions. ([`2725246`](https://github.com/fishfolk/bones/commit/27252465ad0506ff2f8c377531fa079ec64d1750))
 * **[#67](https://github.com/fishfolk/bones/issues/67)**
    - generate changelogs for all crates. ([`a68cb79`](https://github.com/fishfolk/bones/commit/a68cb79e6b7d3774c53c0236edf3a12175f297b5))
</details>

