# Cooldown Viewer and Assisted Combat authored data
verified: 2026-10-12

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
| ModifierTree | 1267645 | F13BB1D8 | 315726 |

No encrypted rows dropped in these six payloads. Export via `scripts/export_db2_csv.py`; behavioral tests check authentic spec/category/order and rule joins. Raw files, definitions and extraction receipt live in canonical `data/diagnostics/cooldown-assist-2026-10-11/` (untracked assets).

## Meaning and boundaries
CooldownSet relates to specialization; CooldownSetSpell defines spell/category/order/PlayerConditionID/flags, not an inference from spell cooldown length. CooldownSetLinkedSpell relates alternate spells to the entry. API category0=Essential,1=Utility,2=TrackedBuff,3=TrackedBar; HideAura1 and HideByDefault2 affect entries.

AssistedCombat relates to specialization; Step defines SpellID and OrderIndex; Rule relates to a step and carries ConditionType and three values. Authored priority therefore **exists locally**: do not invent rotation lists or call the feature server-only. Lua calls `C_AssistedCombat.GetRotationSpells`, `GetActionSpell`, `GetNextCastSpell`; Lua does not implement priority evaluation. SimulationCraft evaluates ordered steps and conjunctions of rule conditions, but is not proof of exact Blizzard comparator behavior.

Keep unidentified DB2 fields under their WoWDBDefs names. Rule field `Field_11_1_7_60520_002` is zero in3114 rows and1 in rows67752/68820 (both condition55); meaning remains unproved. The DBD lists ConditionType and three integer values, without comparator definitions. Blizzard's API documentation defines four native functions and no predicate table. `rotation-source-coverage.json` in the evidence directory records every observed type/count; this is a semantic gap, not missing/encrypted payloads. Condition70 is automation-only in SimulationCraft and excluded from normal assisted rotation. Unsupported condition semantics must remain explicit, never silently treated as true.

## Referenced PlayerConditions
All276 distinct conditions referenced by CooldownSet/Spell exist in the pinned PlayerCondition CSV. Their non-default fields are race/class masks, spell/aura predicates with packed logic, specialization index, and two ModifierTree IDs:160516→455815 (Protection Warrior spell228920),161780→459004 (Fire Mage spell205708). Core evaluates masks/spell/aura/specialization plus the reached ModifierTree nodes. TrinityCore a352b1fa `CriteriaHandler::ModifierTreeSatisfied`/`ModifierSatisfied` and `DBCEnums.h` decode455815 as known228920 OR aura1235113;459004 as known431044 OR (NOTknown386763 AND NOTknown157997). Other unsupported non-default fields still fail explicitly. 275 carry Flags1 (ClientExecutable in TrinityCore DBCEnums.h); race-only row104496 carries33, adding WithinOrAboveRecord. ConditionMgr applies that bit only to ContentTuning bounds; this row has ContentTuningID0. Accept those two flag bits while rejecting nonzero unsupported fields, including ContentTuningID. Regression checks both authored race bits15/16 and rejects Human bit0. Predicate sources: ConditionMgr::PlayerConditionLogic and IsPlayerMeetingCondition. The original mask audit used current TrinityCore and local mount logic. The continuation successfully retrieved pinned a352b1fa CriteriaHandler/DBCEnums for ModifierTree operators2/3/4/8 and leaf types8/104; receipts are in the continuation evidence.

SpellCategories also supplies the spell→charge category join missing from the spell catalog's max/recharge-only projection: Fire Blast108853 maps to category1500. Viewer charges must use the server's category snapshot, never assume max charges from DB2 are current charges.

## Assisted action and predicate boundary
The real private server's KnownSpellsSnapshot already includes Single-Button Assistant1229376. Local SpellMisc attributes include Attributes10=32768 and Attributes13=65537; SpellEffect is Dummy3 with BasePoints25. Native interception must resolve an authored recommendation and send the ordinary actual-spell intent, not cast that dummy as though it were a damage spell. The native spellbook description states a25% longer global cooldown for the assistant. Current actual-spell interception does not preserve that assistant-cast penalty; execution parity remains incomplete. The description is evidence for the requirement, not proof of how the Dummy effect implements it.

Core now evaluates qualitative condition kinds0/1/2/5/6/9/10/14/15/16/56 using supplied learned/readiness/range/cost/real-aura facts. It also checks ordinary usability on unconditional authored steps. Raw resource21–48/59–62, health7/8/68/69, charges63/64 and cooldown clocks65/66 now evaluate only away from equality, where strict and inclusive interpretations agree. Equality returns an explicit error; mana-percent19/20, distance3/4, aura/nearby/count/stack/duration predicates, predictive67 and nonzero chargeValue2 encodings remain gaps. Owner `SpellCostSnapshot` supplies effective nonzero amounts and affordability from the server's ordinary applicable_costs path, including aura gates, base mana, modifiers and scripts. Those errors block potentially eligible higher-priority steps instead of selecting lower spells. This is a bounded mechanism, not all-spec rotation certification.

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

Both private clients fall below loaded terrain, have no selected/server-echoed hostile target, and leave `in_combat=false`. Private revive/teleport did not produce a stable combat sample. Fixture returns2/PARTIAL, not full PASS. This historical failure was subsequently traced to stale private destination Z in the [terrain investigation](../../../../game-server/docs/wiki/investigations/private-terrain-destinations.md). The continuation uses its build-qualified placement helper before login; no physics workaround or shared realm operation was made.

Two native integration failures were fixed: race-only condition104496 legitimately has Flags33; settings `enable_cursor_inputs` diverted onclick actions into the item-cursor queue while the viewer consumed `pop_action`. The flag regression was observed failing then passes; run5 is the native settings RED and runs9/10 prove category/selection/reordering behavior after removing that routing mismatch.

Inspected1280×720 PNGs are published under `/syncthing/AgentShared/2026-10-11/cooldown-assist/`, named by skin and UI surface. They are partial UI proof, not full combat acceptance. [Contract](../../specs/cooldown-assist.md) owns remaining requirements.

## Native continuation proof — 2026-10-12

Evidence: `data/diagnostics/cooldown-assist3-2026-10-12/`; private UDP5863, fresh `fb_cooldown_assist3`/Coolproof30/spec70. Modern run9 and Forever run12 pass the maintained real-client fixture. Own server CastSuccess is642/31884/20271/35395; the last35395 is the actual assistant-button cast, not a fabricated dummy event or outbound-only claim. Both before-cast PNGs show the matching action button and manager icon; both native AtlasTexture crops advance between sampled frames. Crusader Strike's actual category count changes2→1, with a positive server recharge clock. All four categories, settings/order, account layout write and real assistant spellbook drag remain proved.

Retail highlight source: ActionButtonComponentTemplate.xml, centered66px at45px scale,5columns×6rows,30frames/1second. Local UiTextureAtlasMember32008/atlas3260 selects FDID6992992, sheet2048×1024, member rect845,403,333,400. Both skins use this same authenticated Retail effect, not mouseover art. Hidden surfaces restart; async art arrival is observed before claiming animated pixels.

Core coverage checks exact candidate counts for all40 DB2 specializations and every referenced PlayerCondition. These are authored potential inventories before actor gates, not40 live characters or full rotation certification. Frost64 and Devourer1480 legitimately have no authored tracked bars. All40 have authored rotation rows; unsupported semantics still block eligible steps. No priorities were invented.

The maintained driver creates the host agent scope before applying private XDG runtime variables to its child. It uses short owned UNIX socket paths and the existing owner-checked placement helper. Long server processes must drain their output: CLI `spawn()` captures pipes without draining until wait; the final server used the owned tmux backend with inline private environment and durable copied logs. No lowered ptrace mitigation, package install or protected realm operation occurred.

Sources/limits remain the [contract](../../specs/cooldown-assist.md). Fresh-process layout restoration and assistant-specific25% GCD-origin semantics are not certified. Numeric equality, prediction and remaining encodings are source gaps, not encrypted or absent tables.

## See Also
- [Native HUD Edit Mode](../systems/native-hud-edit-mode.md) — shared mover and account persistence behavior.
- [Spellbook/action bars](../../specs/spellbook-action-bar.md) — actual learned-spell drag/casting surfaces.
