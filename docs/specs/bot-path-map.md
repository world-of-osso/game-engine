# Stockade bot path trace

Manual Stockade (map 34) observation for the game-server `bot-map` SVG tool. Capture lives in the native bot harness; [schema and workflow](../wiki/systems/bot-path-map.md).

## What it must do

- [x] Default records nothing; explicit `BOT_PATH_MAP_TRACE=1` enables JSONL without changing harness decisions or fixtures.
- [x] Record admitted bot positions/facing at the existing client sample cadence and observed combat start/end/death events, retaining source clocks and unknowns.
- [x] Feed one manually invoked top-down SVG with baked walkability, source formation packs, observed end-state, paths/time ticks, combat markers and run/revision/duration legend.
- [x] Produce and inspect a real Stockade SVG/PNG.

## How it works

- [Capture/schema](../wiki/systems/bot-path-map.md).

## Implementation inventory

- `godot/tests/bot_path_capture.gd`: optional raw native sampling.
- `godot/tests/bot_controller_live.gd`: existing 100 ms client sampling seam.
- `scripts/bot_path_trace.py`: JSONL transformation.
- `scripts/run_stockade_bot.py`: explicit final trace publication and private-port admin selection.
- `scripts/run_bot_roster.py`: existing preparation with explicit private-port selection.

## Tests asserting this spec

- `scripts/tests/test_bot_path_trace.py`: concrete pose/facing/clocks, combat transitions, complete-roster wipe and exact known boss death.
- `scripts/tests/test_bot_admin_socket.py`: actual private admin/provision subprocess socket selection.
- `godot/tests/bot_path_capture_fixture.gd`, `bot_party_capture_fixture.gd`: actual opt-in/no-file seam and five admitted transform identities.
- Real capture/render acceptance: [wiki receipts](../wiki/systems/bot-path-map.md#real-stockade-proof--2026-10-19).

## Known gaps (current cycle)

- [ ] No existing trash source-GUID↔entity census: trash end-state and overlap-pack pulls remain explicitly UNKNOWN, not inferred from target swaps.
- [ ] Lead retained planner/check gate remains pending; captured runs are failures, not Stockade clear evidence.

## Out of scope

Hazard contact/navigation worker timing; server tick instrumentation; automatic rendering; other maps; inferred simulation timestamps or fabricated spawn identity.
