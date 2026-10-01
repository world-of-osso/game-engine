# Local animation before world readiness

The first native cursor GREEN attempt failed during startup, before cursor readiness. Concurrent world-entry change `5e26bc9b` made pending asynchronous unit visuals valid during Loading; `f7b137fb` gates local animation on the existing `SessionScreen::InWorld` barrier. `76487261` fixes the gate's missing import; post-fix runtime reaches READY and final cursor flow exits0. Independent cursor acceptance remains pending.

## Observed failure and source diagnosis

`/tmp/claude/native-bags-cursor-first-green.log` records `FIXTURE BAGS_CURSOR_LOADING`, then `ERROR: Player animation: Local player Input Fixture has no authored visual`, and parent `EXIT_CODE=101`. It never reaches `BAGS_CURSOR_READY` or the first slot interaction. This is a startup failure, not evidence that cursor `2b3fa596` passed or failed its UI behavior.

The source ordering is Account → attach arrived unit visuals → local player animation → Loading readiness. After `5e26bc9b`, local facing can exist while the authored visual request remains pending; readiness waits for `local_visual_settled`. Animation lacked the InWorld gate already used by input and footsteps, so it could consume that valid Loading state before readiness.

The log does not embed request/attachment timing or state booleans. Source establishes the possible failure path; the captured run does not prove a specific request interleaving or exact pending flags.

## Fix boundary

`f7b137fb` returns from `update_player_animation` unless the session screen is `SessionScreen::InWorld`. The existing missing-authored-visual error remains after readiness. No nil guard, error suppression, server change or fixture change is part of this fix.

## Proof coverage

| Capability | Evidence / limit |
| --- | --- |
| Cursor policy | Depot `nq85qrfm7f`: original pure cursor tests 8/8, warning-free; does not prove UI/wire integration. |
| Cursor icon | Depot `508hngz2pf`: icon test 1/1, warning-free; does not prove actual rendered cursor behavior. |
| Pre-gate native build | Depot `0wg2pzt429` exits 0; does not cover later `f7b137fb`. |
| First actual cursor GREEN attempt | Parent exit 101 before readiness; startup error above blocks cursor assertions. |
| InWorld animation gate | `f7b137fb`/`76487261`; Depot `6tpzkhk2xx` build exits0 and second GREEN reaches READY before cursor-image failure. No exact pending-request timing claim. |
| Final cursor flow | Depot `7p31d9cmcw` at `40497cb7` native+fixture exit0, existing WMO warning only; third GREEN parent0 completes UI/network flow. Independent agent1360 pending; [exact cursor evidence](../systems/godot-conversion.md#native-standalone-bags--bounded-window-pass-cursor-proof-pending). |

Earlier standalone-window PASS and legacy checked tests remain valid within their recorded scope. Full conversion stays open. Owned fixture child `3783421` was intentionally SIGKILLed/reaped with reader errors 0; this is not normal-shutdown proof. Shutdown remains deferred.

## Cursor-image producer mismatch after READY

Diagnostic `9dc97a47` records event point1758,838, OS viewport point0, rendered center0 and texture present in `/tmp/claude/native-bags-cursor-pointer-probe.log`. This is actual producer DataMismatch, not an expected-position calibration. `40497cb7` uses existing `PhysicalInput.pointer` actual event data, preserves pointer across keyboard-only split-modal input and captures position even while handled/UI-blocked. Final `/tmp/claude/native-bags-cursor-third-green.log` parent0 completes authored UI and decoded network flow; child3813397 intentional SIGKILL/reap/readers0 is not normal shutdown. Full conversion and independent cursor gate remain open.

## Sources

- `/tmp/claude/native-bags-proof-ledger.md` — saved build/test/runtime proof and diagnosis.
- `/tmp/claude/native-bags-cursor-first-green.log` — actual startup error, readiness marker boundary, exit and cleanup.
- `godot/rust/src/lib.rs` (`run_frame`), `godot/rust/src/gameplay.rs` (`update_player_animation`); commits `5e26bc9b`, `f7b137fb` — ordering, async transition and gate.

## See Also

- [Godot conversion](../systems/godot-conversion.md#native-standalone-bags--bounded-window-pass-cursor-proof-pending) — cursor/window acceptance limits.
- [[world-entry-stalls]] — asynchronous visual-loading change.
- [Conversion contract](../../specs/godot-conversion.md#standalone-bags-bounded-window-proof-cursor-pending) — full goal remains open.
