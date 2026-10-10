# Native development rebuilds

Native extension and CPU-test builds run through `scripts/depot-build.py`.
The requested outcome is a few seconds in every slot, including newly prepared
slots. [Measured implementation and current limits](../wiki/investigations/native-compile-speed.md)
own the evidence; that outcome is not yet achieved.

## What it must do

### Shared native dependencies

- [x] Concurrent fresh Cargo output directories reuse one complete libktx archive and identical bindings without network access after initial cache population.
- [x] Concurrent initial population publishes exactly once; different keys remain independent.
- [x] A failed population is not published and a later caller rebuilds it; missing artifacts in a completed entry fail loudly.
- [x] Reused libktx preserves concrete RGBA pixels, dimensions, format and metadata; malformed KTX input is rejected.
- [ ] A changed KTX version, target, CMake options or compiler identity must select a distinct compatible native artifact entry. Key isolation is tested; the full toolchain-input matrix is not.

### Build invocation

- [x] Native Cargo uses checkout-local output paths and clang/mold defaults independently of caller cwd; the native host CPU affinity supplies the default job count.
- [ ] Extension and workspace-test loops share the bindgen/KTX dependency graph without changing runtime behavior. Native verbose-build evidence exists; no automated graph-equivalence assertion.
- [ ] Release builds remain Bookworm-based and never consume native Arch cache artifacts; release linking is unchanged.
- [ ] Native dev rebuilds take a few seconds in every slot. Current timing evidence does not meet this requirement.

## How it works

- [Native compile-speed investigation](../wiki/investigations/native-compile-speed.md)
- [Build-host usage and cache boundaries](../remote-builds.md)
- [Fork provenance and retirement conditions](../../vendor/README.md)

## Implementation inventory

| Paths | Role |
| --- | --- |
| `vendor/ktx2-rw/build.rs`, `cache.rs` | Shared source/archive, native library and binding population, locks and compatible entry identity. |
| `vendor/ktx2-rw/Cargo.toml`, root/Godot `Cargo.lock` | Pinned binding generator and compatible formatter dependency. |
| `godot/Cargo.toml` | Common `glob` debug profile for normal/build dependency use. |
| `scripts/depot-build.py` | Native linker, job count, cwd and target-dir invocation. |
| `scripts/depot/Dockerfile` | Separate Bookworm cache and explicit checksum-pinned offline source archive. |

## Tests asserting this spec

- `launcher/tests/ktx_cache.rs` — publication, contention, failure recovery, corruption and key isolation.
- `scripts/tests/test_ktx_shared_cache.py` — real compiled build script, three concurrent fresh output directories offline.
- `scripts/tests/test_native_dev_environment.py` — observable Cargo child cwd, arguments and environment.
- `tools/tests/ktx_roundtrip.rs` — concrete KTX serialization and rejection.

## Known gaps (current cycle)

- [ ] Every-slot seconds target; registry caching did not demonstrate a large wall-time win.
- [ ] Release-container compatibility proof, blocked before compilation by existing UI icon preparation.
- [ ] Current client/protocol compatibility and inherited format failures; see the investigation for exact boundaries.

## Out of scope

System packages, reboot/services, runtime optimization reductions, test consolidation
owned by `build-timings`, unrelated protocol/API or asset repairs, merge and deployment.
