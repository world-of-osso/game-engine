# Login Bevy UI proof ledger

## Baseline (before migration)

- Scope: engine `4a6f62cd2920a8b10317e775513a06d0e839c31b`, dependency revisions in `baseline/report.md`.
- Command: `cargo test --features dev --bin game-engine --no-run --message-format=json`; shared target, isolated XDG config.
- Result: exit 0, 21m01s. Main inspected `baseline/compile-result.json` and report. No unresolved imports reported.
- Command: compiler-emitted executable `game_engine-84b4b3ba2e91c809 scenes::login:: --nocapture --test-threads=1` under remaining cumulative timeout.
- Result: exit 0, 34 selected cases; listing plus test 1.007s. Main inspected `baseline/tests.json` and report; full output/provenance retained in baseline directory.
- Limitation: missing worktree Blizzard logo diagnostics. No rendered or real authentication-server proof.
- Validity: baseline only; future migration code is not covered. Do not repeat baseline compilation or tests merely for a milestone.

## Native integration

- First integrated test compilation at `093d0a30`: exit 101; two TextFont type errors. Saved under `verification/`; corrected in `700f12cf`.
- Retry at `700f12cf`: verifier reports exit 0, zero compiler diagnostics; full artifacts under `verification/attempt-2/`.
- Compiler-emitted bin focused tests: 35 passed (11 form, 23 native, 1 helper), zero failed/ignored. Main inspected `verification/attempt-2/tests.json`: exit 0, listing plus execution 0.120161851 seconds. Exact argv/environment recorded there; executable identity recorded in compile artifacts.
- Scope excludes library-owned automation/IPC tests, actual layout/rendering and real server authentication. Library compilation currently underway. Later source changes invalidate intersecting proof only.

- Main inspected `verification/focused-report.md`: 57 distinct cases passed across revision-scoped bin35, library16 and presentation6; cumulative listing/test execution 0.527190 seconds. Full artifact matrix and retained failed attempts in report. No GUI/runtime proof.
- `8e11fd51` adds one real setup-system asset-failure test; unexecuted. Previous57 scopes remain valid (new test only).
- Readability findings inspected in `verification/readability/report.md`: long orchestration/loading functions, no reported complexity threshold breach. Deferred nonessential extraction; no reproduced behavior failure justifies redesign.

## Current proof (supersedes pending statuses above)

- 64 distinct focused tests recorded through successful setup/fade coverage at `1ad57bc6`; startup-order, setup failure/success, visible fade progression, presentation, input/auth resources, native/legacy diagnostics covered. See `verification/rendered-audit/report.md` and linked revision-scoped reports; do not rerun unaffected cases.
- Settled standalone screenshots at12/30seconds and menu overlay captured. Automated username/password masking supported; physical input not claimed. All diagnostic clients terminated.
- Live local auth at renderer `337e8a15`: native ConnectButton with dev-prefilled admin credentials (no token) authenticated, returned2characters, reached CharSelect, dumped destination UI without native LoginRoot. Independent audit `verification/live-auth/report.md`. AppExitSuccess requested but9s timeout interrupted shutdown (exit-9); do not claim clean exit. Initial typed-alice attempt timed out before submission, not an auth rejection. No production source change or unaffected test rerun.
- Dependency isolation correction: engine `d7dae698`, toolkit `40097e9`, docs `de81eef5`. Four registered dependency worktrees replace removed shared symlinks. Locked offline Linux metadata resolves all four and preserves target/lock/canonical manifests. Dev check passed121.22s. Global fmt failed104unchangedvendor files. See `dependency-worktrees/verification/report.md`.

## Caret implementation (pending integration)

- Commits `d0622b69` through `f70a6f87` add shaped-text cursor geometry and native caret presentation. `0fd27d15` updates the Parley/Bevy test API; `ed14b3d7` updates the shaped-font test fixture.
- Command: `cargo build --features dev --bin game-engine --message-format=json` at `ed14b3d78b17df6b58b447ed3dfb5b3c39924857`.
- Result: exit 0 in 32.35s, zero compiler diagnostics. Emitted executable `/syncthing/Sync/Projects/world-of-osso/game-engine/target/debug/game-engine`, SHA-256 `c497a86d6accbd8f9b5ab15320f1c1462b18151f31dae32127d5109008438955`. Artifacts: `caret/verification/build.json`, `build.stdout`, `build.stderr`.
- Limitation: this proves only the exact build revision. Focused behavior tests, rendered caret proof, physical input, and later source changes require parent-owned integration verification.

## Shared target ownership

Verifier961 released after dependency check; no local Cargo command active. Check other sessions before next Cargo window. Dependencies now resolve through real `*-bevy-ui-login` worktrees; never recreate shared sibling symlinks.

## Runtime audit correction

`src/ipc/plugin/scene.rs::queue_scene_screenshot` independently spawns `Screenshot::primary_window()` without a Login guard. The guard in `app_runtime::take_screenshot` does not block IPC. Earlier explorer claim that IPC needed a source change is rejected. Use only the exact launched PID socket; never select an arbitrary existing client.
