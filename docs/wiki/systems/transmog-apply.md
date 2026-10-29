# Native transmog application

Retail12.1 Transmogrify service: account outfits on the left, detached character in the centre, Items/Sets/Custom Sets on the right. Modern and Forever share elements, layout, actions and authority; Forever changes art only. [Client contract](../../specs/transmog-apply.md); [server contract](../../../../game-server/docs/specs/transmog-apply.md).

## State and rendering

`TransmogBook` stages choices separately from replicated equipment. Paid Apply sends the existing equipped GUID/choice batch and uses exact server quotes. Invalid or unaffordable requests preserve equipment and gold. Stale pending GUIDs are discarded on authoritative refresh; interaction/session closure frees the detached model requests and UI.

The1618×883 frame has312×860 outfit,658×860 character and644×860 collection regions. Appearance slots are59×59, weapon illusion slots42×42, item models100×132 in5×4 pages, and set models178×218 in3×3 pages. These come from `Blizzard_Transmog.xml` and `Blizzard_TransmogTemplates.xml`, not the retired Wardrobe78×104 templates.

Slots and cards do not use the generic wooden project button skin. Save/New Custom Set use authored SharedButton red three-slice members. Main controls use Retail tab, paging, search, undo and checkbox art. Applied/pending item borders use `transmog-itemcard-transmogrified` and its pending suffix with ordinary alpha. Hover uses `transmog-itemcard-default` with ADD, centred at114×146 for items and194×234 for set bases (`Blizzard_TransmogTemplates.xml:1103–1110`). Appearance/illusion slots use their current border with ADD (`Blizzard_TransmogTemplates.lua:676–677,821–822`). Only hover is additive; the old Wardrobe applied-state ADD rule is obsolete.

Each visible card owns its viewport and detached visual request. Item previews use the authoritative offered display/inventory metadata, not world equipment. Set previews dress authored primary appearances, including uncollected pieces; custom previews dress saved choices. Character/set weapons are drawn in the hand. Item weapons are fitted to their actual geometry's blade profile. Authored per-appearance UICamera parity is not certified.

The portrait is static `transmog-icon-ui` (`Blizzard_Transmog.lua:113 SetPortraitAtlasRaw`), not an empty player-bust slot. This raw atlas supplies its own round art; the native FDID-only round-mask pass does not alter it. Set cards have no inline name label: XML1075–1275 defines none; `TransmogSetModelMixin:RefreshTooltip` at1502 owns the title and primary-piece lines.

## Outfits, sets and illusions

The server supplies authentic local-CASC Retail69933 sets, primary memberships/collection states, outfit entries/slots/options and all93 readable illusion identities with SpellItemEnchantment names/icons. Account outfits and Custom Sets persist independently; Custom Set IDs are server-allocated, never DB2 identities. The outfit list uses zero-cost automatically awarded entries; event-condition eligibility is not independently certified.

Outfit row right-click → Edit Outfit opens the525×495 native IconSelector name/icon editor, following `Blizzard_Transmog.lua:199–225,710–783` and SharedUIPanelTemplates.xml:1633. Okay saves selected `icon_fdid` and name without applying or charging. Edit restores the prior selection. The palette uses authentic offered-appearance icons plus IconDataProvider's explicitly selectable index1 question mark134400; it is not the complete global spell/macro icon catalogue. No missing-icon Retail fallback was found: migrated records stay empty until the player chooses an icon. [Versioned server migration contract](../../../../game-server/docs/specs/transmog-apply.md) and [wire contract](../../../../shared-protocol/docs/specs/transmog-catalogue.md).

The handoff's claimed Deadman Blade1181 icon135343 was not the private service's actual data: read-only ItemAppearance query returned135274 for both821 and1181. The native witness deliberately selects135274 and sourced134400 as distinct icons, never substitutes an invented automatic default.

Viewing an outfit or clicking a set/preset stages eligible choices without applying or charging. Save Outfit queues the unchanged paid Apply first, then commits the outfit only after acceptance. Direct server saves change configuration only. Illusion selections are saved and loaded as outfit metadata; this is not proof of visual enchant effects, equipped enchant application or authored illusion-price settlement. Seven authentic default-collected illusions are selectable; conditional unlock grants remain unavailable.

Items lists collected usable choices. Non-collectable Smotts' Cutlass1473 is absent, matching Retail `SetCollectionEntries`. Set primary pieces retain Collected/Uncollected/NonCollectable metadata; tooltips use Retail's collected/light-yellow versus unavailable/disabled colours. Incomplete-set counts use primary appearances per slot, not the number of alternate source records.

## Filters and popup input

Sources carries raw `Enum.TransmogSource` from ItemModifiedAppearance through the wire. Authoritative `Blizzard_APIDocumentationGenerated/TransmogSharedDocumentation.lua` defines acquisition categories1 Boss Drop,2 Quest,3 Vendor,4 World Drop,7 Achievement,8 Profession,10 Trading Post. Special values0 None,5 HiddenUntilCollected,6 CantCollect and9 NotValidForTransmog are not relabelled as acquisition categories. Check All/Uncheck All and source toggles filter before paging while preserving hidden pending choices and quotes.

Sets completion filters use the primary counts. PvE/PvP rows remain disabled: generated API docs do not define a TransmogSet classification flag, and an independent, exhaustive zero-counterexample mapping has not been established. Names alone do not justify a flag assignment. Situations, purchased outfit slots and conditional acquisition APIs are also unavailable, not simulated successes.

Native popup clicks initially hit a collection button underneath despite a higher z_index. Godot GUI hit order follows scene-child order, unlike CanvasItem rendering z. The popup is appended after collection nodes, with explicit popup frame levels. The process fixture clicks World Drop over real `TransmogSource15` and asserts both the staged effective main hand and authoritative applied map remain unchanged. Vendor is above the card grid, so a Vendor-row click alone cannot prove click-through isolation. Raising z_index alone did not fix the failure.

## Retail12.1 native proof — verified2026-10-12

Matched leased client/server/protocol on private UDP5926, owned `fb_transmog_a`/TransmogA, physical Bronze Shortsword2850 GUID123, Gladius821 and Deadman Blade1181. NPC actors require `InMap`; no shared realm or world.db mutations.

| Function | Modern and Forever proof |
|---|---|
| Browse/collection states | Actual skin/layout, collected3D weapon cards570/744 composited pixels; non-collectable1473 absent; native search/no-match/restore. |
| Sources/hover | Real popup hit targets, pending choice remains staged with15-silver quote while filtered out, Uncheck/Check All restores cards; hovered821 retains570 model pixels. |
| Outfits/illusions/paid save | Two distinct source/illusion configurations save through1500-copper Apply each; native load stages the other saved appearance and restores its illusion. Modern40000→38500→37000; Forever37000→35500→34000. A separate Godot process reauthenticates each skin and proves paid main hand1181 and exact final gold persisted, with both saved source/illusion records and selected icons135274/134400. |
| Sets | Battlegear of Might853 reports2/8, native tooltip content logs all eight named Might pieces and distinguishes collected/uncollected pieces; Not Collected toggle hides/restores it; clicking stages2 pieces without charging or changing replicated appearances. |
| Custom Sets | Name popup saves server IDs13/14, held blades contribute2214/1194 pixels; native load replaces a staged original without charging. BOTH skins navigate/capture real pages1 and2 on an account with12/13 presets; new `Z <skin> Blade` presets are onpage2. |

Canonical evidence: `data/diagnostics/transmog-ui-revamp-2026-10-12/followup3-modern.log`, `followup3-forever.log`, both `followup3-<skin>-relog.log` files and corresponding capture directories. Browse runs end `PASS TRANSMOG_WARDROBE_LIVE`; fresh-process relogs end `PASS TRANSMOG_RELOG_LIVE`. Both browse runs verify Escape teardown. 38 inspected1280×720 user PNGs and SHA-256/dimension manifest republished to: `/syncthing/AgentShared/2026-10-12/transmog-ui-revamp/`. The name-popup diagnostic can contain loading cards; ready-card captures provide the model evidence. Logs retain inherited UI FileDataId0 and QuestPOIBlob519073 errors/warnings plus cage capability warnings; they are not error-free runs.

Targeted book/UI/native tests cover pending safety, refusals,20-cell paging, filters, exact card art, membership and save routes. Server targeted transmog tests cover paid authority, save refusals, account isolation and database reopen, including legacy icon migration and unsupported schema refusal. Full protocol suite is a required final gate. Command/revision proof ledger is maintained in the handoff; documentation changes do not invalidate code-scope evidence.

## Native proof — verified 2026-10-11

Historical application proof. Private55087 proved existing self/observer rebuilds, fresh-process relog and private-server restart persistence against the same redb. Modern821 application100000→98500, then1181→97000; Forever821→95500. Evidence: `data/diagnostics/transmog-apply-2026-10-11/`, published PNGs under `/syncthing/AgentShared/2026-10-11/transmog-apply/`. Earlier Wardrobe UI captures prove the old frame only, not the new12.1 layout or current alpha rules.

## Wardrobe UI proof — verified 2026-10-11

Historical78×104 Wardrobe frame: Modern/Forever source821/1181 produced359/463 composited pixels through the old additive state borders, with profile spans62×23/64×20. CPU paging37sources/3pages was not a native second-page census. Evidence: `data/diagnostics/transmog-ui-2026-10-11/modern-corrected-client.log` and `forever-corrected-client.log`. Retail12.1 frame dimensions and ordinary-alpha state borders supersede these old rules.

## Fixture readiness

Wait for item metadata before native bag actions: the prior absent-metadata branch sent Use instead of Equip. `container_item_request` now reports unresolved metadata explicitly. Soft interaction and Z must be configured in isolated fixture homes; otherwise Z toggles Run/Walk. Authenticate owned fixture credentials and never reuse another account's token. The helper builds with `SIBLING_SHARED_PROTOCOL` selecting the matching protocol slot; tracked sibling patches remain intact.

## Sources

- [Book](../../../godot/ui-collections/src/transmog.rs), [wardrobe state](../../../godot/ui-collections/src/transmog/wardrobe.rs), [filters](../../../godot/ui-collections/src/transmog/filters.rs), [frame](../../../godot/ui-collections/src/transmog_frame.rs).
- [Native host](../../../godot/rust/src/transmog.rs), [collection previews](../../../godot/rust/src/transmog/collection.rs), [live fixture](../../../godot/tests/transmog_wardrobe_live.gd).
- Local Retail `Blizzard_Transmog/Blizzard_Transmog.xml`, `Blizzard_TransmogTemplates.xml/.lua`, `Blizzard_Transmog.lua`, and generated `TransmogSharedDocumentation.lua`/`TransmogSetsDocumentation.lua`.
- [Server metadata importer](../../../../game-server/scripts/import_transmog_catalogue.py); pinned TrinityCore a352b1fa CollectionMgr and TransmogrificationHandler sources retained in canonical diagnostics.

## See Also

- [Server transmog](../../../../game-server/docs/wiki/systems/transmog-apply.md) — transactions, ownership and persisted configuration.
- [[offline-asset-closure]] — asset provenance and authored camera/receipt limits.
