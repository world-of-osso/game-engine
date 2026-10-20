# Cooldown Manager and Assisted Combat

Retail Mainline built-ins from Blizzard_CooldownViewer and C_AssistedCombat. [Authored-data audit](../wiki/investigations/cooldown-assist-data.md) owns provenance, implementation boundaries and proof.

## What it must do
- [x] Read authored per-specialization cooldown categories, order and linked spells from local Retail DB2 exports.
- [x] Preserve authored rotation step/rule order; never substitute known-spell order or an invented rotation for an absent specialization.
- [x] Pet-presence predicates57/58 consume the owner's PetSpells/PetClearSpells state. BM rule29672 permits authored Call Pet883 only without the controlled pet; missing readings and nonzero operands remain errors. This does not certify dead-pet, guardian-only or full rotation behavior.
- [x] Caster-centered spells using authored SpellRange1 (Self Only, [0,0]) are not excluded by distance to the selected hostile. Targeted spells retain their existing hostile min/max range checks. This does not infer effect radius, nearby-enemy conditions or body-range adjustments.
- Type67 with exactly `[0,0,0]` adds no Boolean restriction to an authored step; any nonzero operand still refuses. Preserve ordinary learned/readiness/affordability/range checks and existing casting/outstanding-input gates. Default anti-overlap behavior for steps without this modifier remains a separate parity gap; other unsupported predicates remain separate.
- [x] Propagate unavailable rule evaluation on a potentially eligible higher-priority step instead of recommending a lower step as though it were false.
- [x] Render Essential/Utility cooldown icon rows and Tracked Buff icons/Bars for the active specialization. Default buff-bar rows retain CooldownViewerBuffBarItemTemplate220×30, with188×19 status bar at(32,5.5), source textured BG/fill/Pip, XML tint(1,.5,.25), cropped remaining fraction and a source-sized normal-alpha pip following the fill edge; zero remaining time hides fill/pip. Applications uses the XML32×10 fontstring at icon BOTTOMRIGHT(−5,+5), clear of the icon corner.
- [x] Settings allow choosing and reordering spells; Edit Mode persists viewer layouts. Settings uses the source399×609 ButtonFrame shell, LargeSideTab buttons, SettingsItem/BarItem sizes, and character-panel background anchored to its389×523 inset at(4,60). Known/unlocked preview bars use the source default tint(1,.5,.25).
- [x] Sample authoritative cooldowns and raw aura clocks; active tracked auras expire without hiding the ordinary spell cooldown.
- [x] Show authoritative current charge counts and recharge clocks. Both-skin private Crusader Strike changes2→1 with a positive recharge clock; initial full-category publication is tested through real owner loopback.
- [x] Keep matching Cooldown Viewer highlighting independently configurable and disabled by default; this extends Retail action-button highlighting.
- [x] Single-Button Assistant appears in the spellbook and, when a source-derived recommendation is available, casts that exact spell through normal server validation. Both skins prove actual35395 CastSuccess; unsupported rotation rules remain separate gaps.
- [x] Assisted Highlight marks matching action buttons and, when enabled, matching viewer icons. Both consume the same recommendation; both skins prove visible advancing Retail ants in private combat.
- [x] Both skins demonstrate the source-supported behavior above in private combat with an fb_* account, with inspected PNGs in AgentShared. This is not full all-spec rotation certification.

## User-approved spatial policy

`ASSISTED_COMBAT_SIMC_MIRROR`, approved 2026-10-17, is project policy—not verified Retail behavior. Type3 applies generically to all92 authored build70448 rules (extended2026-10-20 after Azure rule1252 refusal); type12 remains limited to Whirlwind190411 rule1268. Type3 compares current target centre-to-centre3D distance `<= v1`; no target returns false. Type12 compares count `> v1`: count living hostile replicated units containing the local player in `UnitThreatList`, within inclusive8yd of the player's current local cast position. Party-only aggro does not count.8yd matches build70448 Whirlwind SpellRadius14. Both require v2/v3zero; other type12 rule IDs retain refusal. Missing/invalid positions of otherwise eligible count members leave count unavailable rather than undercounting.

Type3 follows pinned SimC76f8118 `player.cpp:3839–3841` (`target.distance<=v1`); type12 follows SimC1860cd1d `player.cpp:3635–3642`; radius, centre convention and threat membership are explicit user choices. [Audit/provenance](../wiki/investigations/cooldown-assist-data.md#assisted_combat_simc_mirror) records the source boundary. Concrete policy tests and captured Fury replay use synthetic new spatial readings, never relabel historical failure as native success.

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
- `godot/tests/cooldown_buff_bar_fidelity.gd`: offline actual native projection of concrete full/half/empty bar samples in both skins1920×1080. Proves source default bar art/geometry/progress, not live aura authority or icon-mask/font-family parity.
- `godot/tests/cooldown_assist_live.gd`: real private authentication; grounded authority/prediction, options/settings/reorder, Edit Mode/persistence, spellbook drag, real aura/cooldown clocks, matching animated surfaces and own authoritative CastSuccess. Asserts an authoritative charge2→1 with positive recharge and captures a distinct post-cast recommendation with the exact same-decision evaluator inputs. Pre-cast action/viewer ants must both advance; moving Tab candidates must admit actual Judgment before combat acceptance. Partial runs exit2, never PASS combat acceptance.
- `scripts/tests/run_cooldown_assist_live.py`: private orchestration, owner-checked build-qualified placement before login, marker rendezvous and owned client cleanup.
- `godot/core/tests/cooldown_assist_live_replay.rs`: replay both skins' recorded Holy Power, cooldowns/charges, affordability, auras and target through the authored evaluator; assert the exact first eligible step, not mere rotation membership. Those two manual skin cases are explicitly ignored without private recordings; run them with `COOLDOWN_ASSIST_PROOF_DIR` and `--ignored`. A spender loses eligibility when the recorded resource is removed. Automatic captured-player fixtures additionally assert Resto/Elemental's named fail-closed boundaries and Fire's sourced Fireball133 after zero-operand67 recognition, with readiness/cost/range and nonzero-operand controls.

## Known gaps (current cycle)

Verified: 2026-10-12.
Private UDP5863, fresh `fb_cooldown_assist3`/Coolproof30, specialization70. Post-merge `finish-modern` and `finish-forever` prove four categories, settings, saved Edit Mode placement, real assistant drag to slot6, matching animated action/viewer highlights and own authoritative35395 CastSuccess. Screenshots are combat proof for this concrete source-supported sample, **not full all-spec rotation certification**.

- [ ] Exact numeric/charge/predictive rotation acceptance remains partial. Source-decoded raw resource, health-percent, charge and cooldown-clock comparisons are enabled only away from equality, where strict/inclusive interpretations agree. Outside the bounded policy above, equality, mana-percent units, distance/body-radius, aura-duration/stack/count/nearby targeting, nonzero67 operands and nonzero chargeValue2 remain explicit errors. Default anti-overlap behavior for unflagged steps is a separate parity gap. SimC's decoder explicitly marks strictness TODO; its chosen equality results are not Retail proof. Two unidentified rule fields remain unresolved; never guess them.
- [x] Effective nonzero caster costs/modifiers use owner-only server `SpellCostSnapshot` readings from normal cast admission. Raw300rage admission, wire/error preservation, native replacement cache and live recommendation consumption pass.
- [x] Private combat acceptance uses the corrected build-qualified destination from the [existing terrain investigation](../../../game-server/docs/wiki/investigations/private-terrain-destinations.md). The maintained driver uses owner-checked placement before login; both authority and prediction are asserted on the authored surface. No physics workaround.
- [x] Both highlight surfaces use Retail `rotationhelper_ants_flipbook` (member32008, FDID6992992), centered66px at45px button scale,30 frames in a5×6 grid per second; hidden surfaces restart at frame0. The action highlight is a centered45px Frame containing the centered66px ARTWORK Flipbook (`Blizzard_ActionBar/Shared/ActionButtonComponentTemplate.xml:67-87`; `Mainline/AssistedCombatManager.lua:219-234`), scaled uniformly with the existing button in both skins; Forever adds no highlight layout or behavior. Both-skin rendered animation and charge counts pass; fresh-process selection/order and account-layout restoration also pass in both skins.
- [ ] `ASSISTANT_GCD_ORIGIN_1229376`: the description states25% longer GCD, but exact local1229376/470418 SpellCooldowns/SpellCategory/SpellEffect/SpellAuraOptions rows and checked pinned TC cast/GCD paths do not encode where/how to apply it or preserve origin. [Exact checked rows and paths](../wiki/investigations/cooldown-assist-data.md#assistant_gcd_origin_1229376). Retain a named source gap; do not infer the multiplier policy from Dummy BasePoints25.

## Out of scope
Boss timelines/warnings belong to the boss-timeline agent. PLAN.md is read-only. No invented priorities, no Forever-only UI inventory, no shared UDP5000 operations.
