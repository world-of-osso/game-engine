# Product-isolated model assets

**Approved contract — user decision 2026-10-09: option 1, isolate the full asset chain.** Subsequent shipping direction: extract assets offline with per-asset provenance; converted creature/player/equipment and NPC-appearance paths support explicit `extracted-only` runtime access, with no CASC initialization/extraction or legacy fallback. Terrain M2 placements also require the map's authored product throughout their companion, texture/decode and particle chain. WMO geometry/materials, terrain surface textures, spell/UI/sound extraction remain separate; keep the global default unchanged. Displays 139403/139409 previously selected Forever metadata but received cached Retail M2 bytes. [Investigation](../wiki/investigations/m2-product-shadowing.md) records the collision and failing regression. This contract covers the primary model and its full asset dependencies, not UI skin selection.

## What it must do

Approved acceptance criteria:

- [x] A model request carries its authored product/build identity; identical FDIDs from Retail and Forever coexist without overwriting or borrowing bytes.
- [x] `cache/model-asset-index-<name>.json` fragments publish authenticated offline assets. Runtime validates every fragment and rejects duplicate product/FDID/kind mappings; it never resolves competing receipts by fragment precedence. New-build publication updates the owning index instead of adding a duplicate mapping.
- [x] Disk files, parsed models and offline CASC resolution state honor that identity. Converted model/appearance paths in `extracted-only` mode read shipped files only. Missing matching assets error explicitly; no local-CASC or unqualified legacy fallback.
- [ ] A dependent asset uses the single index owner for its product, FDID and kind, even when that owner's build differs from the model's build. Missing receipts or owner files fail explicitly; do not substitute an unqualified file or publish a second owner.
- [ ] Resolve SFID skins, SKID skeletons, external animations and referenced textures by their own owner receipts. Numbered skin and model-named skeleton aliases are not loader inputs. Preserve owner selection through decoding and material publication.
- [x] Preserve current Retail CDI precedence and metadata-source rules for equipment; do not assume every asset attached to a Forever NPC belongs to Forever.
- [x] Retain authored ItemAppearance rows whose display ID is0. Those equipped slots draw no item model/textures/geoset changes but remain occupied; absent item/appearance rows and malformed display IDs still error. Do not substitute legacy or another product's appearance.
- [x] Both named displays render from extracted-only model/appearance files without type9 failures; a comparable native run shows no new errors for other displays. Model loads do not require access to a WoW installation.

## Installed-build extraction decision (2026-10-13)

Local-CASC runtime misses and new offline extraction follow the product's unique
active `.build.info` build. Resolution metadata/state must be keyed by that build,
including after a patch within a running process. Never require the installed
build to equal the authored DB2 export build. Existing published receipts/files
retain their authenticated historical identity and remain valid; newly extracted
files/receipts use the build actually read, never the former receipt's namespace.
`extracted-only` and explicitly authenticated frozen offline publication remain
unchanged. No DB2 re-export or old-build extraction fallback is authorized.
This supersedes the active-install prohibition below for local extraction only.

## Live Retail index migration (2026-10-13)

[DB2 follows the installed build](db2-follow-install.md#lead-publication) supersedes the earlier no-re-export/historical-live-index restriction for Retail. Authenticate identical bytes against the current root before recording the current build; extract and authenticate changed bytes. Keep one live mapping per product/FDID/kind, migrate companion aliases, and preserve original files and receipts only as history/backup. Stale frozen receipts cannot undo a newer published mapping. Runtime paths remain receipt-derived, without a compiled build key. Bundles include indexed paths, not every historical namespace. Canonical DB2 and index publication is one lead-only runtime-build command with backup and fingerprint diff.

## Decision record (2026-10-09)

1. **Explicit product/build resolver and scoped asset paths — recommended.** Pass ownership from source-selected display/model data into extraction. Use one CASC state per identity and product/build-qualified disk and memory keys. Keep existing filename conventions inside each scoped model directory. Derive the build from verified importer provenance, not whichever build is currently active in the installation.
2. **Content-key-addressed blobs plus product/build/FDID mappings.** Equivalent isolation with deduplication, but requires a new manifest and companion-path adapter. Larger change than this failure justifies.
3. **One product per client process.** Simplifies resolver lifetime but cannot satisfy mixed Retail/Forever world content. Not recommended.

Option 1 is approved for the full referenced render-asset chain, preserving existing per-equipment metadata ownership. Derive build identity from verified importer provenance, never the currently active installation. Re-extract and verify legacy files from local CASC instead of relabeling them or retaining a legacy fallback. A directory-only patch or process-wide `WOW_PRODUCT=wow_classic_beta` is insufficient.

## How it works

- [Current resolver and loader boundaries](../wiki/investigations/m2-product-shadowing.md#collision-boundaries)
- [Existing NPC ownership contract](npc-appearance.md#forever-display-overlay)

## Implementation inventory

- `godot/core/src/game/creatures/creature_display_data.rs`: display rows retain `source_product` through import/SQLite lookup, with Retail collision precedence. Runtime creature requests select that product's actual-build receipt.
- `scripts/import_model_asset_chains.py`: publishes local-CASC staged chains from frozen actual-build provenance and its resolution snapshot, verifying every FDID's content key; writes scoped companions and a per-asset index, keeping metadata hashes separate. Publication must survive install upgrades/removal without re-selecting an active build. Existing scoped bytes are never overwritten: equal bytes retain their inode; different bytes fail explicitly. The derived index can be regenerated independently of asset-byte publication.
- `scripts/verify_legacy_product_assets.py`: recorded no-install receipt misses are authenticated against the requested actual-build root before no-clobber publication. Numbered skin and model-named skeleton aliases additionally require the matching scoped M2 bytes and their exact SFID/SKID slot; canonical payload misses are not interpreted as aliases. Rejected legacy bytes remain unchanged and require independently authenticated alternatives.
- `cache/outfit_links-v4.sqlite`: source product retained separately for each selected display/model/material resource group. Existing Retail groups keep their rows and ownership; owned item catalogs retain their explicit source.
- `godot/core/src/map_catalog.rs`, `godot/rust/src/terrain/{assets,objects}.rs`, `particles.rs`: map-source terrain M2 ownership, product-qualified worker/parsed keys, scoped texture decode and particle-pool namespaces.
- `godot/rust/src/world_models.rs`, `assets/{creature,player,equipment,appearance}.rs`: source-qualified model requests, parsed/decode/publication keys and replacement textures. Equipment models, column materials, explicit replacements, capes and body overlays retain independently selected products; the compositor keys item pixels by `(product, FDID)` separately from customization/default pixels.
- Sibling `asset-resolver/src/{lib.rs,paths.rs,casc_resolver.rs}`: resolver configuration, cache-hit policy and CASC state.

## Tests asserting this spec

`terrain::objects::product_tests::terrain_model_asset_skyborne_579120_keeps_authored_sfid_namespace` exercises real map2991 metadata and production worker loading; flat Forever101-vertex M2/RetailSKIN fails Vertex101 before source qualification. Scoped Forever primarySFID579121 must parse every rendered submesh and retain its namespace.

`godot/rust/tests/resolver_product_shadowing.rs`: enabled explicit-identity resolver IO tests alternate Retail/Forever/Retail within one process and reject tempting unqualified bytes. The `model_asset_` unit subset covers parsed models, companion IO, texture pixels/publication, metadata ownership and extracted-only constructor/parser behavior. `godot/tests/model_product_isolation.gd` renders both named Forever displays, a Retail creature and an equipped Human male; verifies source hashes, equipped-item pixel contribution and the CASC tripwire.

`assets::parser_isolation_tests::retail_head_7321940_reads_primary_sfid_without_filename_alias_or_casc` copies only authentic70391 `7321940.m2` and SFID7530107 into an isolated product/build directory. Extracted-only parsing must retain its real bones/submesh2702 without a numbered alias or CASC access.

## Known gaps (current cycle)

- [x] Terrain M2 qualification: selected `Map.csv` provenance (Retail wins ID/directory conflicts) supplies each placement's product, including WMO-attached M2s. Worker and retained parsed keys include product; primary companions use `authored_model_resolver`, mesh/decode/particle textures retain the model's scoped root, and particle pools distinguish that namespace. No fallback or shared flat repair. The [map2991 model579120/SFID579121 collision](../wiki/investigations/m2-product-shadowing.md#terrain-collision-reproduced-2026-10-21) is the real RED; guarded extracted-only GREEN1 passes. After scoped publication, both-skin Skyborne/Mulgore runs retain expected terrain products and tripwire0; Vertex101 disappears. [Exact bounded coverage](../handoffs/noinstall-cert-2026-10-21.md#in-world-terrain-recheck-and-texture-co-location) retains Mulgore attachment34, absent superset root603672, separate-process/GPU-pool test limits. Lead reports integration gate ni895 GREEN/landed; separate Garrick visual acceptance remains open. This is not P4/full-catalog acceptance.

- [x] Publish verified actual-build per-asset provenance before runtime ownership is consumed. Frozen source publication survives install upgrades and incremental imports retain earlier dependency graphs; no legacy bytes are relabelled.
- [x] Wire converted creature/player/equipment/appearance constructors, parsed/decode/publication keys, material/cape/body-overlay products and nested animation readers. Focused CPU gate: resolver1/core6/native7 tests pass; this is not named-display GPU acceptance.
- [x] Complete authentic named-display equipment/appearance asset availability and render acceptance. Frozen-build local archive extraction recovered all four earlier gaps; no borrowed, fabricated or zero-filled assets replace them.
- [x] Native acceptance 4/4: displays139403/139409/21774 and Human male equipped with item25. ExtractedOnly, tripwire0, no model/texture/type9 errors. Four inspected PNGs at `/syncthing/AgentShared/2026-10-10/modelisolation/`; [proof ledger](../wiki/investigations/m2-product-shadowing.md#native-acceptance-2026-10-10) records exact scope. This is P2, not full-catalog closure or pristine-bundle certification.

## Out of scope

Seven unrelated missing imported profiles, encrypted DB2 recovery, parser relaxation, fabricated textures and shared-cache replacement.
