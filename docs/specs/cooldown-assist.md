# Cooldown Manager and Assisted Combat

Retail Mainline built-ins from Blizzard_CooldownViewer and C_AssistedCombat. [Authored-data audit](../wiki/investigations/cooldown-assist-data.md) owns provenance, implementation boundaries and proof.

## What it must do
- [x] Read authored per-specialization cooldown categories, order and linked spells from local Retail DB2 exports.
- [x] Preserve authored rotation step/rule order; never substitute known-spell order or an invented rotation for an absent specialization.
- [x] Propagate unavailable rule evaluation on a potentially eligible higher-priority step instead of recommending a lower step as though it were false.
- [x] Render Essential/Utility cooldown icon rows and Tracked Buff icons/Bars for the active specialization.
- [x] Settings allow choosing and reordering spells; Edit Mode persists viewer layouts.
- [x] Sample authoritative cooldowns and raw aura clocks; active tracked auras expire without hiding the ordinary spell cooldown.
- [x] Show authoritative current charge counts and recharge clocks. Both-skin private Crusader Strike changes2→1 with a positive recharge clock; initial full-category publication is tested through real owner loopback.
- [x] Keep matching Cooldown Viewer highlighting independently configurable and disabled by default; this extends Retail action-button highlighting.
- [x] Single-Button Assistant appears in the spellbook and, when a source-derived recommendation is available, casts that exact spell through normal server validation. Both skins prove actual35395 CastSuccess; unsupported rotation rules remain separate gaps.
- [x] Assisted Highlight marks matching action buttons and, when enabled, matching viewer icons. Both consume the same recommendation; both skins prove visible advancing Retail ants in private combat.
- [x] Both skins demonstrate the source-supported behavior above in private combat with an fb_* account, with inspected PNGs in AgentShared. This is not full all-spec rotation certification.

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
- `godot/tests/cooldown_assist_live.gd`: real private authentication; grounded authority/prediction, options/settings/reorder, Edit Mode/persistence, spellbook drag, real aura/cooldown clocks, matching animated surfaces and own authoritative CastSuccess. Asserts the recommended Ret charge spell (Crusader Strike or Judgment) consumes exactly one authoritative charge with positive recharge, then the recommendation changes to the other ready authored spell with matching advancing highlights. Partial runs exit2, never PASS combat acceptance.
- `scripts/tests/run_cooldown_assist_live.py`: private orchestration, owner-checked build-qualified placement before login, marker rendezvous and owned client cleanup.

## Known gaps (current cycle)

Verified: 2026-10-12.
Private UDP5863, fresh `fb_cooldown_assist3`/Coolproof30, specialization70. Post-merge `finish-modern` and `finish-forever` prove four categories, settings, saved Edit Mode placement, real assistant drag to slot6, matching animated action/viewer highlights and own authoritative35395 CastSuccess. Screenshots are combat proof for this concrete source-supported sample, **not full all-spec rotation certification**.

- [ ] Exact numeric/charge/predictive rotation acceptance remains partial. Source-decoded raw resource, health-percent, charge and cooldown-clock comparisons are enabled only away from equality, where strict/inclusive interpretations agree. Equality, mana-percent units, distance/body-radius, aura-duration/stack/count/nearby targeting, predictive67 and nonzero chargeValue2 remain explicit errors. SimC's decoder explicitly marks strictness TODO; its chosen equality results are not Retail proof. Two unidentified rule fields remain unresolved; never guess them.
- [x] Effective nonzero caster costs/modifiers use owner-only server `SpellCostSnapshot` readings from normal cast admission. Raw300rage admission, wire/error preservation, native replacement cache and live recommendation consumption pass.
- [x] Private combat acceptance uses the corrected build-qualified destination from the [existing terrain investigation](../../../game-server/docs/wiki/investigations/private-terrain-destinations.md). The maintained driver uses owner-checked placement before login; both authority and prediction are asserted on the authored surface. No physics workaround.
- [x] Both highlight surfaces use Retail `rotationhelper_ants_flipbook` (member32008, FDID6992992), centered66px at45px button scale,30 frames in a5×6 grid per second; hidden surfaces restart at frame0. The action highlight is a centered45px Frame containing the centered66px ARTWORK Flipbook (`Blizzard_ActionBar/Shared/ActionButtonComponentTemplate.xml:67-87`; `Mainline/AssistedCombatManager.lua:219-234`), scaled uniformly with the existing button in both skins; Forever adds no highlight layout or behavior. Both-skin rendered animation and charge counts pass; fresh-process selection/order and account-layout restoration also pass in both skins.
- [ ] The assistant spellbook description states a25% longer global cooldown. The existing actual-spell interception does not yet preserve that authoritative assistant-cast penalty; this remains an execution gap, not a guessed Dummy-effect interpretation.

## Out of scope
Boss timelines/warnings belong to the boss-timeline agent. PLAN.md is read-only. No invented priorities, no Forever-only UI inventory, no shared UDP5000 operations.
