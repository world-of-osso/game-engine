# Native professions crafting

The native client extends the existing ProfessionBook/ProfessionsFrame, trainer and server spell pipeline. [Modern crafting](../../specs/modern-crafting.md) owns acceptance, profession choice and data gaps; [ProfessionsFrame](../../specs/professions-frame.md) owns the shared Retail presentation contract. Forever changes art, not recipe/craft rules.

Verified: 2026-10-11. Engine behavioral source31c2bba12, overlay geometryac11dd7d5, formatter orderd22ab6878; serverd76b0e3; matching leased protocolb4523221e28c84d845066a532fafb8f838d4bf69. No master merge, canonical code edits, world.db writes or UDP5000 use. Extracted CSV/DB2/texture assets persist in canonical data, outside tracked code.

## Catalog and rendering

`godot/ui-model/src/professions_catalog.rs` loads the active Retail CSV build; earlier proof below retains its original pinned inputs. Category order is the signed `TradeSkillCategory.OrderIndex` (the source includes−1), not alphabetical category names. Known recipes remain name-sorted within each category.

`professions_quality.rs` joins `Item.CraftingQualityID` → `CraftingQuality` → `CraftingQualityAtlasSet.IconInv` → `UiTextureAtlasElement.Name`. The stored tier is distinct from item rarity. `professions.rs` in the native extension supplies quality records only for displayed items; reagent slots draw the inventory overlay at its authored atlas size. Local-CASC export/layout/source receipts and absent outcome formulas are recorded in the [contract](../../specs/modern-crafting.md#quality-source-receipt), not inferred from item IDs.

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
