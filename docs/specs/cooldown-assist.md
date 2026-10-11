# Cooldown Manager and Assisted Combat

Retail Mainline built-ins from Blizzard_CooldownViewer and C_AssistedCombat. Native client sources live in `godot/core/src/cooldown_assist.rs` and `godot/ui-model/src/cooldown_assist.rs`; [authored-data audit](../wiki/investigations/cooldown-assist-data.md) records provenance and current boundaries.

## What it must do
- [x] Read authored per-specialization cooldown categories, order and linked spells from local Retail DB2 exports.
- [x] Preserve authored rotation step/rule order; never substitute known-spell order or an invented rotation for an absent specialization.
- [x] Propagate unavailable rule evaluation on a potentially eligible higher-priority step instead of recommending a lower step as though it were false.
- [x] Model per-spec selection/reordering and cooldown fractions/charges from supplied authoritative samples.
- [x] Keep the optional matching Cooldown Viewer highlight disabled by default; document it as a World of Osso extension, not Retail parity.
- [ ] Render Essential/Utility cooldown icon rows and Tracked Buff icons/Bars for the active specialization.
- [ ] Settings allow choosing and reordering spells; Edit Mode persists viewer layouts.
- [ ] Swipes, charge counts and aura durations tick from real server state.
- [ ] Single-Button Assistant appears in the spellbook and casts the authored recommended next spell through normal server validation.
- [ ] Assisted Highlight marks matching action buttons and, when enabled, matching viewer icons.
- [ ] Both skins demonstrate all above behavior in private UDP5860 combat with an fb_* account, with inspected PNGs in AgentShared.

## How it works
- [Local data and decision boundaries](../wiki/investigations/cooldown-assist-data.md)

## Implementation inventory
- `scripts/export_db2_csv.py`: pinned six-table local-CASC export.
- `godot/core/src/cooldown_assist.rs`: authored joins and ordered recommendation mechanism with explicit predicate input.
- `godot/ui-model/src/cooldown_assist.rs`: selection/order and authoritative-sample projection; no mounted native UI yet.

## Tests asserting this spec
- `scripts/tests/test_cooldown_assist_export.py`: authentic DB2→CSV values and joins.
- `godot/core/tests/cooldown_assist.rs`: categories/links, authored rotation order, unknown-rule refusal, missing-spec behavior.
- `godot/ui-model/tests/cooldown_assist.rs`: fractions/charges, highlight option, per-spec selection/order.

## Known gaps (current cycle)
- [ ] Native renderer/settings/Edit Mode and runtime/server-state adapters remain unimplemented.
- [ ] Rotation predicate semantics and state capture require integration; two nonzero unidentified rule fields remain unresolved.
- [ ] Private both-skin combat proof, inspected screenshots and final checks remain outstanding.

## Out of scope
None beyond the user's named built-ins; no Forever-only behavior or invented spell priorities.
