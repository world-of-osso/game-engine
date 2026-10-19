# Native professions crafting

The native client extends the existing ProfessionBook/ProfessionsFrame, trainer and server spell pipeline. [Modern crafting](../../specs/modern-crafting.md) owns acceptance, profession choice and data gaps; [ProfessionsFrame](../../specs/professions-frame.md) owns the shared Retail presentation contract. Forever changes art, not recipe/craft rules.

Verified: 2026-10-11. Engine behavioral source31c2bba12, overlay geometryac11dd7d5, formatter orderd22ab6878; serverd76b0e3; matching leased protocolb4523221e28c84d845066a532fafb8f838d4bf69. No master merge, canonical code edits, world.db writes or UDP5000 use. Extracted CSV/DB2/texture assets persist in canonical data, outside tracked code.

## Catalog and rendering

`godot/ui-model/src/professions_catalog.rs` loads the active Retail CSV build; earlier proof below retains its original pinned inputs. Category order is the signed `TradeSkillCategory.OrderIndex` (the source includes−1), not alphabetical category names. Known recipes remain name-sorted within each category.

`professions_quality.rs` joins `Item.CraftingQualityID` → `CraftingQuality` → `CraftingQualityAtlasSet.IconInv` → `UiTextureAtlasElement.Name`. The stored tier is distinct from item rarity. `professions.rs` in the native extension supplies quality records only for displayed items; reagent slots draw the inventory overlay at its authored atlas size. Local-CASC export/layout/source receipts and absent outcome formulas are recorded in the [contract](../../specs/modern-crafting.md#quality-source-receipt), not inferred from item IDs.

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

Three ignored screen REDs were pushed and failed before implementation: learned hidden, unlearned hidden and both hidden. Targeted controls cover default/search visibility and complete view-state conservation after Reset. `godot/tests/modern_crafting_filter_reset_live.gd` prepares three native nondefault states, single Reset clicks, source23px/TOPRIGHT−3 geometry, retained search/favorite-instance selection and gameplay conservation; that proof remains pending its own lead gate/window. The accepted earlier filter proof above does not certify Reset.

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
