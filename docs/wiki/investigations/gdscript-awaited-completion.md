# Awaited GDScript completion

A nested coroutine assertion can abort that coroutine without terminating SceneTree. Its caller may resume, print PASS, and quit zero. Exit status alone therefore cannot establish native fixture completion.

## Mechanism and repair

The retained Random Bonus counterexample emitted SCRIPT ERROR and recorded4 of32 actions while exiting zero. The shared `godot/tests/completion.gd` replaces assertion-only flow with explicit checked results, completed-case accounting, a watchdog, and a sticky failure that exits nonzero. Each migrated top-level coroutine passes its actual return result to `finish`; typed awaited results are checked before use. Existing per-fixture behavior predicates and authority boundaries remain intact.

`godot/network/src/fixture_completion.rs` validates full process streams. `native_input_fixture/{menu,sound}.rs` feed every collected output line through that boundary, including pre-selection startup output. Their production Exit actions remain necessary; the completion marker does not itself quit those clients. The standalone `gdscript_completion_fixture` retains stdout/stderr before validating either. No runtime/import ERROR allowlist exists.

## Proof boundary

A process-level regression runs the original nested-assert/continued-caller failure with a forged final marker, then checked-failure and successful cases. Separate output tests cover missing/duplicate/short markers, nonzero exit, and errors on both streams. Parser-only runs do not prove behavior. Historical proof requalification lives in canonical `data/diagnostics/gdscript-completion-contract-2026-10-18/requalification.md`; missing streams remain UNKNOWN, and old PASS-only counts remain unqualified.

## Sources

- [Completion requirements](../../specs/gdscript-completion.md) — acceptance and migration scope.
- `godot/tests/completion.gd` — explicit results and case completion.
- `godot/network/src/fixture_completion.rs` — runner and real-process regression.

## See Also

- [[godot-conversion]] — native client fixtures.
