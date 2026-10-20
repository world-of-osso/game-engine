# Native professions crafting

The native client extends the existing ProfessionBook/ProfessionsFrame, trainer and server spell pipeline. [Modern crafting](../../specs/modern-crafting.md) owns acceptance, profession choice and data gaps; [ProfessionsFrame](../../specs/professions-frame.md) owns the shared Retail presentation contract. Forever changes art, not recipe/craft rules.

Verified: 2026-10-11. Engine behavioral source31c2bba12, overlay geometryac11dd7d5, formatter orderd22ab6878; serverd76b0e3; matching leased protocolb4523221e28c84d845066a532fafb8f838d4bf69. No master merge, canonical code edits, world.db writes or UDP5000 use. Extracted CSV/DB2/texture assets persist in canonical data, outside tracked code.

## Recipe cooldown

`RecipeCatalog.cooldown_categories` maps recipe spells to their nonzero base `SpellCategories.Category`. `professions.rs::selected_recipe_cooldown` reads `account.spells` (fed by `SpellCooldownUpdate`, including the server's world-entry replay) and `professions_cooldown::remaining_secs` takes the longer clock in whole seconds; `ProfessionView.recipe_cooldown_secs` disables Create/Create All/quantity and renders `ProfessionsRecipeCooldown`. [Contract](../../specs/modern-crafting.md#recipe-cooldown). Native both-skin proof is pending.

## Catalog and rendering

`godot/ui-model/src/professions_catalog.rs` loads the active Retail CSV build; earlier proof below retains its original pinned inputs. Category order is the signed `TradeSkillCategory.OrderIndex` (the source includes−1), not alphabetical category names. Known recipes remain name-sorted within each category.

`professions_quality.rs` joins `Item.CraftingQualityID` → `CraftingQuality` → `CraftingQualityAtlasSet.IconInv` → `UiTextureAtlasElement.Name`. The stored tier is distinct from item rarity. `professions.rs` in the native extension supplies quality records only for displayed items; reagent slots draw the inventory overlay at its authored atlas size. Local-CASC export/layout/source receipts and absent outcome formulas are recorded in the [contract](../../specs/modern-crafting.md#quality-source-receipt), not inferred from item IDs.

## Server-recorded crafting origin

[Origin contract](../../specs/modern-crafting.md#server-recorded-crafting-origin): shared ItemStack carries optional `CraftingOrigin` containing SkillLineAbilityID, its joined recipe SpellID and an optional actual known crafting quality ID. TC modifier39 denotes the ability row, not the spell: real2153 maps to ability1355. Source absence or ambiguity remains unknown; class item-creation abilities are not profession recipes. Existing operation/refusal behavior remains unchanged, with no slots/watermark writes, fabricated quality or recraft activation.

The server retains the authored ability row ID, passes origin from real CREATE_ITEM completion into inventory storage and persists it in an additive GUID-keyed table in the inventory transaction. Existing ItemInstanceData/character encodings are not extended. Stack compatibility additionally checks origin; unique-equipped template identity remains separate. Unknown-origin stacks cannot be relabeled by merging craft output, same-origin outputs retain their surviving GUID, and split GUIDs copy origin. Owner item projections (including bag/equipment state mirrored by the engine) carry authoritative metadata; merchant-state observation exposes GUID/origin, not a new player-facing capability.

### Targeted provenance proof

All three ignored REDs were pushed before implementation: protocol drops origin, actual2153 cast plus fresh DB/owner snapshot lacks origin, and engine same-GUID snapshot loses origin. First engine fixture lacked data-root initialization and emitted a secondary catalog panic; its corrected stable-catalog RED is retained separately. A further actual2963×2 RED exposed merging into an unknown2996 stack (count3 instead of1).

Targeted GREEN: protocol `fcff8da`1 JSON/binary case; server `5ea22bf55`2 real-cast cases; engine `4b43d476a`1 actual owner-snapshot projection case. Server proof includes2153→new2303GUID, ability1355/recipe2153 after actual database close/reopen, identical non-crafted item unknown, and2963→ability2359 with compatible stacking, rejected unknown-origin merge and split persistence. Quality remains unknown; no operation result is invented. Post-test master merges are separate revisions; lead owns current atomic integration/check/fmt/readability gates. No new rendered native or real OS-process reopen proof is claimed by these CPU tests.

SSOT: canonical `data/diagnostics/crafting-provenance-2026-10-19/` contains source refs, exact RED/GREEN command ledger, raw logs and fixture qualifications. Read-only source matrix: `data/diagnostics/crafting-lineage-readonly-2026-10-19/evidence-matrix.md` with TC/70448 schema hashes. [Server origin contract](../../../../game-server/docs/specs/crafting-origin.md) owns persistence requirements; [protocol contract](../../../../shared-protocol/docs/specs/crafting-origin.md) owns wire semantics. Protocol and consumers must land together; this is causal provenance, not a complete Retail recraft eligibility predicate.

### Native provenance proof — 2026-10-20

Landed P`5660814`/S`9683d56e8`/E`e95e6c31e` (cp749) plus fixture-only commits. Both skins at 1920×1080 on an isolated, prerequisite-only redb run `godot/tests/crafting_provenance_live.gd`: real trainer264578, fourteen2881 casts, trainer2153 and one Create. The new 2303 GUIDs (Modern 7475, Forever 7984) carry {1355, 2153, null}; the generic 2303 controls stay unknown; leather 14→10, thread 1→0, rank 15→16. After stopping that server, a fresh server generation reopens the same database and fresh readback-only clients observe the same GUIDs/origin, 940 copper and rank 16 without casting. Hovered crafted and control slots show the same normal item tooltip; origin is the `merchant_state` owner readback, not tooltip text. A post-stop DB assert agrees.

NPC opening reuses the accepted IPC `quest interact` setup (`trading_post_live.gd`); trainer purchase, casts, Create and hovers are native input. The base-profession confirmation is `TrainerConfirmAccept`. `modern_crafting_unlearned_live.gd` remains natively broken (fixture right-click, `StaticPopup1Button1`, a stall in its disabled-click section) — named gap. SSOT: `data/diagnostics/crafting-provenance-2026-10-19/native/native-provenance-proof.json`; twelve inspected PNGs under `/syncthing/AgentShared/2026-10-20/crafting-provenance/`.

The run also recorded origin on the crafted Light Leather stack (2881). Policy `CRAFTING_ORIGIN_NOT_RECORDED_ON_STACKABLE_ITEMS` (origin only on max-stack-1 items) is the follow-up; see the [server contract](../../../../game-server/docs/specs/crafting-origin.md).

## Passive operation status

The [passive diagnostic contract](../../specs/modern-crafting.md#passive-operation-diagnostics) removes the typed missing-operation diagnostic from player-facing CraftStatus without inventing operation results or hiding Details. Native `profession_view` uses pure `professions_reagent_selection::craft_status`; pending validation and concrete validation/action errors retain priority. Existing native `godot_print` logs the named gap at the selected-recipe transition, using existing reagent-selection identity to avoid repeated per-frame logs. Internal typed response/refusals remain unchanged.

Retail cache investigation: `RecipeSchematicForm.lua:273–302,1370–1443` skips operation requests/stats when metadata/results are absent; `RecipeCrafterDetails.lua:288–310,380–419` independently gates quality/concentration controls. No-quality/no-concentration does not imply hiding the whole Details pane. Engine fixed-output `Option<(item,count)>` is not Retail supportsQualities/supportsCraftingStats metadata; those capabilities were not inferred. Hash-pinned read-only evidence: canonical `data/diagnostics/profession-pane-gap-readonly-2026-10-19/evidence-matrix.md` and its source receipt.

### Native passive-status proof — 2026-10-19

Pushed ignored RED `00231cbb7` exposes the raw named gap in the actual screen; compatible protocol `724ec71` observes exit101 before GREEN `0d7294fc3`. An earlier `b3a34fe` compile mismatch lacks the newly landed guild leadership variant and is archived separately, not counted as behavioral RED. Both-skin/five-state screen regression passes; ps713 tests/fmt and ps713b built-extension parse pass. Initial cold gate parse failed only on missing extension registration, retained as harness evidence.

Exact rebuilt cohort engine `2894ce9e3`, server `cf1fc7e36`, protocol `724ec71`: both native1920×1080 clients show blank passive status after120frames and after authoritative allocation validation; concrete missing-slot error remains visible. One raw named-gap diagnostic for375188 is logged per client across refreshes/dialogs/responses. Quality remains UNASSIGNED; no output/Create permission is fabricated, Create remains disabled, and bags/money/skill/no-cast conservation passes. Both clients exit0 with one completion marker and no error lines; known geoset/MSAA/QuestPOI warnings and separate editor-import errors remain qualified.

SSOT: canonical `data/diagnostics/profession-passive-operation-status-2026-10-19/native-passive-status-proof.json` binds exact binary/source identities, diagnostic counts, twelve inspected PNG hashes/resolutions and cleanup. Originals: `/syncthing/AgentShared/2026-10-19/profession-passive-operation-status/{Modern,Forever}-{1..6}-*.png`. Owned server3284449 stops;5951 unbound, crafting slice inactive and window cleared. This is presentation/logging proof only; operation composition, concentration origin and successful recraft mechanics remain missing.

## Recipe search magnifier

The [magnifier contract](../../specs/modern-crafting.md#recipe-search-magnifier) uses existing `ProfessionView.search_focused` and query state. The profession tree authors10×10 `common-search-magnifyingglass` at LEFT(+1,−1), tinting RGB0.6 only when empty and unfocused, otherwise1. No shared search template or query behavior changes. Source: cached Mainline `InputBoxTemplates.xml:211–217`, `InputBoxTemplates.lua:175–200`.

Ignored RED `5e5d57d80` failed at actual screen dimensions12×12 versus10×10 before implementation `89539cb49`; targeted GREEN passes the magnifier's both-skin/four-state assertions and two existing Clear controls. Parent pm700 gates/lands `00afe30a7`, including native fixture `c1a2b7dc6` and direct parse.

### Native magnifier proof — 2026-10-19

Exact binary cohort engine `00afe30a7`, server `46d5aba82`, protocol `b3a34fe`: both1920×1080 native skins pass four focus/query states and Clear restoring dim. Icon bounds `(43,180,10,10)` relative to search `(42,174,162,20)` match source anchors; native TextureRect RGB changes0.6→1. Painted icon pixels brighten on focus and restore byte-identically after Clear. Selection, original Favorites presence, hidden-unlearned flag and bags/copper/skill remain unchanged; default filters are restored and no cast starts. Both clients exit0 with one completion marker and no error lines. Known geoset/MSAA/QuestPOI warnings remain; unrelated one-time editor-import thread/dialog errors are archived separately, not relabeled clean.

Evidence SSOT: canonical `data/diagnostics/profession-search-magnifier-2026-10-19/native-magnifier-proof.json` binds binary hashes, exact source cohort, native state logs, ten inspected PNG hashes/resolutions and cleanup. PNG originals: `/syncthing/AgentShared/2026-10-19/profession-search-magnifier/{Modern,Forever}-{1..5}-*.png`. Owned server1410830 stops,5951 is unbound, crafting slice inactive and window cleared. This proves magnifier presentation/conservation only, not missing operation composition, concentration origin or successful recraft mechanics.

## Recipe search clear control

The [clear contract](../../specs/modern-crafting.md#recipe-search-clear-control) adds the missing profession-specific SearchBoxTemplate control without replacing other search screens. `ProfessionView.search_focused` reads actual native registry focus; the screen shows the authored clear control for focus or nonempty query. The existing `search_for_clear_action`/edit/blur path consumes its action locally, clears registry text and releases native focus. The ordinary profession poll observes the new text; selection, filters and authoritative Favorites are not reset.

Ignored both-skin RED was pushed and failed on the missing clear control before implementation. Two screen tests and one production registry click→clear/blur test pass; pc694 gates and lands the implementation.

### Native search-clear proof — 2026-10-17

Engine binary `c62691ee2`, server `40252e6f1`, protocol `b3a34fe`: both native skins at1920×1080 pass all four focus/query states and two coordinate Clear clicks each (empty-focused and nonempty-unfocused). Clear empties the query, releases search focus and hides itself; selected recipe/favorite instance, owner Favorites and hidden-unlearned filter survive. Final cleanup restores the original unfavorited preference and default visibility; bags/copper/skill remain unchanged and no cast starts. Bounds are Clear `(184,176,17,17)` against search `(42,174,162,20)`, matching source RIGHT−3/vertical centering within native pixel rounding.

Fixture-only `991c1372d` corrects a failed stimulus assumption: clicking Filter does not take native edit focus, so the positive focus-transfer case clicks the existing quantity EditBox. Modern explicitly resumes its already-authoritative favorite instead of repeating the request; Forever prepares its favorite normally. No production code changed after the gate. Both drained clients exit0 with exactly one completion marker, seven/eight unique captures respectively and zero `ERROR`/`SCRIPT ERROR` lines; existing geoset/QuestPOI/MSAA warnings remain.

Evidence SSOT: canonical `data/diagnostics/profession-search-clear-2026-10-17/native-search-clear-proof.json` binds binary/fixture identities, fifteen current PNG hashes, explicit resume, preserved failed/interrupted attempts and cleanup. Twelve inspected state originals: `/syncthing/AgentShared/2026-10-17/profession-search-clear/{Modern,Forever}-{1..6}-*.png`; three current preparation/cleanup captures stay private. Owned processes stop,5951 is unbound and the window is cleared. The initial queued f9 protocol cohort was cancelled before building after the guild online-zone wire change; both real consumers were built againstb3a34fe. No audio-engine, persistence, unrelated search-screen or general renderer-cleanliness claim.

## Recipe search scroll position

The [contract](../../specs/modern-crafting.md#recipe-search-scroll-position) discards the recipe list offset on a changed nonempty query and retains it for ordinary refreshes. Existing `RegistryUi.set_state` typed scroll-transition/data replacement is extracted into `RegistryModel.replace_state`, preserving journal/map policies and exposing that actual production boundary to CPU tests. Profession view replacement compares the old/new query and resets only the recipe list registry offset; it does not mutate the view or selection.

Ignored RED was pushed first:80 concrete matching recipes remain over two pages, but a new query retains630px instead of0. Controls cover a second nonempty query after scrolling, unchanged-query refresh/no-op retention, owner/selection conservation and search's Favorites omission. The first native coordinate run exposed a prerequisite input bug before any query:5522px of real owner recipe content in a546px viewport remained at offset5 after18 wheel ticks. `GameClient::input` omitted professions from its pointer chain, so no profession path called the existing shared scroll handler. The approved correction adds that route and marks consumed wheel/thumb input handled, matching adjacent quest/achievement/guild panels. The failed native log/PNG remains RED evidence; a CPU shared-handler wheel/thumb test is supplemental and does not certify Godot event dispatch. Gates ss667 and pr676 land the state transition and input route respectively; profession switching is not included.

### Native search-scroll proof — 2026-10-17

The unchanged `godot/tests/modern_crafting_search_scroll_live.gd` passes both Modern and Forever at1920×1080 on engine `f5f65878b`, retained server `0406fb041`, protocol `f9a9175`. Actual coordinate wheel input now establishes a deep offset in the5522px owner list. Typing broad searches `a` and `e` starts each result list at child offset5; matching content stays4625px/5008px against546px viewport, so clamping cannot masquerade as reset. Both result sets remain user-scrollable; unchanged frames retain those offsets. The selected schematic and bags/copper/skill remain unchanged; no cast starts.

Both drained clients exit0 with one unique completion marker and five unique captures each, with zero `ERROR`/`SCRIPT ERROR` lines. Existing geoset/QuestPOI/MSAA warnings remain. CPU tests separately prove unchanged-query state refresh/no-op and ordinary refresh retention, plus shared thumb movement/release. The native retry proves wheel dispatch, not a separately exercised physical thumb drag or inventory-triggered refresh timing.

Evidence SSOT: canonical `data/diagnostics/recipe-search-scroll-2026-10-17/native-search-scroll-proof.json` binds exact cohort/binary receipts, independently hash-verified server reuse, unchanged fixture hash, original routing RED archive, actual multi-page extents, ten PNG hashes and cleanup. Inspected originals: `/syncthing/AgentShared/2026-10-17/recipe-search-scroll/{Modern,Forever}-{1..5}-*.png`. All owned processes stop,5951 is unbound and the crafting window is cleared. No profession-switch, persistence, new filter or server-authority claim.

## Recipe Favorites

`ProfessionSnapshot.favorites` supplies character-owned favorite spell IDs; the client never promotes a pending preference request into authoritative state. Favorites uses a pseudo-category before ordinary categories only outside search, with separate favorite/ordinary row identities and selection. The right-click menu reuses the shared Retail menu primitive and preserves recipe selection. The [contract](../../specs/modern-crafting.md#recipe-favorites) owns sources/requirements.

### Accepted reopen proof — 2026-10-17

Engine `339c94be6`, server `9a8fd3951`, protocol `f9a9175`: six fresh native clients pass at1920×1080, sequentially in Modern and Forever. Set Favorite produces the authoritative duplicate while retaining ordinary selection; category collapse and search omission pass. Restarting the actual private UDP5951 server and reopening each client restores the favorite **without re-favoriting**. Remove Favorite retains the ordinary recipe; a second server/client restart keeps the favorite absent. Bags, copper and profession skill remain unchanged throughout. Three distinct server PIDs are recorded; all processes stop,5951 is unbound and the window is cleared.

Evidence SSOT: canonical `data/diagnostics/crafting-favorites-2026-10-17/native-reopen-proof.json` binds the binary hashes, fresh read-only world snapshot, three server generations, six client results, fourteen PNG hashes and cleanup. Inspected original PNGs only: `/syncthing/AgentShared/2026-10-17/recipe-favorites/{set,readback-remove,readback-empty}/{Modern,Forever}-*.png`. Existing known375188 is fixture knowledge, not acquisition evidence; unknown2153 rejection and same-account character isolation have separate server behavioral proof.

The first rendered attempt found a real input-boundary bug: default native Button input does not emit `AltClick`. Recipe actions must join `ui/projection.rs`'s frame-click prefixes (`profession:select:` and `profession:favorite-select:`). The physical-input RED precedes the two-prefix fix; unchanged-baseline GREEN follows its landing. A separate stale private world snapshot lacked `tdb_jump_charge_params`; a new read-only SQLite backup fixes the test environment without changing production data, code or the retained redb.

**Boundary:** synthetic `root.push_input` captures show menus near the viewport origin. Physical-device cursor anchoring/layout fidelity remains unproven, explicitly outside the accepted reopen result. Existing signed-geoset, QuestPOI and renderer/resource teardown diagnostics remain in the logs; no warning-free runtime claim.

## Learned/unlearned recipe filters

The [filter contract](../../specs/modern-crafting.md#learnedunlearned-recipe-filters) adds only the two sourced learned-state checkboxes. `ProfessionBook.hide_learned`/`hide_unlearned` default false, so both Show checkboxes start checked. `groups()` applies owner-snapshot knowledge before search/category grouping; Favorites additionally obey learned visibility. The shared Retail Filter dropdown reuses one content-width calculation for its right anchor and the neighboring search field. No other filter semantics or persistence are supplied.

Native input toggles presentation flags without sending a server request; window close resets them. Filter and favorite menus are mutually exclusive. Ignored screen RED was pushed first and failed on the missing dropdown. Two targeted screen tests pass: all four visibility combinations, checked-state art, search/empty controls, unchanged ownership and emitted source geometry. The first geometry harness incorrectly read native-only `layout_rect` before native layout; its corrected authored-output test and the later resolved native measurements remain separate proof levels.

### Native filter proof — 2026-10-17

Landed engine `cc4858b97`, server `58c0844b3`, protocol `f9a9175`: `godot/tests/modern_crafting_visibility_filters_live.gd` passes in both Modern and Forever at1920×1080 through actual Godot coordinate/key input. Each run covers all four checkbox combinations, learned-filter-aware Favorites, search's ordinary match without Favorites, close/reopen default-on state, unchanged bags/copper/skill and no cast. Authoritatively favoriting existing known375188 prepares the check; cleanup removes it again. The owner-projected unlearned row is293135; neither fixture knowledge nor browsing proves acquisition.

Resolved rectangles are identical in both skins: Filter `(208,175,87,18)`, search `(42,174,162,20)`, list background `(29,166,274,581)`. Thus the dropdown's right8/top9 anchor,18px height and4px search gap match the authored source. Each complete drained client process exits0 with exactly one completion marker, nine unique captures and zero `ERROR`/`SCRIPT ERROR` lines. Existing signed-geoset, QuestPOI and MSAA warnings remain; this is not a general warning-free renderer claim.

Evidence SSOT: canonical `data/diagnostics/recipe-visibility-filters-2026-10-17/native-visibility-proof.json` binds source/binary receipts, fresh read-only world snapshot, both process results, actual geometry, eighteen PNG hashes and cleanup. Four favorite-menu setup/cleanup captures stay private. Fourteen inspected filter-state originals: `/syncthing/AgentShared/2026-10-17/recipe-visibility-filters/{Modern,Forever}-{1..7}-*.png`. Owned server/client processes stop,5951 is unbound and the crafting window is cleared. Filter persistence and the other named filters remain excluded; the separate favorite physical-cursor anchoring boundary above is unchanged.

### Recipe filter Reset

The [contract](../../specs/modern-crafting.md#learnedunlearned-recipe-filters) exposes the existing shared `UIResetButtonTemplate` only when either visibility flag is nondefault. `ProfessionBook.reset_visibility_filters()` clears those two inverted flags; native Reset and window close share that operation. Search, selected recipe/instance, authoritative Favorites and other owner state are untouched. Search alone does not expose Reset. No additional filter family, persistence, server request or audio-engine change is included.

Three ignored screen REDs were pushed and failed before implementation: learned hidden, unlearned hidden and both hidden. Five targeted tests pass, including default/search visibility and complete view-state conservation after Reset. Lead gate rr647 passes205/0 plus fmt0; Reset lands at engine2535a4f17.

#### Native Reset proof — 2026-10-17

`godot/tests/modern_crafting_filter_reset_live.gd` passes both skins1920×1080 on engine `2535a4f17`, server `93d357ce3`, protocol `f9a9175`. Default flags plus active search keep Reset hidden; each of three nondefault combinations exposes it. Three actual coordinate Reset clicks per skin restore both flags and hide Reset while retaining the exact search text, selected recipe/favorite instance and owner Favorites. Clearing search reveals the still-selected favorite instance. Cleanup restores the original unfavorited preference; bags/copper/skill stay unchanged and no cast starts.

Resolved Reset bounds are `(281,164,23,23)` against Filter `(208,175,87,18)` in both skins, matching CENTER/TOPRIGHT−3,0 within native pixel rounding. Each drained native client exits0, emits exactly one completion marker and ten unique captures, with zero `ERROR`/`SCRIPT ERROR` lines. Existing signed-geoset/QuestPOI/MSAA warnings remain; no audio-engine or general renderer-cleanliness claim.

Evidence SSOT: canonical `data/diagnostics/recipe-filter-reset-2026-10-17/native-reset-proof.json` binds exact cohort/binary receipts, fresh read-only world snapshot, two clients/six Reset clicks, twenty PNG hashes and cleanup. Sixteen inspected state originals: `/syncthing/AgentShared/2026-10-17/recipe-filter-reset/{Modern,Forever}-{1..8}-*.png`; four favorite-menu preparation/cleanup captures stay private. All owned runtime stops,5951 is unbound and the window is cleared. Other filters/persistence and the separate physical favorite-cursor boundary remain excluded.

## Unlearned recipe discovery

The list uses separate learned/unlearned category groups; selection is independent of spell knowledge, while craft requests still require the authoritative known-spell snapshot. Native catalog projection includes unlearned recipes only from owned skill lines. `ProfessionSnapshot.recipe_sources` supplies read-only trainer rows for the source tooltip, without a new message or local acquisition inference. The [contract](../../specs/modern-crafting.md#unlearned-recipe-discovery) owns behavior and sources. Native both-skin acceptance remains pending the lead's granted5951 window.

## Trainer-learned recipe boundary

The private flow exposed a client-only craft block after a legitimate trainer purchase: Handstitched Leather Pants2153 was known at rank15, while the pinned automatic-learning threshold is30. The client reported zero crafts despite sufficient bags. Removing the invented acquisition-rank check restores authoritative knowledge/reagent decisions. TrinityCore and Retail source grounding lives in the [contract](../../specs/modern-crafting.md#learned-recipe-boundary). Unknown recipes still cannot produce a craft request.

## Bounded proof

Private UDP5770; accounts`fb_moderncraft_m`/`fb_moderncraft_f`; characters30/31. Both skins learned Classic Leatherworking from Simon Tanner, searched/selected recipes, crafted14 Light Leather from42 scraps, progressed rank1→15, learned pants2153, then consumed4 Leather+1 Thread for item2303/rank16. Native InventoryList ended with10 Leather, no Thread and1 Handstitched Leather Pants. Modern also resumed its persisted trainer/rank/recipe state after private server restart; final clean captures verify both persisted books at rank16.

| Scope | Status / proof |
|---|---|
| Categories/search | Both-skin native physical-input captures; authored-order RED→GREEN |
| Reagents / crafting tiers | Native concrete counts; both-skin model tests join quality14→tier2 and assert authored33×28 glyph; no native variable-quality outcome claim |
| Cast / result | Both-skin replicated player cast bars and InventoryList item2303 |
| Trainer / skill / persistence | Both-skin purchases/rank1→16; concrete server behavioral test; Modern restart and both final relogs |
| Effect288/157, concentration/inspiration, tier substitution | Missing/underivable operation or slot bindings; explicitly not implemented or certified |

Evidence SSOT: canonical `data/diagnostics/modern-crafting-2026-10-11/`. `modern/result-inventory.log`, `forever/result-inventory.log`, native logs and frame dumps retain the authoritative result. Inspected PNGs only are published to `/syncthing/AgentShared/2026-10-11/modern-crafting/`; logs/config/token state are not published there.

Targeted proof ledger: three exporter tests; nine ProfessionBook/catalog tests and four integration tests; final overlay-geometry test; chosen server trainer/craft test; scoped engine/server Cargo checks and workspace formatter checks. No full suite was run. Initial server check accidentally resolved the later canonical protocol (new unrelated guild requests) and failed; the same leased sibling graph used by the server build passed. Final checker/runtime pins that graph explicitly.

## Harness observations

NPC service gossip uses `AuctionGossipOption0`, not quest-only `GossipOption0`. An observer's extra interaction while transitioning away from the trainer reopened gossip; final clean captures avoid those extra interactions. The direct-InWorld attempt displayed Forever despite explicit Modern account configuration; normal charselect→EnterWorld applied the requested preset. These are retained observations, not profession behavior fixes or claims about all startup paths.

Native world logs contain unrelated missing scenery/NPC receipt failures; these did not prevent trainer/UI/cast/inventory proof. Headless import also emitted pre-existing editor dialog/thread and ObjectDB leak diagnostics. No warning suppression or fabricated assets were introduced.

## Authored quality selection — verified2026-10-16

Private UDP5951, `Craftmodern`1 and `Craftforever`2, both explicit skins at1920×1080. The current catalog reads `db2/active` (Retail12.1.5.70391). Recipe375188 retains no fabricated output. Physical inputs select193222×2 and mixed193208×6+193210×4; numeric9 clamps to6 on Enter, Cancel discards drafts, unowned tier3 stays disabled, and tier2 item hover resolves Resilient Leather. Server first rejects the missing slot0, then validates the complete owned allocation but returns `CRAFTING_OPERATION_COMPOSITION`; Create stays disabled and quality unassigned.

`godot/tests/modern_crafting_quality_live.gd` passed both runs. Six originals per skin live under `/syncthing/AgentShared/2026-10-16/modern-crafting/`; `Modern-2-mixed-tier-dialog.png` and `Forever-2-mixed-tier-dialog.png` are the inspected shared-layout comparison. Logs, exact gate heads and screenshot hashes live in canonical `data/diagnostics/modern-crafting-2026-10-16/gates/proof-ledger.jsonl`. Source receipts and unsourced mechanics are owned by the [gap ledger](../../sources/modern-crafting/gaps.md), not inferred from these UI captures.

The rendered run exposed a real native conversion failure: `show_professions` registered Portrait chrome, while the new quality dialog uses Plain chrome. Registering both styles resolves `ProfessionsQualityNineSlice` and lets the native input fields materialize; the original failing log is `gates/native-quality-red-missing-plain-chrome.log`. Harness fixes separately wait for asset startup and actual character-select control visibility: that UI node exists while the character-scene loading screen still hides its controls. Readiness requires the private admin pong, not merely a bound UDP socket.

## Knowledge reuse — verified2026-10-16

No second progression engine was added. Existing specialization authority passes six targeted tests, including authored recipe learning, one-point quest rewards, no committed refunds and atomic wallet/rank/recipe reopen. Private native `modern_crafting_knowledge_live.gd` at1920×1080 passes both skins: unlock tree350, inspect path28546/perk28534 tooltips, stage and Undo, then Apply. Both actual characters move0→1/30 and25→24 knowledge; fixture grants are not acquisition evidence.

A subsequent private server restart and fresh clients use the fixture's read-only `CRAFTING_PHASE=readback`, physically reopen the path and retain1/30 plus24 knowledge. `Modern-4-persisted-point.png` and `Forever-4-persisted-point.png` in the same AgentShared directory are the inspected stable relog comparison. The ignored server test `assert_private_modern_knowledge_commit_persisted` independently reopens that actual run database and verifies both characters' public specialization snapshots, with profession skill2830 still25. Post-quality conservation separately passes for both characters' three reagent stacks. Exact logs and hashes remain in the shared proof ledger above.

## Sources

- [Modern crafting contract](../../specs/modern-crafting.md) — requirements, static receipts, source citations and gaps.
- [ProfessionsFrame](../../specs/professions-frame.md) — Retail layout and existing trainer/craft behavior.
- `godot/ui-model/src/professions*.rs`, `godot/rust/src/professions.rs` — catalog, decisions and native projection.
- `../game-server/crates/server/src/profession.rs`, `profession_tests.rs` — existing authoritative pipeline and chosen behavioral proof.
- [Private live workflow](../../headless-live-run.md) — account/port isolation.

## See Also

- [[crafting-ui]] — specialization/order UI continuation and explicit recraft boundary.

- [[forever-preset]] — authenticated per-character skin selection.
- [[cache-import-sources]] — pinned CSV provenance.

## Concentration metadata boundary

`shared::concentration::ConcentrationCatalog` joins active ProfessionExpansion and CurrencyTypes CSVs. The server answers learned-line owner queries on ProfessionChannel without writing currencies; the native client requests the selected line and ignores replies for a different line. The shared screen renders Mainline ProfessionsCurrencyTemplate in both skins. Its currency tooltip uses authored GlobalStrings and explicitly exposes [CRAFTING_CONCENTRATION_ORIGIN](../../sources/modern-crafting/gaps.md#crafting_concentration_origin). An absent wallet entry stays unknown; static cap/recharge metadata does not initialize or run a refill clock. Both native skins passed the rendered1920×1080 UDP5951 fixture: `—/1000`, currency hover with the named origin gap, and disabled Create. Inspected originals: `/syncthing/AgentShared/2026-10-16/modern-crafting/{Modern,Forever}-{1-concentration-origin-gap,2-concentration-currency-tooltip}.png`. Catalog/owner tests4, screen tests9, consumer checks/builds and fmt passed at engine9a3553a8/server33fb1078/protocol7bbc06c0 (includes masterb701f32); fixture-only31b957bd6 corrects the tooltip snapshot's amount suffix. Evidence SSOT: canonical `data/diagnostics/modern-crafting-2026-10-16/gates/proof-ledger.jsonl` and `concentration*.log`. A stale private world snapshot was refreshed through a read-only backup of the already-published schema; the character redb was not reseeded. Offline reopening of the actual private redb after both clients stopped passes1: currency3049 remains absent for both customers. No live recharge or initial grant is certified.
