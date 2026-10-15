# Player-bot test harness

Private real player characters exercise the native controller path and measure class/spec damage without tuning gameplay. Sources and proof boundaries live in [the harness wiki](../wiki/systems/bot-harness.md). Source: `scripts/bot_harness.py`.

## What it must do

- [ ] Maintain one real player per class/spec, including player stats, talents and sourced rotations.
- [ ] An explicitly selected, user-approved pinned SimulationCraft APL provider may supply priorities; record raw URL, commit and SHA-256. It is not an alternate path after a native authored-evaluator error. Unknown potentially applicable predicates block; spells remain constrained by actual learned/native usability facts, with no buffs, kit or budget changes.
- [ ] Read-only native `spell_priority_state(target, spell_ids)` exposes requested learned/readiness/affordability/range facts, owner haste, native spell GCD metadata and observed target casting. Invalid queries or unavailable native data return explicit errors; querying an unselected unit must neither select/cast at it nor alter its health.
- [ ] Deliver three DPS-spec dummy runs first, then all specs and repeated Stockade runs with followers, headless on private UDP 5871.
- [x] Derive generic direct-target abilities from SpellMisc, SpellPower, SpellCooldowns, SpellEffect and SpellCastTimes; preserve coefficients and exclude passive spells.
- [x] Where installed AssistedCombatStep orders a spec, consume the existing native authored evaluator; unsupported/loading predicates block rather than silently enabling generic behavior. Only absent authored order permits the labeled generic coefficient rule.
- [ ] Capture casts that error or never fire, abilities never used, zero-damage damage spells, ineffectual auras, cooldown/charge/resource violations, deaths and server errors per run.
- [ ] Report unavailable detection evidence explicitly, never treating missing observability as passing.
- [x] Compute DPS over the full specified dummy window, excluding follower damage and other targets; incomplete collections cannot qualify as balance evidence.
- [x] Attribute owned pet/guardian damage through spell-driven Summon events and observed persistent-owner relations; exclude follower roster actors and their pets. Unknown attribution withholds DPS.
- [x] Emit Retail-style Summon only for actual spell-created units, with source=summoner, target=spawned unit and the actual summoning spell. Non-spell/login/journal spawns do not fabricate events or spell IDs.
- [ ] Compare DPS specs only with recorded comparable stats, level, dummy and duration. Report median-relative spread without pass/fail until lead/user chooses tolerance. Never use external DPS numbers.
- [ ] Write JSON and short Markdown per run beneath `data/diagnostics/bot-harness-<date>/`.
- [x] Behavioral detection tests feed actual native combat records and a known missing request; distinguish zero damage from no cast and consume each server success once. Own private Ret32 fixture retains actual submitted20271 request1792056099829 without a success, three real successes and899 damage.
- [x] Merge overlapping combat polls without double counting or collapsing real duplicate events; identify overflow using observed ring retention.

## How it works

- [Evidence and detection boundaries](../wiki/systems/bot-harness.md)
- [Native automation contract](native-ui-automation.md)

## Implementation inventory

- `scripts/bot_harness.py`: native combat parsing, request/result reconciliation, fixed-window DPS and comparison reporting.
- `scripts/tests/test_bot_harness.py`: behavioral detector and comparable-fixture regressions.
- `scripts/bot_rotation.py`: sourced metadata, absent-order generic rule and fail-closed native authored selection.
- `scripts/run_dummy_bots.py`: repeated explicit-socket fixed-duration collection and raw receipts.
- `godot/tests/bot_harness_observer.gd`: existing native account/spell/aura/charge/assisted observation and private spec selection.
- `godot/rust/src/spells/assisted_combat.rs`: shared native usability facts and read-only requested-spell snapshot; no target mutation.
- `godot/tests/spell_priority_facts_live.gd`: native selected/unselected query and invalid-input response fixture.
- `scripts/tests/fixtures/bot_harness/`: authentic native Stockade and own private dummy combat with provenance. Stockade's absent request is a labeled injection; Ret's missing request is an actual captured submission.
- `scripts/bot_roster.py` / `scripts/run_bot_roster.py`: source40-spec census and isolated private provisioning/client lifecycle.

## Tests asserting this spec

- `scripts/tests/test_bot_harness.py`
- `scripts/tests/test_bot_rotation.py`

## Known gaps (current cycle)

- [ ] Complete three-DPS sustained dummy MVP before all-spec/Stockade phase expansion. Native provisioning/spec selection/dummy casts work, but source evaluator stops Arcane12408/20, Ret6425/equality, Elemental38530/17 and Frost33686/67. No fallback and no sustained-DPS claim.
- [ ] Comparable spec-correct gear/stat receipts. Sourced native talent purchase and Ret fresh-login12/12-rank persistence are verified; source weapon-plan execution remains a separate gate. Fury's dual-two-hand kit is blocked by unmodelled Titan's Grip; no gear fallback.
- [ ] Aura-effect, cooldown reset, charge and resource-spending observability; combat-log aura presence alone proves none of these.
- [ ] Three-spec and all-spec comparable damage receipts.

## Out of scope

Gameplay fixes, stat tuning and balance changes belong to owning agents via the lead. No external damage oracle or fabricated rotations.
