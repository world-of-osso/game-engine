# Player-bot test harness

Private real player characters exercise the native controller path and measure class/spec damage without tuning gameplay. Sources and proof boundaries live in [the harness wiki](../wiki/systems/bot-harness.md). Source: `scripts/bot_harness.py`.

## What it must do

- [ ] Maintain one real player per class/spec, including player stats, talents and sourced rotations.
- [ ] Deliver three DPS-spec dummy runs first, then all specs and repeated Stockade runs with followers, headless on private UDP 5871.
- [ ] Derive abilities from SpellMisc, SpellPower, SpellCooldowns, SpellEffect, Retail UI cache and TrinityCore a352b1fa. Without sourced priorities, label the generic highest-DB2-damage-coefficient rule; never invent a priority list.
- [ ] Capture casts that error or never fire, abilities never used, zero-damage damage spells, ineffectual auras, cooldown/charge/resource violations, deaths and server errors per run.
- [ ] Report unavailable detection evidence explicitly, never treating missing observability as passing.
- [ ] Measure sustained DPS over the full fixed dummy duration, excluding follower damage and other targets.
- [ ] Compare DPS specs only with recorded comparable stats, level, dummy and duration. Report median-relative spread without pass/fail until lead/user chooses tolerance. Never use external DPS numbers.
- [ ] Write JSON and short Markdown per run beneath `data/diagnostics/bot-harness-<date>/`.
- [ ] Behavioral detection tests feed actual native combat records and a known missing request; distinguish zero damage from no cast and consume each server success once.

## How it works

- [Evidence and detection boundaries](../wiki/systems/bot-harness.md)
- [Native automation contract](native-ui-automation.md)

## Implementation inventory

- `scripts/bot_harness.py`: native combat parsing, request/result reconciliation, fixed-window DPS and comparison reporting.
- `scripts/tests/test_bot_harness.py`: behavioral detector and comparable-fixture regressions.
- `scripts/tests/fixtures/bot_harness/`: authentic native Stockade combat with provenance. The missing request is a labeled test injection, not a production failure claim.

## Tests asserting this spec

- `scripts/tests/test_bot_harness.py`

## Known gaps (current cycle)

- [ ] Native bot provisioning, sourced spec rotations, dummy execution and Stockade repetition.
- [ ] Aura-effect, cooldown reset, charge and resource-spending observability; combat-log aura presence alone proves none of these.
- [ ] Three-spec and all-spec comparable damage receipts.

## Out of scope

Gameplay fixes, stat tuning and balance changes belong to owning agents via the lead. No external damage oracle or fabricated rotations.
