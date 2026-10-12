# Cooldown Manager and Assisted Combat

Retail Mainline built-ins from Blizzard_CooldownViewer and C_AssistedCombat. [Authored-data audit](../wiki/investigations/cooldown-assist-data.md) owns provenance, implementation boundaries and proof.

## What it must do
- [x] Read authored per-specialization cooldown categories, order and linked spells from local Retail DB2 exports.
- [x] Preserve authored rotation step/rule order; never substitute known-spell order or an invented rotation for an absent specialization.
- [x] Propagate unavailable rule evaluation on a potentially eligible higher-priority step instead of recommending a lower step as though it were false.
- [x] Render Essential/Utility cooldown icon rows and Tracked Buff icons/Bars for the active specialization.
- [x] Settings allow choosing and reordering spells; Edit Mode persists viewer layouts.
- [x] Sample authoritative cooldowns and raw aura clocks; active tracked auras expire without hiding the ordinary spell cooldown.
- [ ] Show authoritative current charge counts and recharge clocks. Category joins and projection tests pass; native charge snapshots need live proof.
- [x] Keep matching Cooldown Viewer highlighting independently configurable and disabled by default; this extends Retail action-button highlighting.
- [ ] Single-Button Assistant appears in the spellbook and casts the exact authored recommended next spell through normal server validation. Spellbook/button placement is proved; full evaluation/execution is not.
- [ ] Assisted Highlight marks matching action buttons and, when enabled, matching viewer icons. Both consume the same recommendation; rendered combat acceptance remains blocked.
- [ ] Both skins demonstrate all above behavior in private combat with an fb_* account, with inspected PNGs in AgentShared.

## How it works
- [Authored-data audit, native integration and proof boundaries](../wiki/investigations/cooldown-assist-data.md)

## Implementation inventory
- `scripts/export_db2_csv.py`: six-table local-CASC export.
- `godot/core/src/cooldown_assist.rs`: authored joins and ordered recommendation mechanism.
- `godot/core/src/assisted_combat.rs`: qualitative predicates; unsupported semantics refuse recommendations.
- `godot/core/src/cooldown_conditions.rs` and `cooldown_modifiers.rs`: referenced PlayerCondition masks/spell/aura/spec and authentic ModifierTree known-spell/aura gates; unsupported fields fail explicitly.
- `godot/ui-model/src/cooldown_assist.rs`: per-spec choice/order and supplied-sample projection.
- `godot/ui-model/src/cooldown_viewer_component.rs` and `cooldown_settings_component.rs`: four viewer roots and category/select/reorder controls.
- `godot/rust/src/spells/cooldown_viewer.rs`: native loading/mounting, cooldown/charge/raw-aura sampling, choice persistence and ordinary settings actions.
- `godot/rust/src/spells/assisted_combat.rs`: native facts, availability/recommendation/error state. Casting/action-bar adapters resolve the assistant and share its recommendation with both highlight surfaces.
- HUD Options enable the manager, action highlight and matching-icon highlight. Four viewer roots participate in the existing Edit Mode system.

## Tests asserting this spec
- `scripts/tests/test_cooldown_assist_export.py`: authentic DB2 values and joins; no encrypted rows dropped.
- `godot/core/tests/cooldown_assist.rs`: categories/links, authored order, unknown-rule refusal, readiness, missing specs, all referenced condition loading, signed masks and inactive ContentTuning flags.
- `godot/ui-model/tests/cooldown_assist.rs`: fractions/charges, aura precedence/expiry, both-skin frame inventory/highlight visibility, per-spec selection/order and registry click emission.
- `godot/tests/cooldown_assist_live.gd`: real private authentication; grounded authority/prediction, options/settings/reorder, Edit Mode/persistence, spellbook drag, real aura/cooldown clocks, matching animated surfaces and own authoritative CastSuccess. Partial runs exit2, never PASS combat acceptance.
- `scripts/tests/run_cooldown_assist_live.py`: private orchestration, owner-checked build-qualified placement before login, marker rendezvous and owned client cleanup.

## Known gaps (current cycle)

Verified: 2026-10-11.
Private UDP5860, `fb_cooldown_assist`/Coolproof, specialization70. Modern run10 and Forever run9 show all four viewer categories, settings, saved Edit Mode placement, spellbook and the actual assistant action on visible slot6. Screenshots are bounded UI proof, **not combat or full rotation certification**.

- [ ] Exact numeric/charge/predictive rotation acceptance remains partial. Source-decoded raw resource, health-percent, charge and cooldown-clock comparisons are enabled only away from equality, where strict/inclusive interpretations agree. Equality, mana-percent units, distance/body-radius, aura-duration/stack/count/nearby targeting, predictive67 and nonzero chargeValue2 remain explicit errors. SimC's decoder explicitly marks strictness TODO; its chosen equality results are not Retail proof. Two unidentified rule fields remain unresolved; never guess them.
- [ ] Effective nonzero caster costs/modifiers now use owner-only server `SpellCostSnapshot` readings from normal cast admission; client cache/wire tests and live consumption must pass before certification.
- [ ] Private combat acceptance is pending with the corrected build-qualified destination. The [existing terrain investigation](../../../game-server/docs/wiki/investigations/private-terrain-destinations.md) traced falls to stale fixture Z, not missing tiles. The maintained driver uses `place-private-character.py` before login; the native fixture requires both authority and prediction on the authored surface. No physics workaround.
- [x] Both highlight surfaces use Retail `rotationhelper_ants_flipbook` (member32008, FDID6992992), centered66px at45px button scale,30 frames in a5×6 grid per second; hidden surfaces restart at frame0. Live rendered animation, charge counts and fresh-process layout restoration remain proof gates.
- [ ] The assistant spellbook description states a25% longer global cooldown. The existing actual-spell interception does not yet preserve that authoritative assistant-cast penalty; this remains an execution gap, not a guessed Dummy-effect interpretation.

## Out of scope
Boss timelines/warnings belong to the boss-timeline agent. PLAN.md is read-only. No invented priorities, no Forever-only UI inventory, no shared UDP5000 operations.
