# Stockade bot path map

Opt-in client observations for a manual Stockade SVG. Real bounded failed-run capture/render proof recorded below; no Stockade clear claim.

## Capture

Set `BOT_PATH_MAP_TRACE=1` on both the owned Godot bot-controller process and `scripts/run_stockade_bot.py`; all other runner arguments remain unchanged. Unset means no path files. Use a fresh existing `BOT_CONTROLLER_OUTPUT` directory. The client writes `bot-path-samples.jsonl`; runner finish writes `bot-path-trace.jsonl` beside its normal results. No input/target/cast behavior changes.

`stockade_path_map_v1` starts with `run_start` (run ID, engine revision, map 34, cadence, capture gaps), contains `pose`, `combat_start`, `combat_end`, `death` and `combat_event`, and ends with `run_end` (failure retained). Poses have string actor identity, admitted party name, `[x,y,z]` Bevy/world position and radians facing. Owner facing comes from account state; companion facing from the existing unit visual transform. Missing observations are not synthesized.

Every row has `at_ms` Unix milliseconds, `simulation_s` (null: not exposed by the harness), `monotonic_ns`, and `monotonic_clock`. Native samples use Godot process ticks in nanosecond units (`godot_process_ticks`); combat-log events have their original wall timestamp and null monotonic clock. Do not compare these with Python/OS monotonic values. Sampling uses the existing 100 ms client timer, **not server ticks**. Death delivery can be later than its retained event timestamp. Combat transitions describe observed owner CombatStatus, not target swaps or pack completion.

## Sources

- [Contract](../../specs/bot-path-map.md).
- `godot/tests/bot_controller_live.gd`, `godot/tests/bot_path_capture.gd` — existing native seam.
- `scripts/bot_path_trace.py`, `scripts/run_stockade_bot.py` — trace publication.

## See Also

- [Bot harness contract](../../specs/bot-harness.md).

Already-read party rosters are appended only when enabled to `bot-path-party.jsonl`; final trace emits `party_sample` and `wipe` only when a complete observed admitted-party roster is simultaneously dead. Partial deaths, missing members and target changes never imply a wipe. Host monotonic timestamps are explicitly `python_host_monotonic`, not comparable with Godot process ticks.

Known authored boss `spawn_guid`↔native entity mappings already held by the harness produce `spawn_state(alive=false)` only when that exact entity has an actual combat-log death. Missing deaths or unmapped trash remain UNKNOWN. No end-state ALIVE is inferred from the initial boss snapshot.

Private runs select the matching admin socket with `run_stockade_bot.py --server-port 5981`; default5871 preserves the existing owner workflow when no override is supplied. Server must itself bind the corresponding private socket.

Fresh preparation reuses `run_bot_roster.py --server-port 5981`: admin callbacks, private placement, token filename and native observer address all follow the same explicit port. Default5871 remains unchanged. Existing source gear/talent/native preparation is reused without stat or kit overrides; Stockade diagnostic attachment is a subsequent explicit server phase.

The first actual63s run exposed missing UnitPick tracks for three admitted companions. Capture now samples each admitted identity explicitly through the existing read-only `unit_transform(id)` API, using the runner's `admitted-party.json`; no new runtime/protocol observability. Missing transforms remain explicit `missing_admitted_actors` in raw samples. Owner position/facing still comes from account state. First failed-run SVG/PNG retain and label the three missing tracks; they are not retroactively qualified.

## Real Stockade proof — 2026-10-19

Capture revision `011ef565b`, native extension/CLI Rust `0a960bcf6` (unchanged Rust trees), server/renderer `48538fa49`, protocol `f9a9175`. All own artifacts built successfully; SHA receipts and exact commands in `/home/osso/.worktrees/logs/bot-path-map-2026-10-19/proof-ledger.json`. Native five-identity capture reproducer observed0→5 after the UnitPick omission fix; original pose/combat/wipe/boss and SVG fixtures pass. Default-disabled harness/report controls35 pass; actual private admin/provision subprocess tests2 pass. Independent retained planner/check gate remains lead-owned.

Authentic private5981 refresh captured52.365s, two acknowledged Heroic Throw openings,354 owner samples and290 samples for each of Garrick, Crenna, Shuja and Meredy; all five have facing.37 already-read roster samples,296 combat events, two observed combat starts/one end and one mob death. No boss completed and no party death: controller stopped on existing `Sourced route exhausted before goal; no direct-line fallback`. No retry, alternate route or fabricated completion. Actual walkability uses baked map34 polygons;143 source spawns use the shared formation identity. Unknown trash end-state remains gray UNKNOWN.

Inspected SVG/rsvg PNG pair: `/syncthing/AgentShared/2026-10-19/bot-path-map/stockade-five-bots-failed-52s.{svg,png}`. Visible heading was annotated FAILED52s/no bosses with the exact failure; floor/path geometry unchanged. The earlier63.222s failure pair `stockade-failed-63s-no-bosses.{svg,png}` retains three named missing visual tracks. It ended on a5308 outcome deadline without repeating the request; later server work stalled/disconnected and panicked inserting UnitThreatList into despawned227883v0 in `followers::party::advance_runs`. That server root cause belongs to separate owner work, not this capture fix. Neither record is a clear, death-free dungeon acceptance, clean exit or packet-loss diagnosis.

Private JSONL/logs/hashes/cleanup: `stockade-refresh-1/final-proof.json` and `stockade-real-1/` under the proof root above. All owned controller/client/server processes stopped, UDP5981 empty, both lead windows cleared. Only requested SVG/PNG files were published.
