# Native dev rebuild timings

Verified 2026-10-10 on agent-server, Rust/Cargo 1.99.0. Consolidating UI integration tests removes a measured workspace-test scheduling tail. The reported five-minute steady-state rebuild was not reproduced; cache relocation and first-time profile/feature compilation must be separated from warm touch rebuilds.

## Measurement

Checkout: `/home/osso/.worktrees/game-engine-buildtime`, based on `72853d204`. Target seeded from `game-engine-verify/target`. The seed script accepts a repository/cache label, not an explicit source checkout, and rejected the existing destination. Reflink copying returned `Operation not supported`; an ordinary independent copy preserved the requested warm source. No shared target writes, installs, release lock, reboot, or service restart.

Each timed run touches `godot/core/src/lib.rs` without changing its contents. Tests use:

```text
PATH=/home/osso/.local/bin:/home/osso/.cargo/bin:$PATH CARGO_TARGET_DIR=<slot>/target CARGO_BUILD_JOBS=20 scripts/agent/agent-run buildtime cargo test --locked --manifest-path godot/Cargo.toml --workspace --no-run --timings
```

Extension runs use `scripts/agent/agent-run buildtime python3 scripts/depot-build.py --root <slot>`; the helper removes inherited job overrides. Both commands execute from the checkout. Full output was captured to files, not rerun to recover logs. Wall times include command startup/helper overhead. One warm measurement per loop before and after the change. Baseline and final test-count runs warm the corresponding test harnesses before timing. Initial relocated-cache runs are separately retained, not credited as improvements.

| Loop | Relocated cache / first profile (excluded) | Warm before | Warm after |
|---|---:|---:|---:|
| Workspace test compilation, 20 jobs | 560.92s | 37.75s | 14.21s |
| Native extension helper | 285.59s | 8.73s | 7.55s |

Workspace compilation fell 62.4% (2.66× faster). Extension timing differs by 1.18s, but its dependency graph is unchanged: no causal extension speedup claimed from one pair on a shared host.

## Critical path

Initial test timing: `ktx2-rw` build script occupies 488.39s, followed by the Godot library/test builds (51.26s/69.18s). This is a relocated-cache rebuild, not evidence that touching core intrinsically rebuilds KTX. KTX is absent from the subsequent warm timing.

Warm test timing before consolidation:

| Unit | Start | Duration | Frontend / codegen |
|---|---:|---:|---|
| Core library | 0.78s | 2.25s | 1.68s / 0.57s |
| UI-model library | 2.46s | 6.83s | 6.22s / 0.61s |
| Godot library | 9.29s | 24.99s | Not exposed |
| Godot unit-test binary | 23.62s | 13.97s | Not exposed |

The 93 UI integration targets create a fan-out after UI-model metadata/codegen. Cargo records up to 94 waiting units; representative UI targets occupy 4–5.49s each while sampled rustc CPU time is only about 0.2–0.3s. The Godot unit-test build starts at 23.62s, well after its dependencies are available. Only two Cargo units remain at 31.59–34.28s and one at 34.93–37.59s. These are low **Cargo-unit concurrency** stretches, not measured whole-host CPU-idle percentages.

After consolidation, one UI integration target occupies 3.01s; Godot library/test both start at 6.86s and finish at 13.00s/14.13s. The final Godot test is now the longest tail. Higher job count was not tested again.

### Profiling boundary

Cargo HTML exposes frontend/codegen partitions for ordinary libraries, but reports null sections for test executables and Godot's combined cdylib/rlib target. The CSVs preserve every target's total duration and mark unavailable partitions explicitly. These totals are **not link-only times**. Process sampling observed rustc but no separate linker process in the warm touch loops; per-binary linker wall times remain unproven. Touching unchanged source can reuse incremental codegen/link results. No claim about true source-edit codegen or linker speed follows from this experiment.

`ld.lld`, `mold`, `clang`, `cc`, and `ld` already exist. No package was installed. No linker/profile/crate split was justified by these warm data; `debug = 1`, `split-debuginfo = "unpacked"`, optimization and incremental defaults remain unchanged.

## Change and test preservation

`godot/ui-model/Cargo.toml` explicitly selects `tests/integration.rs` and disables automatic per-file test targets. The harness declares all 93 existing suites as modules; original files, fixtures and assertions remain untouched. Add future suites to this harness. Named selection now uses `-p game-engine-ui-model --test integration <suite>::` rather than `--test <suite>`.

Before and after `cargo test --locked --manifest-path godot/Cargo.toml --workspace --no-fail-fast`: exit 0, **2681 passed, 0 failed, 8 ignored** (2689 total including doctests). Harness result summaries decline from 175 to 83, exactly 92 fewer binaries. Test-name/outcome multisets match after removing only the 523 newly introduced suite prefixes; no missing or extra cases. Existing Godot macro-semicolon warnings remain visible and unchanged; none suppressed.

Changed harness rustfmt check and UI-model `cargo check --tests` pass. The native helper also resolves the consolidated workspace test target successfully. Readability audit: declarations only, no new function, suppression, mutable state, or conditional control flow.

## Sources

- `data/diagnostics/buildtime-2026-10-10/` (untracked shared diagnostic data): before/after logs and wall JSON, Cargo HTML and per-target CSV, process samples, concurrency summary, test-count logs and normalized inventory comparison.
- [UI-model manifest](../../../godot/ui-model/Cargo.toml) and [integration harness](../../../godot/ui-model/tests/integration.rs).
- [Native build helper](../../../scripts/depot-build.py).

## See Also

- [Remote/native builds](../../remote-builds.md) — helper and cache boundaries.
- [[build-hosts]] — build-host history.
