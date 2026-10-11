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

Keep unidentified DB2 fields under their WoWDBDefs names. Rule field `Field_11_1_7_60520_002` is zero in3114 rows and1 in2 rows; meaning remains unproved. Condition70 is automation-only in SimulationCraft and excluded from normal assisted rotation. Unsupported condition semantics must remain explicit, never silently treated as true.

## User extension
Retail Assisted Highlight targets action buttons. Optional matching Cooldown Viewer icon highlighting is an explicit World of Osso extension; not Retail parity. Forever must re-skin the same behavior, not introduce another UI inventory.

## Status
Export and pure model mechanisms are implemented; [contract](../../specs/cooldown-assist.md) owns acceptance. Core joins and orders authored categories/steps/rules, excludes automation-only steps, and takes explicit predicate evaluators. Unknown predicates block a potentially eligible higher-priority step, but a known false conjunction permits later steps. UI model stores per-spec selection/order and projects cooldown/charge/aura samples, with active aura precedence and independent bar/viewer highlight switches.

Proof: two local WDC5 export tests, four authentic-catalog core tests and two UI-model projection tests pass. Test compile failures demonstrated the missing core/model APIs before implementation; these were compile REDs, not runtime REDs. No concrete rotation predicate evaluator, native mounted viewer/settings/Edit Mode, live-state adapter, single-button execution or rendered highlights exist yet. Native both-skin/private combat proof remains outstanding.
