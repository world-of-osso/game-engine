# Native compile-speed investigation

Verified: 2026-10-10. Branch `compile-speed`, base `ccd1073fa`; native timings at
`c7fdd1bda`, target-dir experiment at `1de220c0f`, final generator recipe at
`9506a0d0e` plus root formatter-lock correction `f15c66a3d`. The requested **few
seconds in every slot** is not achieved. KTX's per-output-directory rebuild is
removed, but empty-slot Rust compilation remains minutes and warm extension
improvement was not demonstrated.

## Root causes and implementation

Previously `ktx2-rw` downloaded/extracted KTX 4.4.0, CMake-built it and generated
bindings inside each Cargo `OUT_DIR`. Relocating targets, new slots and host
profile hashes repeated that work. The earlier `build-timings` evidence measured
488.39 seconds in one KTX build-script unit alone.

The fork now keeps a SHA-256-verified source archive, libktx and bindings under
`${XDG_CACHE_HOME:-~/.cache}/game-engine/ktx` (explicit `KTX_CACHE_DIR` override).
The 212 MB archive is too large to vendor. A separately locked source entry
fetches it once; `KTX_SOFTWARE_ARCHIVE` supplies an explicit offline archive,
not a fallback. Binary entries hash version/target, recipe including CMake
options, compiler identity/arguments, libclang and selected toolchain environment.
File locks cover population; a completion marker follows required-artifact
validation. Incomplete entries rebuild after failure; corrupt completed entries
fail loudly. Every new `OUT_DIR` copies only the cached bindings and links the
shared static library. Bookworm uses a separate BuildKit cache and explicit
archive at `/opt/ktx-software-v4.4.0.tar.gz`; release linker settings are unchanged.

The duplicate native graphs had identical KTX/bindgen/clang-sys features.
`glob` differed in host profile: `16914814784498550834` versus
`17712669511201206781`. It is both a CLI normal dependency and a clang-sys build
dependency. Explicit `profile.dev.package.glob.debug = 1` unifies the graph;
a subsequent verbose workspace invocation reported **Fresh glob, clang-sys,
bindgen and ktx2-rw** after an extension build.

Contrary to the initial brief, tracked root `.cargo/config.toml` already used
clang/mold and four linker threads. It also capped jobs at two. Native invocation
now supplies deterministic clang/mold defaults, runs from the actual checkout,
and sets jobs to its CPU affinity count (32 on this measured host). Explicit
Cargo `-j` and linker/Rust-flag environment overrides remain authoritative.
This is not evidence of a newly enabled or faster linker.

## Wall-time evidence

Seconds; one sample per loop/change, shared host, no variance or causal isolation
claim. All listed measured loop builds exited zero before subsequent sibling
protocol drift. Touch changes mtime only; it does **not** prove source-edit link
time. Workspace tests below are `--workspace --no-run`, not test execution.

| Loop | Before | After | Boundary |
| --- | ---: | ---: | --- |
| Empty new slot, extension | Not measured | 207.96 | After A–C; truly empty target, no sccache. Historical KTX-only 488.39 is not a whole-build baseline. |
| Warm touch `godot/rust`, extension | 5.70 | 14.16 | No warm extension improvement demonstrated. |
| Warm touch `godot/core`, extension | 6.84 | 8.65 | No warm extension improvement demonstrated. |
| Warm touch `godot/core`, workspace no-run | 77.84 | 44.49 | Original unconsolidated harness, helper defaults: 2 versus 32 jobs. |
| Actual extension export addition | 49.47 | 15.65 | Added then removed a temporary exported function; combined compilation/link loop, not isolated linker time. |
| Three concurrent fresh KTX output dirs, offline | Fails downloading | 0.139 | Final real build-script test, shared artifact/binding identity. |

`build-timings` commits `465497b1f`/`3a3bddf04` were neither copied nor redone.
Their separately measured consolidated workspace result, 14.21 seconds at 20
jobs, must not be substituted for this branch's unconsolidated/default-helper
measurement. Initial relocated warmups (319.54-second extension,
2479.21-second workspace) are retained but excluded from steady-state comparisons.
Later cache-recipe changes repopulated one entry (114.08 seconds); current
whole-client timing reruns are blocked, not silently attributed to old samples.

## Registry cache experiment: not enabled

Installed `sccache 0.18.0`, user-level `cargo install --locked
--no-default-features`, into `~/.cargo/bin/sccache`; justified by the 208-second
empty-slot dependency build. No system install, reboot or service change.
The experiment used a private Unix socket; its daemon is stopped.

Initially Rust hits were zero because sccache hashes `CARGO_*` environment,
including checkout-specific `CARGO_TARGET_DIR`. The helper now passes the same
output destination with Cargo's `--target-dir` instead, without that compiler
environment value. No engine/path-dependency Rust source reads that variable.
Output path, linker and build semantics are preserved; the process-boundary test
asserts the actual child arguments/environment.

After this correction, the next empty slot recorded **375 Rust hits / 93 Rust
misses**, but took **236.94 seconds**, versus **207.96 seconds** without the
wrapper. Cache fill was 278.14 seconds; uncorrected cached run 321.15 seconds.
Host contention limits comparability, but there is no demonstrated large wall-time
win. Therefore sccache is **not** a default helper dependency or recommended
rollout. It remains installed for evaluation; cache data stays user-local.

Two optional client-side-mode attempts hit `agents.slice`'s 4096-task boundary;
one left a waiting client, terminated by its verified owned PID. Normal daemon
mode subsequently completed both fill/hit experiments. Failed samples and
cleanup evidence are retained; they are not performance improvements.

## Proof ledger and current blockers

Final relevant positive proof: cache publication/recovery/isolation 3/3;
real offline output-dir reuse 1/1; Cargo environment 1/1; native source roots 3/3;
helper process fixtures 35/35; actual KTX pixel/metadata/rejection roundtrip 2/2.
Changed Rust formatting and launcher `cargo check --tests` pass. New Rust was
manually audited for readability; inherited platform branches/warnings were
not expanded or suppressed.

Current whole-client helper `cargo check` fails at unchanged
`godot/rust/src/account.rs:505` and `godot/rust/src/ipc/world.rs:229`: sibling
`shared-protocol` HEAD `75fe6a4` (October 10, 2026, 03:24 -0500) now requires
`SpellCastIntent.destination`. That API repair is not part of build-speed work.
Twenty inherited Godot macro-semicolon warnings remain visible. Package-wide
launcher format check fails only in untouched `launcher/tests/process.rs:657,673`;
that file is byte-identical to the base commit. Changed-file format check passes.

The **single** requested release attempt acquired build-lock at 03:48 and failed
before Docker compilation: existing UI preparation reported seven required
icons absent/invalid. No bypass, fallback or second release attempt was used.
Bookworm cache/link compatibility therefore remains unverified. Available Claude
delegation failed with expired OAuth, so no independent verifier acceptance is
claimed. These blockers and the unmet speed target prevent a ready/completed claim.

## Sources

- [Build invocation](../../../scripts/depot-build.py), [container build](../../../scripts/depot/Dockerfile).
- [Fork provenance](../../../vendor/README.md), [KTX build/cache](../../../vendor/ktx2-rw/build.rs).
- Evidence: `data/diagnostics/compile-speed-2026-10-10/` in the canonical checkout; full logs, wall JSONs, fingerprint inventories and cache statistics.
- Earlier evidence: `data/diagnostics/buildtime-2026-10-10/`, `/home/osso/.worktrees/handoff-build-timings.md`.

## See Also

- [Contract](../../specs/native-dev-builds.md)
- [Remote/native build usage](../../remote-builds.md)
- [[build-hosts]] — host admission and cache boundaries.
