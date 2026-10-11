# Cooldown Viewer and Assisted Combat authored data
verified: 2026-10-11

## Sources
Retail Mainline Lua under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:
`Blizzard_APIDocumentationGenerated/{CooldownViewerDocumentation,CooldownViewerConstantsDocumentation,AssistedCombatDocumentation}.lua`,
`Blizzard_ActionBar/Mainline/AssistedCombatManager.lua`.
Exact table layouts: wowdev/WoWDBDefs `definitions/<table>.dbd`.
Rotation condition reference: simulationcraft/simc midnight `engine/dbc/data_enums.hh` and `engine/player/player.cpp:3697` (primary implementation; its comments explicitly leave comparator precision uncertain).

## Local audit
Pinned CSVs previously contained none of the six tables below. Local CASC extraction succeeded for all six against Retail `12.1.0.69933`, build `dcfc90fffd79ba00406ae46f5f657592`; no CDN acquisition.

| Table | FDID | Layout | Readable rows |
|---|---:|---|---:|
| CooldownSet | 6405756 | AA6C140C | 87 |
| CooldownSetSpell | 6405757 | 0AC7B8F5 | 3044 |
| CooldownSetLinkedSpell | 6422327 | 11E4972F | 1363 |
| AssistedCombat | 6698666 | 5A49E0BC | 40 |
| AssistedCombatStep | 6698668 | 89C93557 | 693 |
| AssistedCombatRule | 6698667 | 43CDA4C0 | 3116 |

No encrypted rows dropped in these six payloads. Export via `scripts/export_db2_csv.py`; behavioral tests check authentic spec/category/order and rule joins. Raw files, definitions and extraction receipt live in canonical `data/diagnostics/cooldown-assist-2026-10-11/` (untracked assets).

## Meaning and boundaries
CooldownSet relates to specialization; CooldownSetSpell defines spell/category/order/PlayerConditionID/flags, not an inference from spell cooldown length. CooldownSetLinkedSpell relates alternate spells to the entry. API category0=Essential,1=Utility,2=TrackedBuff,3=TrackedBar; HideAura1 and HideByDefault2 affect entries.

AssistedCombat relates to specialization; Step defines SpellID and OrderIndex; Rule relates to a step and carries ConditionType and three values. Authored priority therefore **exists locally**: do not invent rotation lists or call the feature server-only. Lua calls `C_AssistedCombat.GetRotationSpells`, `GetActionSpell`, `GetNextCastSpell`; Lua does not implement priority evaluation. SimulationCraft evaluates ordered steps and conjunctions of rule conditions, but is not proof of exact Blizzard comparator behavior.

Keep unidentified DB2 fields under their WoWDBDefs names. Rule field `Field_11_1_7_60520_002` is zero in3114 rows and1 in rows67752/68820 (both condition55); meaning remains unproved. The DBD lists ConditionType and three integer values, without comparator definitions. Blizzard's API documentation defines four native functions and no predicate table. `rotation-source-coverage.json` in the evidence directory records every observed type/count; this is a semantic gap, not missing/encrypted payloads. Condition70 is automation-only in SimulationCraft and excluded from normal assisted rotation. Unsupported condition semantics must remain explicit, never silently treated as true.

## Referenced PlayerConditions
All276 distinct conditions referenced by CooldownSet/Spell exist in the pinned PlayerCondition CSV. Their non-default fields are race/class masks, spell/aura predicates with packed logic, specialization index, and two ModifierTree IDs:160516→455815 (Protection Warrior spell228920),161780→459004 (Fire Mage spell205708). Core now evaluates the masks/spell/aura/specialization subset; ModifierTree and any other unimplemented non-default field return an explicit error. 275 carry Flags1 (ClientExecutable in TrinityCore DBCEnums.h); race-only row104496 carries33, adding WithinOrAboveRecord. ConditionMgr applies that bit only to ContentTuning bounds; this row has ContentTuningID0. Accept those two flag bits while rejecting nonzero unsupported fields, including ContentTuningID. Regression checks both authored race bits15/16 and rejects Human bit0. Predicate sources: ConditionMgr::PlayerConditionLogic and IsPlayerMeetingCondition. The remote short revision a352b1fa was unavailable; comparison used current TrinityCore source and existing local mount logic, not a claim of exact pinned-revision proof.

SpellCategories also supplies the spell→charge category join missing from the spell catalog's max/recharge-only projection: Fire Blast108853 maps to category1500. Viewer charges must use the server's category snapshot, never assume max charges from DB2 are current charges.

## Assisted action and predicate boundary
The real private server's KnownSpellsSnapshot already includes Single-Button Assistant1229376. Local SpellMisc attributes include Attributes10=32768 and Attributes13=65537; SpellEffect is Dummy3 with BasePoints25. Native interception must resolve an authored recommendation and send the ordinary actual-spell intent, not cast that dummy as though it were a damage spell. The native spellbook description states a25% longer global cooldown for the assistant. Current actual-spell interception does not preserve that assistant-cast penalty; execution parity remains incomplete. The description is evidence for the requirement, not proof of how the Dummy effect implements it.

Core now evaluates qualitative condition kinds0/1/2/5/6/9/10/14/15/16/56 using supplied learned/readiness/range/cost/real-aura facts. It also checks ordinary usability on unconditional authored steps. All other numeric/charge/predictive condition kinds remain explicit errors, as do the two unidentified rule-field cases. Native affordability only accepts authored zero-cost rows; effective nonzero costs/modifiers are not replicated and are not guessed. Those errors block potentially eligible higher-priority steps instead of selecting lower spells. This is a bounded mechanism, not all-spec rotation certification.

Cooldown and recommendation inputs now use raw counted-down UnitAuras, not BuffFrame's displayable-aura projection, so hidden gameplay auras remain available to these built-ins.

## User extension
Retail Assisted Highlight targets action buttons. Optional matching Cooldown Viewer icon highlighting is an explicit World of Osso extension; not Retail parity. Forever must re-skin the same behavior, not introduce another UI inventory.

## Status
Export and pure model mechanisms are implemented; [contract](../../specs/cooldown-assist.md) owns acceptance. Core joins and orders authored categories/steps/rules, excludes automation-only steps, and takes explicit predicate evaluators. Unknown predicates block a potentially eligible higher-priority step, but a known false conjunction permits later steps. UI model stores per-spec selection/order and projects cooldown/charge/aura samples, with active aura precedence and independent bar/viewer highlight switches.

Native loading, four viewer roots, ordinary settings actions, server clock/charge-category sampling, raw aura precedence, persisted per-spec choices and shared Edit Mode integration exist. Qualitative recommendation/error capture and actual-spell interception are wired; they are not exact all-spec execution certification.

## Native continuation proof — 2026-10-11
Canonical evidence: `data/diagnostics/cooldown-assist-2026-10-11/`. Private UDP5860 only, account `fb_cooldown_assist`, Coolproof/spec70. Engine merged origin/master in8817e69eb; matching serveraff9b4b1 and protocol6ee323e built through the native helpers.

| Surface | Observable proof | Boundary |
|---|---|---|
| All four categories | run9-forever/run10-modern `viewer-buff-bar.json` and PNG: Essential/Utility rows, Divine Shield92839 tracked icon, Avenging Wrath68661 tracked bar | Genuine server self-casts642/31884; no hostile combat |
| Settings | Real Options/Open/category/select/Up/Close input; Crusader Strike19389 moves before Judgment19397; per-spec choices saved in user data | Fresh-process restoration not exercised |
| Edit Mode | All four mover labels mounted; native drag moves Essential rect(883,720) to(760,640); Escape retains it and writes the account layout (`saved-layout.ron`) | No fresh-process restoration claim |
| Assistant surface | Learned1229376 visible in both native spellbooks; real drag assigns visible action slot6; clicking without a recommendation sends no invented spell | No valid hostile target/assisted cast or rendered highlights proved |
| Core/model/export |10 core +5 UI-model +2 Python export tests; aura precedence/expiry and explicit unknown-rule refusal | Model highlight/charge tests are not live combat/charge proof |

Both private clients fall below loaded terrain, have no selected/server-echoed hostile target, and leave `in_combat=false`. Private revive/teleport did not produce a stable combat sample. Fixture returns2/PARTIAL, not full PASS. Root cause of that movement failure is not established here; no movement fix or shared realm operation was made.

Two native integration failures were fixed: race-only condition104496 legitimately has Flags33; settings `enable_cursor_inputs` diverted onclick actions into the item-cursor queue while the viewer consumed `pop_action`. The flag regression was observed failing then passes; run5 is the native settings RED and runs9/10 prove category/selection/reordering behavior after removing that routing mismatch.

Inspected1280×720 PNGs are published under `/syncthing/AgentShared/2026-10-11/cooldown-assist/`, named by skin and UI surface. They are partial UI proof, not full combat acceptance. [Contract](../../specs/cooldown-assist.md) owns remaining requirements.

## See Also
- [Native HUD Edit Mode](../systems/native-hud-edit-mode.md) — shared mover and account persistence behavior.
- [Spellbook/action bars](../../specs/spellbook-action-bar.md) — actual learned-spell drag/casting surfaces.
