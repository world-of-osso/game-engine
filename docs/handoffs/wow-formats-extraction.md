# wow-formats extraction — phase 1

Verified: 2026-10-18. Public `world-of-osso/wow-formats` master: `7dd561c88f92945befb12b2f1203fadf95ca7ae7`. Engine implementation: `76d301bec`, rebased onto `7b7736c49`. Phase 2 is not authorized or started.

## Inventory and behavior boundary

See [shared reader ownership/features](../wiki/systems/wow-formats.md). Original parser implementations are deleted from core, not copied or wrapped. Old namespaces use direct public re-exports. Client model assembly, engine-axis adapters, material/simulation policies and data-backed fixtures remain in game-engine. Equipment imports shared bone names instead of including a removed source file. Both root and Godot lockfiles pin the same public revision.

No decompiled or leaked source inspected. No files under `data/` changed. No game-server files touched.

## Proof ledger

| Scope/revision | Command | Result |
| --- | --- | --- |
| Engine baseline `55e1f8687` | helper `--test -p game-engine-core --no-fail-fast` | 1063 passed / 1 failed / 4 ignored |
| wow-formats `7dd561c` | `cargo test --workspace` | 225 passed / 0 failed / 0 ignored |
| wow-formats `7dd561c` | `cargo test --workspace --no-default-features` | 149 passed / 0 failed / 0 ignored |
| wow-formats `7dd561c` | `cargo check --workspace`; individual `wow-m2 --no-default-features --features <feature>` checks for animation/cameras/events/lights/particles/ribbons/textures; `cargo fmt --check` | All pass, no warnings |
| Extracted engine `1b1aca78c` (same core/dependency inputs after rebase to `76d301bec`) | helper `--test -p game-engine-core --no-fail-fast` | 838 passed / same 1 failed / 4 ignored |
| Engine `76d301bec` | helper `--test -p game-engine-godot --no-fail-fast -- m2 wmo adt` | 54 passed / 0 failed / 0 ignored |
| Engine `76d301bec` | helper `--test -p game-engine-godot --no-fail-fast -- assets:: animation:: terrain:: ground:: --skip m2 --skip wmo --skip adt` | 188 passed / 0 failed / 1 pre-existing ignored |
| Engine `76d301bec` | `cd godot && cargo fmt --check` | Pass |

All builds/tests run through `agent-run wow_formats`, with `CAPACITY_OWNER=wow_formats` and capacity admission; engine tests use `DEPOT_SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol python3 scripts/depot-build.py --root /home/osso/.worktrees/game-engine-s13 ...`. Standalone Cargo is permitted in the new repository.

**Test conservation:** the 259 original format-module tests split into225 synthetic standalone tests +34 retained authored-asset fixtures. Combined after totals equal baseline1063/1/4. Comparing the full1068-name multiset (including failed/ignored), independent of changed module prefixes: zero lost and zero added tests. The242 passing Godot consumer/adapter tests are disjoint filter sets; one additional ignored test remains ignored.

**Unchanged baseline failure:** `blp::tests::guild_tabard_palette_keeps_authored_red_and_separate_alpha` cannot find `data/textures/180071.blp`. Lead accepted this missing-data issue before extraction; no test waived or altered.

Durable full output and name evidence: `/home/osso/.worktrees/logs/wow_formats-baseline-all-core-full.log`, `wow_formats-workspace-test-2.log`, `wow_formats-workspace-no-default.log`, `wow_formats-engine-core-after-1-full.log`, `wow_formats-engine-godot-consumers-full.log`, `wow_formats-engine-godot-adapters-full.log`.

## Lead gate

Use the two exact Godot filter commands above and core `--no-fail-fast` when reproducing the complete conserved proof. Standalone tests belong to the pinned repository, not the engine package. Do not enable a sibling path patch for wow-formats: this phase proves consumption of the published git revision. Phase 2 needs separate approval and a byte-identical reference LOS/nav bake before replacing server readers.
