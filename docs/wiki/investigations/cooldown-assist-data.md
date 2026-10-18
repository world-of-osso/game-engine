# Cooldown Viewer and Assisted Combat authored data
verified: 2026-10-17

## Fury distance/nearby-count source boundary — 2026-10-17

[Run21](../systems/bot-harness.md#run21-native-talents-and-predicate-boundary--2026-10-17) applied the matching installed talent loadout and persisted22 entries across a fresh client/server login. That made Whirlwind step330 eligible; native evaluation stopped at rule1261/type3 `[10,0,0]`. The same conjunction contains rule1268/type12 `[3,0,0]`. Neither predicate was replaced or skipped.

Pinned upstream SimC `515f4a3fe9d1f35e1dbaf253745b6105fba906fb` gives the exact matching conjunction in `ActionPriorityLists/assisted_combat/warrior_fury.simc:24`. Source bytes, URLs and SHA-256 receipts are retained under `data/diagnostics/bot-predicate-2026-10-17/<pin>/` (not tracked assets). Line numbers below refer to those retained bytes.

| Operand | Exact inspected source | Proof / unresolved boundary |
|---|---|---|
| Type3 | `engine/player/player.cpp:3840–3844`; `engine/action/action.cpp:4021–4023` | Emits `target.distance<=10`; the expression reads simulated base distance, not a live geometric measurement. Parser3799 still flags comparator precision as unverified. |
| Type12 | `engine/player/player.cpp:3900–3903,7057–7060,7167–7170` | Emits `active_enemies>3` but explicitly omits spatial targeting. The counter follows enemy arise/demise; no nearby-player radius or zero-radius binding is established. |
| Other spatial operands | `engine/action/action.cpp:3673–3703,3808–3886` | `active_enemies_within.N` and `spell_targets` have different spatial/action-list behavior; neither is the emitted operand. Substituting one would invent policy. |

Allowed addon export inspection did not fill that gap: TalentLoadoutManager `d505fc7fdf6f229cacae5d574a17071a7647c16e`, `modules/SimulationCraft.lua:74–89`, appends saved talent strings; SimC addon `8741352777a1d22ec7c89cbd051cd736f6025c6e`, `core.lua:435–455`, serializes purchased entries/ranks. Inspected export modules supply no AssistedCombat radius/default-zero metadata. Their separate pin/hash receipts live beside the SimC sources.

**Still blocked:** exact type12 eligible-set/radius/default-zero semantics, plus a justified live-distance/comparator mapping for type3. The source-to-runtime oracle is incomplete: no honest Boolean RED/GREEN implementation claim follows. Preserve the actual run21 refusal; keep both predicates fail-closed pending sufficient authority, with no global NPC count, alternate provider or lower-step substitution.

## Fire cast-overlap modifier67 — 2026-10-17

Pinned SimC `515f4a3fe9d1f35e1dbaf253745b6105fba906fb` identifies type67 as `AC_COOLDOWN_ALLOW_CASTING_SUCCESS` (`engine/dbc/data_enums.hh:2121`). `engine/player/player.cpp:3703–3710,3752–3753` emits a step option, while4109–4112 emits no Boolean expression. The matching Fireball line is `ActionPriorityLists/assisted_combat/mage_fire.simc:27`. The actual guard is `engine/action/action.cpp:2613–2618`: with the option disabled, same-action current execution or a positive-time last foreground action rejects readiness. This is a cast-overlap modifier, not a sourced numeric prediction/lookahead policy.

Lead approved the bounded projection: type67 with exactly `[0,0,0]` adds no Boolean restriction; any nonzero operand remains an explicit error. Core recognition leaves learned/readiness/affordability/range and harness casting/outstanding-input gates unchanged. No executing-state reading is needed to remove a restriction from a flagged step. The separate default overlap policy for **unflagged** steps remains unimplemented; no global anti-recast behavior was added.

The captured Fire63 frame originally refused rule33340; its newly sourced replay oracle is Fireball133 from the same facts and authored order, with readiness/cost/range refusal controls. Original native JSON/error is preserved, not rewritten into a later live success. Resto/Elemental type11 and Fury3/12 remain fail-closed. At383cee709 the actual Fire replay RED (`Err33340` versus sourced133) becomes GREEN: four focused helper tests pass, including ordinary usability, every nonzero operand position, unchanged Resto/Elemental boundaries and explicit Fury3/12 refusal controls. No compiler warnings; the two pre-existing manual skin cases were filtered. Source bytes/hashes are retained beside the Fury audit; fresh native Fire behavior still needs a matching rebuilt extension and live proof.

## Caster-centered assisted range — 2026-10-16

Native-parity2 Fury72 recorded415 samples with Whirlwind190411 known/ready/affordable but out of range. Active70391 SpellMisc164411 uses Range1, authored "Self Only" with both min/max ranges0; effect279027 uses caster destination18/area-enemy16 and the other effects use self1. Comparing selected-enemy distance against [0,0] excludes the caster-centered cast. TC a352b1fa `Spell.cpp:621–650/7277–7321` normalizes explicit targets and checks the cast target/destination, not arbitrary selection.

The catalog retains raw `SpellMisc.RangeIndex` as `range_id`, and the native assisted fact uses a shared predicate: Self Only row1 is distance-independent; other hostile min/max checks are unchanged. Cache format11 invalidates the old serialized shape. No radius, body-range, priority, training or unknown-rule change. Automated actual-catalog RED at334b795ca fails Whirlwind at3yd while targeted Slam/Charge controls pass. Ata6764af2b the bounded spell_catalog and cooldown_assist suites pass25/0/0, including near/far Whirlwind, self Battle Shout and unchanged targeted bounds. [Contract](../../specs/cooldown-assist.md); source/proof: `data/class-completeness6-2026-10-12/{fury-native-range-observation,caster-range-proof-ledger}.json` in the leased server evidence directory. Fresh native/UDP and independent acceptance remain open.

## Owner pet predicates — 2026-10-16

Fresh master-readiness1 BM253 stopped before casting at rule29672/type58, values[0,0,0], step5716/Call Pet883. Pinned SimC76f8118 `data_enums.hh` names57/58 HAS_PET/HAS_NO_PET; `player.cpp:4067–4072` decodes presence/absence without numeric operands. Native facts retain `has_pet` from owner PetSpells/PetClearSpells plus replicated living-pet health, not a scan for pet-looking NPCs. An absent bar supplies false; a living controlled pet supplies true; a missing/dead pet actor remains unknown. Missing readings and nonzero operands fail explicitly. Tests exercise Boolean opposites, unknown readings and the actual70391 authored BM Call Pet decision. Dead-pet/guardian-only distinctions and fresh UDP acceptance remain separate; this is not full BM or all-spec certification.

Arms rule1187/type12 values[2,10,0] shares the [nearby-count source boundary](#fury-distancenearby-count-source-boundary--2026-10-17). A global active-enemy count does not establish its radius-sensitive semantics.

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

Core now evaluates qualitative condition kinds0/1/2/5/6/9/10/14/15/16/56 using supplied learned/readiness/range/cost/real-aura facts. It also checks ordinary usability on unconditional authored steps. Raw resource21–48/59–62, health7/8/68/69, charges63/64 and cooldown clocks65/66 now evaluate only away from equality, where strict and inclusive interpretations agree. Equality returns an explicit error; mana-percent19/20, distance3/4, aura/nearby/count/stack/duration predicates, nonzero [cast-overlap modifier67](#fire-cast-overlap-modifier67--2026-10-17) operands and nonzero chargeValue2 encodings remain errors; unflagged default overlap policy is a separate parity gap. Owner `SpellCostSnapshot` supplies effective nonzero amounts and affordability from the server's ordinary applicable_costs path, including aura gates, base mana, modifiers and scripts. Those errors block potentially eligible higher-priority steps instead of selecting lower spells. This is a bounded mechanism, not all-spec rotation certification.

Cooldown and recommendation inputs now use raw counted-down UnitAuras, not BuffFrame's displayable-aura projection, so hidden gameplay auras remain available to these built-ins.

## User extension
Retail Assisted Highlight targets action buttons. Optional matching Cooldown Viewer icon highlighting is an explicit World of Osso extension; not Retail parity. Forever must re-skin the same behavior, not introduce another UI inventory.

## Status
Export and pure model mechanisms are implemented; [contract](../../specs/cooldown-assist.md) owns acceptance. Core joins and orders authored categories/steps/rules, excludes automation-only steps, and takes explicit predicate evaluators. Unknown predicates block a potentially eligible higher-priority step, but a known false conjunction permits later steps. UI model stores per-spec selection/order and projects cooldown/charge/aura samples, with active aura precedence and independent bar/viewer highlight switches.

Native loading, four viewer roots, ordinary settings actions, server clock/charge-category sampling, raw aura precedence, persisted per-spec choices and shared Edit Mode integration exist. Qualitative recommendation/error capture and actual-spell interception are wired; they are not exact all-spec execution certification.

## Settings template art — 2026-10-14

The stale audit's399×609 shell,43×55 LargeSideTab controls and38×38/317×38 SettingsItem/BarItem geometry already existed. Remaining defects were the missing `character-panel-background` and white preview fill. Settings XML:153–166 anchors that background to ButtonFrameTemplate's Inset; SharedUIPanelTemplates.xml:684–690 yields(4,60,389,523). The panel now draws that atlas and reuses the shared `InsetFrameTemplate` border. Settings.lua:6,597 defines/applies default preview tint(1,.5,.25); no new known/locked-state inference or reducer behavior added.

Both missing-background and white-fill regressions reproduce RED. Atcc3e0e759,13 focused cooldown UI tests pass; native extension build/check/fmt pass without warnings. `cooldown_settings_art_fidelity.gd` projects concrete offline samples through the production presenter and verifies shell/inset/side tabs/icons/bar bounds and decoded source art/tint. Four1920×1080 Spells/Auras PNGs in both skins and full-size pane details were inspected at `/syncthing/AgentShared/2026-10-14/ui-questlog-settings/settings-source-final/`. This is presentation proof, not new server/persistence/unknown-spell authority acceptance. Later amendment records proof only; Godot source unchanged. Exact revision/commands/hashes: `data/diagnostics/ui-questlog-settings-2026-10-14/settings-proof-ledger.json`.

## Source tracked-buff bar art — 2026-10-14

`cooldown_viewer_component` now gives tracked bars a separate source composition instead of the shared icon renderer's gold rectangle. CooldownViewer.xml:209–277 defines220×30 item,30×30 icon frame(level512),188×19 bar(level511) at icon RIGHT+2/vertical centre, BG(−2,−2)..(+4,+7), source bar tint(1,.5,.25), name inset5/25 and duration RIGHT−8. CooldownViewer.lua:1492–1499 anchors the10×46 atlas pip centre to fill RIGHT with y−1; :1558–1585 shows it only while time remains. Unlike Encounter Timeline's spark, this XML pip does **not** use ADD.

The fill crops texture UVs with supplied remaining fraction rather than stretching the entire gradient into a shorter bar. Existing sample authority, ceil-duration/count strings, assisted highlight, icon rendering, viewer inventory/anchors and controls remain; no new aura clock or recommendation behavior. Icon mask/font-family parity is not part of this bounded bar-art repair. [Contract/native fixture](../../specs/cooldown-assist.md) owns acceptance and separates offline full/half/empty samples from prior private-combat proof.

### Lead review corrections

Lead rejected the initial proof pending Applications anchor and current-data gate repair. XML:237–241 requires a32×10 Applications fontstring at icon BOTTOMRIGHT(−5,+5); the previous30×12 corner placement clipped the count. Corrected without changing aura/count authority. Two core regressions still expected older CSV values after the active publication changed: entry280 now links102416/45334/50259, and exact Mage/Hunter/Monk category counts follow current source rows. No build string is selected or pinned by the tests.

Full/half pip intersections are source behavior, accepted by lead on2026-10-14: Lua:1498 centres pip on status-fill RIGHT at(0,−1); XML:271–279 places pip/name/duration in the same OVERLAY. BG rect ends4px beyond the bar, although its atlas has transparent shadow padding and a painted rim inside the logical edge. No invented painted-rim inset, reduced fill width or pip layer change added. Native source atlas exports accompany recaptures.

### Review recapture proof

At ef4372c64 (engine master347aa3dc0 integrated, protocol sibling0eed96f), exact prior gate failures and the Applications regression reproduce RED; all15 core cooldown/assist and7 UI-model tests then pass. Build/check/fmt pass without warnings. Both skins pass native full/half/empty checks and Applications glyph containment; full1920×1080 captures and enlarged details inspected at `/syncthing/AgentShared/2026-10-14/ui-questlog-bars/cooldown-review-final/`. Logical Applications32×10 anchors right5/bottom5; Godot uses15px font height with glyphs contained in the30px icon. Source BG/fill/pip atlas exports preserve their actual132×19/124×10/10×46 art and source overlap evidence. No invented rim clipping added.

The proof ledger supersedes the rejected a735af829 candidate and retains its source-gate failures. Review fixture/code changes manually audited for Rust readability. Only proof documentation changes after this verified binary; no additional gameplay or scheduling behavior.

### Initial bar-art proof (superseded by review recapture)

Seven focused `cooldown_assist` tests, native extension build/check and formatter passed at7196fe250 after integrating engine master58f2d5c6c and protocol master0ed9eb0 (sibling mergec4e6484). Missing-background and LEFT-name regressions are preserved RED→GREEN. Both skins passed native full/half/empty geometry; both1920×1080 PNGs and enlarged bar details were inspected against XML/Lua at `/syncthing/AgentShared/2026-10-14/ui-questlog-bars/cooldown-pixel-final/`. The source orange fill remains cropped at94/188px, the normal-alpha pip follows its edge, and expired fill/pip hide. Duration glyph widths9/17px end exactly8px inside the bar; name remains LEFT+5.

Logical y5.5 snaps to physical y6; only the native oracle allows0.51px. An initial duration oracle incorrectly compared against Godot clipped-label minimum width1px; actual glyph measurement confirms correct natural widths. These fixture failures are not product defects. Exact commands/revisions/hashes and rejected evidence: `data/diagnostics/ui-questlog-bars-2026-10-14/bar-proof-ledger.json`. Later proof-documentation amendment changes no Godot source. Manual changed-Rust readability audit found no in-scope violation. Offline presentation proof only: no new live aura clocks, scheduling, server transport, icon-mask or font-family parity claim. Native exited; no private server or new assets required.

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

Evidence: `data/diagnostics/cooldown-assist3-2026-10-12/`; private UDP5863, fresh `fb_cooldown_assist3`/Coolproof30/spec70. Post-merge `finish-modern` and `finish-forever` pass the maintained real-client fixture. Own server CastSuccess is642/31884/20271/35395; the last35395 is the actual assistant-button cast, not a fabricated dummy event or outbound-only claim. Both before-cast PNGs show the matching action button and manager icon; both native AtlasTexture crops advance between sampled frames. Crusader Strike's actual category count changes2→1, with a positive server recharge clock. All four categories, settings/order, account layout write and real assistant spellbook drag remain proved.

Retail highlight source: ActionButtonComponentTemplate.xml, centered66px at45px scale,5columns×6rows,30frames/1second. Local UiTextureAtlasMember32008/atlas3260 selects FDID6992992, sheet2048×1024, member rect845,403,333,400. Both skins use this same authenticated Retail effect, not mouseover art. Hidden surfaces restart; async art arrival is observed before claiming animated pixels.

Core coverage independently compares exact candidate identities/spells/conditions/flags/order and counts for all40 DB2 specializations and every referenced PlayerCondition. These are authored potential inventories before actor gates, not40 live characters or full rotation certification. Frost64 and Devourer1480 legitimately have no authored tracked bars. All40 have authored rotation rows; unsupported semantics still block eligible steps. No priorities were invented.

The maintained driver creates the host agent scope before applying private XDG runtime variables to its child. It uses short owned UNIX socket paths and the existing owner-checked placement helper. Long server processes must drain their output: CLI `spawn()` captures pipes without draining until wait; the final server used the owned tmux backend with inline private environment and durable copied logs. No lowered ptrace mitigation, package install or protected realm operation occurred.

Sources/limits remain the [contract](../../specs/cooldown-assist.md). Both `restore2-modern` and `restore2-forever` independently restart the client with saved data, proving selected tracked rows, per-spec ordering and account Edit Mode position. Inactive auras remain hidden normally; saved selection is checked through actual settings rows. Assistant-specific25% GCD-origin semantics are not certified. Numeric equality, prediction and remaining encodings are source gaps, not encrypted or absent tables.

## Source-gap re-audit — 2026-10-12

Receipts: `data/diagnostics/cooldown-assist4-2026-10-12/assistant-db2-audit.json`, `remaining-predicate-audit.json`, `tc-audit.json`, and the pinned raw single-file copies. These are searches of the named local tables/code paths, not a claim that every TrinityCore file was audited. No new comparator, priority or GCD policy follows from an unidentified field.

### `ASSISTANT_GCD_ORIGIN_1229376`

The description explicitly says the global cooldown is `$s1%` longer. Exact checked local rows:

| Table | Spell1229376 | Related named aura470418 (`Assisted Highlighting Elemental`) |
|---|---|---|
| SpellCooldowns (36334 rows) | No SpellID row or reverse AuraSpellID reference | No row/reference |
| SpellCategories (93212 rows) / SpellCategory | No category/start-recovery-category join; therefore no category row to decode | No category join |
| SpellEffect (629424 rows) | ID1220649: Difficulty0, Index0, Dummy3, BasePoints25, EffectAura0, TriggerSpell0, MiscValues0/0, target25 | ID1185618: ApplyAura6, Aura226, period1000, BasePoints0, TriggerSpell0, MiscValues0/0, target1 |
| SpellAuraOptions (32164 rows) | No SpellID row | ID239368: Difficulty0, cumulative0, proc category recovery0, chance101, charges0, PPM0, masks87312/0 |
| SpellPower (5715 rows) | No cost row or RequiredAuraSpellID reverse reference | No row/reference |

SpellMisc783909 also has Attributes10=32768 and Attributes13=65537. Pinned TC `SharedDefines.h` calls the relevant bits Attr10 `UNK15` and Attr13 `ALLOW_CLASS_ABILITY_PROCS`/`UNK16`; these are not an assistant-origin/GCD implementation. Name/description scans found no other Single-Button Assistant aura chain. Neither checked SpellEffect triggers another spell.

Pinned TC a352b1fa `Spell.cpp::TriggerGlobalCooldown` (downloaded lines9179–9228) uses the actual spell's StartRecoveryTime/category, ordinary StartCooldown modifiers and haste. `SpellHistory.cpp` stores GCD by category. Checked `SpellEffects.cpp`, `SpellHandler.cpp::HandleCastSpellOpcode`, `spell_generic.cpp`, `DB2Structure.h` and `SpellPackets.h` provide no decoded assistant25% origin handler. The generic request's SendCastFlags/Misc fields are not evidence for a particular assistant encoding. The Dummy25 value establishes the tooltip number, not where to apply/clamp haste or how to preserve cast origin. **Named source gap retained; no guessed multiplier or protocol flag added.**

### Exact remaining rule boundaries

The local3116 rules contain no comparator/operator field defining strictness or inclusive equality. The Retail UI cache calls `C_AssistedCombat.GetNextCastSpell`; its Lua and generated API documentation expose no predicate evaluator. Checked pinned TC definitions/cast paths do not decode this table. Existing source-limited numeric behavior remains unchanged.

- Equality remains unresolved. Concrete private Ret boundary: rule6425, step1655, type38, values3/0/0; Holy Power3 returns the explicit equality error rather than an invented comparison.
- Mana-percent type20 has2 authored rows; range/distance3/4, aura/count/stack/duration/nearby11/12/13/17/18/49–55/57/58 and type67 (60 rows) were originally unresolved; the [later SimC audit and bounded projection](#fire-cast-overlap-modifier67--2026-10-17) recognize zero-operand67 as a cast-overlap modifier, not prediction. Nonzero operands and unflagged default overlap policy remain gaps. The receipt records counts and exact source rows, not inferred units or comparators.
- Charge predicates:10 authored type63 rows, **zero nonzero ConditionValue2 rows**, no type64 rows. Nonzero chargeValue2 is a reserved unsupported encoding, not an implementable gap in this pinned authored inventory.
- Rule67752: step15297, order1, type55, values1254252/2/0, unidentified field1. Rule68820: step1223, order4, type55, values1254252/1/0, unidentified field1. No approved source defines that field; both remain explicit errors when potentially eligible.

Art re-authentication: atlas member32008 → element30221 `RotationHelper_Ants_Flipbook`, atlas3260/FDID6992992, crop845/403/333/400 on2048×1024. Local BLP SHA256 `3fa9b0678f4b7a7ce9835981b12315706984eec8820a122e75a49b1c13cdc1c8` matches the already-extracted canonical asset. No replacement art or new extraction was needed.

### Same-decision recommendation replay

The native observer retains the facts and canonical selection from one actual evaluator invocation, before effective-spell presentation mapping. Its `evaluation_json` contains specialization, target, raw powers, health percentages, cooldowns/charges, aura sets, known/readiness/range/authoritative affordability and the exact selected step/error. Snapshot serialization is on demand; no per-frame JSON encoding or second gameplay decision is introduced. The [contract](../../specs/cooldown-assist.md) names the recorded-private replay tests. Casting and predicates remain unchanged.

## Private continuation proof — 2026-10-12, cooldown-assist4

Fresh private UDP5864, `fb_cooldown_assist4_modern`/Coolmodern30 and `fb_cooldown_assist4_forever`/Coolforever31, both level10/spec70. Engine includes origin/master62ceaf544, server3f80f41a, protocol80bec38. The maintained fixture passes in both skins on native builds; no shared realm operations or physics changes.

| Claim | Both-skin observable proof | Boundary |
|---|---|---|
| Combat/assistant | Pre-cast `in_combat=true`, real selected/server-echoed Blackrock enemy, matching advancing ants, exact own CastSuccess35395 | Forever's victim dies on that cast; combat can end afterward |
| Charges | Actual Crusader Strike2→1, positive recharge5.40888s Modern/5.26543s Forever | At the later recommendation sample the charge has naturally recovered to2; do not assert stale1 against newer facts |
| Recommendation update | Recorded35395→20271; untouched native evaluation JSON replays to exact first eligible20271 in both skins | Captured Holy Power2, real cooldown/charge/cost/target facts; not mere rotation membership |
| Replay control | Disabling that selected spell's readiness changes the evaluator result | No invented numeric equality or priority changes |

Ten inspected1280×720 PNGs are in `/syncthing/AgentShared/2026-10-12/cooldown-assist4/`; logs, JSON and private redbs remain in `data/diagnostics/cooldown-assist4-2026-10-12/`. Same-decision JSON must be replayed from the preserved `evaluation_json` string: Godot's parsed/display projection converts integer cells to floats and is not the typed native payload.

The harness now waits for **both** own setup CastSuccess and replicated combat entry before cycling targets; CastSuccess can precede combat damage/replication. Moving enemies behind authentic occluders require another ordinary Tab candidate, not removing LoS or changing NPC data. No assistant predicate or casting behavior was changed.

## Assigned bot rotation blockers — 2026-10-15

Verified: 2026-10-15. Exact rows below match both published `12.1.5.70391` and current `data/db2/active` across AssistedCombatRule, Step and AssistedCombat (nine joined rows). Rule order is not a replacement policy: all40 specs already have authored steps.

| Named source gap | Exact authored identity | Missing approved semantics / proof level |
|---|---|---|
| `ASSIST_RULE_12408_CONDITION20_SOURCE` | Arcane62; rotation209, step2999/order14, spell44425; rule12408/order3, type20, values100/0/0 | Checked TC/Retail sources do not define units, normalization or comparator. Third-party mana-percent naming is not Retail proof. Native bot receipt reports this explicit refusal. |
| `ASSIST_RULE_6425_EQUALITY_INCLUSIVITY` | Retribution70; rotation117, step1655/order15, spell20271; rule6425/order3, type38, values3/0/0 | Holy Power3 is the unresolved equality boundary. Strict/inclusive outcomes differ; preserve the existing explicit error. Native default-kit receipt fails here; a different talent/loadout avoiding the boundary does not fix this rule. |
| `ASSIST_RULE_33686_CONDITION67_POLICY` | Frost64; rotation85, step1165/order11, spell116; rule33686/order3, type67, values0/0/0 | The [later allowed SimC audit](#fire-cast-overlap-modifier67--2026-10-17) identifies a cast-overlap step option, not predictive inputs/lookahead. The later zero-operand projection removes that Boolean refusal without adding default unflagged overlap policy. Earlier native refusal remains historical evidence, not proof of current rebuilt-native behavior. |

Rechecked the pinned a352b1fa SpellHandler.cpp, DB2Structure.h and spell_generic.cpp copies against their recorded SHA256 hashes; none contains an AssistedCombat decoder. This is a bounded search of those files, not an exhaustive TrinityCore audit. Retail `Blizzard_APIDocumentationGenerated/AssistedCombatDocumentation.lua` exposes `GetNextCastSpell` with empty `Tables`/`Predicates`; `Blizzard_ActionBar/Mainline/AssistedCombatManager.lua:329` delegates to `C_AssistedCombat.GetNextCastSpell(checkForVisibleButton)`. Neither checked Lua file defines these numeric predicates. No evaluator/gameplay/stat/protocol change follows from the DB2 fields alone.

Exact source hashes, joined rows and named gaps: `/home/osso/.worktrees/game-server-s6/data/class-completeness6-2026-10-12/rotation-assigned-source-gaps.json`. Native Arcane/Ret counterexamples: `/home/osso/.worktrees/bot-harness-2026-10-15/mvp-dummy2/failures.json`. The bot harness's separately reported Elemental38530/type17 blocker is outside this three-rule assignment. These refusals must not be hidden by reordering, a generic no-order path or an invented lower-priority fallback; source-supported samples do not certify complete rotations.

## Sources
- [Exact assistant DB2 receipt](../../../data/diagnostics/cooldown-assist4-2026-10-12/assistant-db2-audit.json) — named rows, reverse references and table counts.
- [Exact unresolved-rule receipt](../../../data/diagnostics/cooldown-assist4-2026-10-12/remaining-predicate-audit.json) —333 rows in wholly undecoded predicate groups, numeric boundary6425, counts, special fields and charge encoding census.
- [Pinned TC receipt](../../../data/diagnostics/cooldown-assist4-2026-10-12/tc-audit.json) and [Spell.cpp](../../../data/diagnostics/cooldown-assist4-2026-10-12/Spell.cpp) — checked cast/GCD code, not an assumed assistant policy.
- [Private replay test](../../../godot/core/tests/cooldown_assist_live_replay.rs) and [contract](../../specs/cooldown-assist.md) — exact native snapshot/evaluator acceptance and retained gaps.

## See Also
- [Native HUD Edit Mode](../systems/native-hud-edit-mode.md) — shared mover and account persistence behavior.
- [Spellbook/action bars](../../specs/spellbook-action-bar.md) — actual learned-spell drag/casting surfaces.


## ASSISTED_COMBAT_SIMC_MIRROR

Verified: 2026-10-17. Explicit user-approved project policy, **not verified Retail predicate semantics**. Scope and exact behavior live in the [spec](../../specs/cooldown-assist.md#user-approved-spatial-policy); only Fury Whirlwind190411 rules1261/1268 are enabled. Other distance/count rows continue refusing, so full Fury completion is not implied.

Source-only laptop packet `/syncthing/AgentShared/2026-10-17/laptop-requests/assisted-combat-types-3-12-ANSWER.md`, SHA256 `eab706331035349dca755d95d6d29b217bcfee78eca75df5565a7e1bcbe22b09`, reports no exact native authority. SimC1860cd1d5763db68095823d4cb15731987652bd8 `engine/player/player.cpp:3578–3583,3635–3642` supplies the approved comparator mirror only. User separately chose centre-position geometry, inclusive8yd and alive+hostile+local-player-threat membership. Build70448 Whirlwind SpellEffect279027 references SpellRadius14, Radius=RadiusMax=8 and RadiusPerLevel=0. Fixture provenance records actual source-table hashes.

`godot/core/tests/assisted_simc_mirror.rs` retains original run21 native failure JSON unchanged and injects explicitly synthetic new distance/count inputs for ordered replay. Native replica tests cover owner versus party-only threat, death/friendliness, horizontal/vertical radius boundaries and unavailable positions. Model/replica proof does not establish a fresh native cast, boss engagement or clear.


### Native policy observation — run22

Verified: 2026-10-17. Private UDP5871 matched engine2948bb1ce, release/system-timings server61849c270 and own detached protocole9b80b8. Ordinary exit plus fresh credential login retained all22 received Fury ranks without another Apply. Native authored combat passed the prior1261/1268 refusal and recorded one owner Whirlwind190411 CastSuccess. This is **not exact step330 attribution**: lower authored step18147 offers the same spell. Other type3/12 rules remain unsupported; unrelated type4 errors appeared in nonterminal snapshots. No new Retail-semantic authority is inferred.

Run ended451.121s after entry at Captain Garrick's death:0player/1party death, complete death capture, all three bosses NotStarted. No boss DPS or clear proof. Final20s retained11497 tank damage versus5083 Regrowth healing with no overflow; healer casts and owner successful casts continued. This does not establish a healer stall or authorize kit/stat tuning. Release tick windows degraded (minimum5.2, median15.3ticks/s); shared tick-performance investigation owns attribution. Owned processes exited and exact window grant was cleared.

Receipts: `/home/osso/.worktrees/bot-harness-2026-10-15/stockade22-native-proof.json` binds source revisions, artifact hashes and cleanup; sibling `stockade22-native-policy-observations.json`, `stockade22-tank-death-window-analysis.json` and `native-stockade-fury22-trace-class-spec-proof.json` retain native decisions, cast/death accounting and timing. Passive FPS report retains missing samples and build/device/load qualifications; early readings are not a steady-performance benchmark. Full Stockade acceptance remains open in the [bot spec](../../specs/bot-harness.md).
