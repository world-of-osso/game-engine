# Patched crates

Copied from `world-of-osso/bevy-patches` at fork commit
`ad6dd31447a48082bcdcc783f4a4951c9d47e4ba`. No sibling checkout is required.
Only `ktx2-rw` and `taffy` are patched; Bevy dependencies remain registry crates.
Sources, examples, tests, authorship and license declarations are retained.
Neither imported package contained separate LICENSE files.

| Crate | Upstream version / commit | Last fork change |
| --- | --- | --- |
| ktx2-rw | 0.2.4 / `1dfa70d893ab7a393b5c3754a5e15c3a6d182453` | `c7a98962d0cadc518313c073aeb5d42e7cb00265` |
| taffy | 0.10.1 / `639f0acdd20f914f847abffbeaa903d66cae4882` | `20ed24db468de3a960cb81eb628952999f8164cf` |

## ktx2-rw (MIT OR Apache-2.0)

The fork adds MinGW GCC runtime and target sysroot library search directories
beneath `MINGW_PREFIX` and uses bindgen 0.73.1 to avoid deprecated generated code.
It also caches libktx and generated bindings outside Cargo's `OUT_DIR`, under
`${XDG_CACHE_HOME:-~/.cache}/game-engine/ktx` (`KTX_CACHE_DIR` overrides it).
Entries hash KTX version/target, this build recipe (CMake options), compiler identity
and arguments, libclang identity and toolchain/bindgen environment. A file lock
serializes each entry; completion is published only after both artifacts exist.
Failed entries are rebuilt on the next attempt; corrupt completed entries fail loudly.
The SHA-256-pinned 212 MB source archive is fetched once into a separately locked
source entry, not vendored. Warm/new-OUT_DIR builds use no network. An explicit
`KTX_SOFTWARE_ARCHIVE` supplies an offline archive; there is no `/tmp` fallback.
Bookworm uses its own BuildKit cache and explicit archive, not Arch native artifacts.
Build dependencies `cc` and `sha2` identify compiler inputs and hash inputs/archive;
both already exist in the workspace lockfiles.

Retire the patch when upstream includes the Windows/bindgen changes and equivalent
cross-OUT_DIR native caching, and native GNU Windows linking plus concurrent offline
cache reuse pass. Root tools tests exercise KTX conversion; native Windows linking
is a separate verification boundary.

## taffy (MIT)

The fork backports upstream's unreleased cumulative-coordinate location rounding:
`round(cumulative) - round(parent_cumulative)`. This keeps abutting siblings aligned
under fractional parent coordinates. `tests/rounding_gaps.rs` covers shared edges.
Retire the patch when the selected upstream release contains the fix.

Standalone crate tests can run with `cargo test --manifest-path
vendor/taffy/Cargo.toml --test rounding_gaps` (through `agent-run` for agents).
Godot tests use `scripts/depot-build.py`; its source snapshot includes `vendor/`,
including taffy's compile-time README input.
