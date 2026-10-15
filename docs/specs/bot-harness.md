# Player-bot test harness

Private real player characters exercise the native controller path and measure class/spec damage without tuning gameplay. Sources and proof boundaries live in [the harness wiki](../wiki/systems/bot-harness.md). Source: `scripts/bot_harness.py`.

## What it must do

- [ ] Maintain one real player per class/spec, including player stats, talents and sourced rotations.
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

## Native race fixture

Northshire Shaman bots use source-valid Draenei11 up front, not a retry-based alternative after Dwarf rejection. Other classes use the lowest source-valid Alliance race. Missing authored source pairs block provisioning. This fixture choice does not close the server's separately tracked Dwarf/Shaman eligibility gap.

## Comparable equipment budget

Every stat-bearing armor slot and preferred weapon uses authentic Item/ItemSparse at the same raw item-level30, quality2 and squish-era0, within the character's level requirement. Armor follows ChrClasses.ArmorTypeMask (cloaks use their native cloth class); positive primary-stat allocation follows ChrSpecialization.PrimaryStatPriority. Record item IDs, raw ilvl, quality, native owner GUIDs and visible native primary stats per spec in fixture-receipt.json. Comparison keys include observed character level, native dummy level/max-health/difficulty, source world.db SHA-256 and duration; roster comparison withholds medians when any expected DPS repetition is absent/incomplete or fewer than three DPS specs qualify. Neck, both rings and both trinkets must be empty for every bot, removed through ordinary paperdoll-to-empty-bag input and confirmed by authoritative equipment/bag snapshots. Storage bags remain equipped and are recorded separately from the combat budget. Missing source slots, mismatched native equipment or unbudgeted stat-bearing equipment block comparison; no changed item/stat definitions or alternate budget.

## How it works

- [Evidence and detection boundaries](../wiki/systems/bot-harness.md)
- [Native automation contract](native-ui-automation.md)

## Implementation inventory

- `scripts/bot_harness.py`: native combat parsing, request/result reconciliation, fixed-window DPS and comparison reporting.
- `scripts/bot_ability_summary.py`: read-only expected-roster JSON/Markdown per-ability request/success/event-damage/missing/unused breakdown. Keep child event IDs and untagged native errors unattributed; preserve absent specs/repetitions.
- `scripts/tests/test_bot_harness.py`: behavioral detector and comparable-fixture regressions.
- `scripts/bot_rotation.py`: sourced metadata, absent-order generic rule and fail-closed native authored selection.
- `scripts/run_dummy_bots.py`: repeated explicit-socket fixed-duration collection and raw receipts.
- `godot/tests/bot_harness_observer.gd`: existing native account/spell/aura/charge/assisted observation and private spec selection.
- `scripts/tests/fixtures/bot_harness/`: authentic native Stockade and own private dummy combat with provenance. Stockade's absent request is a labeled injection; Ret's missing request is an actual captured submission.
- `scripts/bot_roster.py` / `scripts/run_bot_roster.py`: source40-spec census and isolated private provisioning/client lifecycle.

## Tests asserting this spec

- `scripts/tests/test_bot_harness.py`
- `scripts/tests/test_bot_rotation.py`

## Known gaps (current cycle)

- [x] Three-DPS common-budget dummy MVP: Arms71/Fury72/Havoc577 each complete two60s windows with matching fixture keys and empty jewelry; native-equal-budget2 retains receipts and findings. This qualifies only the phase gate, not balance or all-spec correctness. Arcane12408/20, Ret6425/equality, Elemental38530/17 and Frost33686/67 refusals remain recorded.
- [x] Comparable spec-correct gear/stat receipts for the three-spec MVP: Arms10 slots, Fury/Havoc11 slots at30/quality2/era0, visible native stats and owner GUIDs; Fury native dual-two-hand25129 succeeds. Havoc jewelry moves normally into bag0/1 and retains all four storage-bag IDs/GUIDs/counts. Other specs remain unverified; no gear fallback.
- [ ] Aura-effect, cooldown reset, charge and resource-spending observability; combat-log aura presence alone proves none of these.
- [ ] All-spec comparable damage receipts beyond the completed three-spec MVP.

## Out of scope

Gameplay fixes, stat tuning and balance changes belong to owning agents via the lead. No external damage oracle or fabricated rotations.
