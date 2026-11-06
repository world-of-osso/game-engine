# Patched crates

`ktx2-rw` and `taffy` are copied from `world-of-osso/bevy-patches` at fork commit
`ad6dd31447a48082bcdcc783f4a4951c9d47e4ba`; `godot-ffi` is the crates.io 0.5.5 package.
No sibling checkout is required. Only these three are patched; Bevy dependencies remain
registry crates.
Sources, examples, tests, authorship and license declarations are retained.
Neither imported package contained separate LICENSE files.

| Crate | Upstream version / commit | Last fork change |
| --- | --- | --- |
| ktx2-rw | 0.2.4 / `1dfa70d893ab7a393b5c3754a5e15c3a6d182453` | `c7a98962d0cadc518313c073aeb5d42e7cb00265` |
| taffy | 0.10.1 / `639f0acdd20f914f847abffbeaa903d66cae4882` | `20ed24db468de3a960cb81eb628952999f8164cf` |

## ktx2-rw (MIT OR Apache-2.0)

The fork adds MinGW GCC runtime and target sysroot library search directories
beneath `MINGW_PREFIX` and pins bindgen 0.73.2 to avoid deprecated generated code
and keep the cached generator version identical in the root/Godot workspaces.
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
is a separate verification boundary. CI temporarily pins Ubuntu 24.04 because
KTX 4.4.0's SSE encoder fails to compile on the Ubuntu 26.04 runner. Retire the
runner pin when the KTX build passes on Ubuntu 26.04 with SSE enabled.

## taffy (MIT)

The fork backports upstream's unreleased cumulative-coordinate location rounding:
`round(cumulative) - round(parent_cumulative)`. This keeps abutting siblings aligned
under fractional parent coordinates. `tests/rounding_gaps.rs` covers shared edges.
Retire the patch when the selected upstream release contains the fix.

Standalone crate tests can run with `cargo test --manifest-path
vendor/taffy/Cargo.toml --test rounding_gaps` (through `agent-run` for agents).
Godot tests use `scripts/engine-build.py`; its source snapshot includes `vendor/`,
including taffy's compile-time README input.

## godot-ffi (MPL-2.0)

The crates.io `godot-ffi` 0.5.5 package (gdext commit
`e7b3a4a7dc9678a5502dc8432c258c702ff89429`), with its normalized `Cargo.toml`, `build.rs`
and `src/`. One change, in `src/lib.rs` `is_main_thread`: the calling thread's id comes
from a thread-local instead of `std::thread::current().id()`. With the default balanced
safeguards every engine call checks the main thread; `std::thread::current()` clones and
drops the thread handle (two atomic operations) each time: 4.4% of the main thread's CPU
in Stormwind in a release profile (`docs/wiki/investigations/godot-stormwind-fps.md`). The comparison with
`main_thread_id()` is unchanged, so `discover_main_thread` still moves the main thread.

Retire the patch when upstream godot-rust releases an equivalent fix.

### Upstream patch note

Title: `is_main_thread`: cache the calling thread's id in a thread-local.

```diff
-        std::thread::current().id() == main_thread_id()
+        thread_local! {
+            static CURRENT_THREAD_ID: std::thread::ThreadId = std::thread::current().id();
+        }
+        CURRENT_THREAD_ID.with(|id| *id == main_thread_id())
```

With balanced safeguards every engine call checks the main thread, and
`std::thread::current()` clones and drops the `Thread` handle (an atomic increment and
decrement) each time. A thread's id never changes, so a lazily initialized thread-local
gives the same answer; `main_thread_id()` is still read on every call, so
`discover_main_thread` still moves the main thread.

Benchmark (Godot 4.7.2 release template, Linux, Stormwind indoor pose, 1920x1080):
- microbenchmark: 15.1 → 4.8 ns per `is_main_thread` call;
- frame-pointer profile: `std::thread::current` was 4.4% (release) to 9% (debug) of
  main-thread samples;
- interleaved A/B, 4 pairs per build (`data/diagnostics/fps-measure-2026-11-06/close-2`):
  main-thread CPU 15.73 → 15.05 ms per frame (−0.68 ms), client process 10.54 → 9.78 ms,
  p99 frame 47.2 → 38.5 ms.
