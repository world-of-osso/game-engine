# GDScript completion-contract gate packet — 2026-10-18

## Scope and contract

All16 named candidates migrated to `godot/tests/completion.gd`; all original behavior predicates retained. Void assertion-bearing steps now return explicit checked success; typed awaited results are checked before use. Each complete skin/view/zoom/phase increments a fixed independently expected case count. Failure is sticky, prints `FAIL completion`, exits1, and cannot emit final success. A900s watchdog bounds abandoned coroutines.

`godot/network/src/fixture_completion.rs` accepts only exit0 +exactly one matching `ALL_CASES_DONE n/n` +no SCRIPT ERROR/ERROR:/FAIL line on either full stream. No startup/import allowlist. The native-input menu/sound Rust runners use this boundary; their GDScripts publish8/8 and3/3 before the real production Exit action, which still must terminate successfully. No product behavior changed. Other legacy native runners are outside this migration.

## Converted files and owner provenance

Provenance below is exactly historical inventory evidence, not current ownership inferred from Git authors. Every basename below is under `godot/tests/` and ends `.gd`.

| File | Fixed cases | Owner provenance from candidate-inventory.md |
|---|---:|---|
| campsite_selected_hover | 2 | Historical handoff-ui-questlog-collections.md:325; current owner unverified |
| communities_fidelity | 2 | Historical handoff-ui-gameplay-panels.md:87; released-slot provenance, current owner unverified |
| dropdown_caption_centering | 4 | Historical handoff-ui-questlog-collections.md:371; current owner unverified |
| guild_control_fidelity | 2 | No exact handoff; lead owner mapping required |
| guild_member_notes_live | 1 per invocation | No exact handoff; lead owner mapping required |
| guild_tabard_fidelity | 2 | Historical handoff-ui-gameplay-panels.md:13; released-slot provenance, current owner unverified |
| house_access_fidelity | 2 | No exact handoff; lead owner mapping required |
| hud_edit_controls_fidelity | 2 | No exact handoff; lead owner mapping required |
| loot_item_hitarea | 2 | No exact handoff; lead owner mapping required |
| minimap_zoom_fidelity | 6 | No exact handoff; lead owner mapping required |
| pvp_random_bonus | 4, plus32 action receipts | Current PvP owner9366; historical handoff-pvp-progression.md:7; migrated after lead included landed975719e64 |
| pvp_ready_placement | 8, plus16 actions/8 overlaps | Current PvP owner in report; inventory exact handoff absent |
| ready_check_difficulty | 22 | ui-questlog-collections current handoff explicitly runs it; historical exact ref23 |
| ready_check_portrait | 2 | Historical handoff-ui-questlog-collections.md:112; current owner unverified |
| ready_check_title_bounds | 2 | Historical handoff-ui-questlog-collections.md:94; current owner unverified |
| registration_flow | 2 | No exact handoff; lead owner mapping required |

Support files: `godot/tests/completion.gd`, `world_menu_flow.gd`, `world_sound_flow.gd`; `godot/network/src/{lib,fixture_completion}.rs`; `godot/network/examples/gdscript_completion_fixture.rs`; `godot/network/examples/native_input_fixture/{menu,sound}.rs`; [spec](../specs/gdscript-completion.md); [wiki](../wiki/investigations/gdscript-awaited-completion.md) and wiki index/log; this packet.

## Proof ledger

Canonical evidence root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/gdscript-completion-contract-2026-10-18/`.

| Proof | Exact scope/result | Artifact |
|---|---|---|
| Historical requalification | Sent BEFORE implementation:2 VALID/3 SUSPECT/11 UNKNOWN; missing counts/streams not promoted | `requalification.md`, `requalification-search.txt` |
| RED process regression | Old exit-only validator accepted actual nested-awaited assert +forged completion +exit0; regression failed as intended | `red-runner.log`, `red-depot-test.log` |
| GREEN process/stream/parser tests |8dcdc8a1d:3 passed; real legacy nested failure rejected, checked nested failure nonzero/no marker, success accepted; marker/stream counterexamples;17 isolated native-free script parse checks | `final-focused-runner.log`, `final-focused-depot-test.log` |
| Extension +standalone runner build |aa09dfa35: helper built extension and gdscript_completion_fixture successfully; no compiler warnings | `build.log` |
| Actual-extension member-note parse |aa09dfa35: exit0/empty stderr; parser-only, not live behavior | `native/guild_member_notes_live.{stdout,stderr}.log` |
| Headless ready placement, run once |aa09dfa35: exit0, final8/8 exactly once,16 actions/8 overlaps, failures[], empty stderr, clean stdout | `native/pvp_ready_placement.{stdout,stderr}.log`, `placement-new-cases.json`, `pvp_ready_placement-invocation.log` |
| Headless Random32, run once |aa09dfa35: exit0, final4/4 exactly once,32 actions, empty stderr, clean stdout; original32-action/4-case check retained | `native/pvp_random_bonus.{stdout,stderr}.log`, `random-new-cases.json`, `pvp_random_bonus-invocation.log` |
| Scoped check |6c3a4c57b: network library +both runner examples, exit0/no warnings; helper API check (no direct Cargo invocation) | `check-helper.py`, `cargo-check.log` |
| Format |6c3a4c57b: changed Rust package cargo fmt --check exit0 | `fmt-check.log` |

The6c3a4c57b merge changed docs only; build/headless evidence remains valid. Later69212f71a deletes only an unreachable duplicate Communities return; other script/Rust evidence remains valid. Its focused parser passed exit0/empty stderr: `communities-final-parse-invocation.log`, `native/communities_fidelity.{stdout,stderr}.log`. Failed intermediate development logs remain present and are not accepted proofs. Initial import is separate (`import.log`): Godot editor thread/dialog ERROR diagnostics occurred there; they were not suppressed or allowed in either accepted fixture stream. No live server or rendered client was started.

Built extension SHA256: `9690d3fd608f9fb5c95e8472f045c0504011c13c167c0502dc7c44a1b9b2f005`.

## Commands

Worktree `W=/home/osso/.worktrees/game-engine-gdcontract`; evidence root `D` as above. Every command below runs with `CAPACITY_OWNER=gdcontract`; build/test commands also use `SIBLING_SHARED_PROTOCOL=/home/osso/.worktrees/shared-protocol`. Native admission used capacity.sh before builds. Godot binary `GODOT_BIN=/home/osso/.cache/game-engine/godot/4.7.2-pr123946-pr123546/godot-4.7.2-pr123946-pr123546`.

```text
$W/scripts/agent/agent-run gdcontract python3 $W/scripts/engine-build.py --root $W --test -p game-engine-network --lib fixture_completion::tests -- --nocapture
$W/scripts/agent/agent-run gdcontract python3 $W/scripts/engine-build.py --root $W --fixture gdscript_completion_fixture
$W/scripts/agent/agent-run gdcontract python3 $D/check-helper.py
$W/scripts/agent/agent-run gdcontract cargo fmt --manifest-path $W/godot/network/Cargo.toml --check
$W/scripts/agent/agent-run gdcontract timeout 180 $GODOT_BIN --headless --path $W/godot --editor --import
$W/scripts/agent/agent-run gdcontract $W/target/debug/examples/gdscript_completion_fixture $W guild_member_notes_live $D/native --check
$W/scripts/agent/agent-run gdcontract $W/target/debug/examples/gdscript_completion_fixture $W pvp_ready_placement $D/native --headless
$W/scripts/agent/agent-run gdcontract $W/target/debug/examples/gdscript_completion_fixture $W pvp_random_bonus $D/native --headless
```

Headless invocation env also set private `XDG_CONFIG_HOME=$D/private-config`, `PVP_READY_PLACEMENT_RECEIPT=$D/placement-new-cases.json`, `PVP_RANDOM_BONUS_RECEIPT=$D/random-new-cases.json`. The format command ran with working directory `$W/godot/network`. RED selected only nested_awaited_failure_is_not_a_pass with `-- --ignored --nocapture`; GREEN removed the RED ignore after fixing the validator.

## Pending lead-scheduled windows

After landing, one batched rendered requalification from landed master: campsite, Communities, dropdown, guild control, guild tabard, House Access, HUD edit, loot, minimap, ReadyCheck difficulty/portrait/title bounds. These12 offline fixtures await `RenderingServer.frame_post_draw` and inspect screenshots/pixels; Dummy/headless parsing is not behavior acceptance. No shared UDP ports.

Second private-server window: `guild_member_notes_live` (existing real guild/notes/master/self/readback persistence choreography) and `registration_flow` (private UDP5300, fb_regtest1 and approval barrier). No server started. Every FAIL will be classified with evidence and sent to lead; product code will not be modified for requalification findings.
