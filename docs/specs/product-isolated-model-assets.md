# Product-isolated model assets

**Approved contract — user decision 2026-10-09: option 1, isolate the full asset chain.** Subsequent shipping direction: extract assets offline with per-asset provenance; converted creature/player/equipment and NPC-appearance paths support explicit `extracted-only` runtime access, with no CASC initialization/extraction or legacy fallback. Keep the global default unchanged while terrain/WMO/spell/UI/sound extraction is handled separately. Displays 139403/139409 previously selected Forever metadata but received cached Retail M2 bytes. [Investigation](../wiki/investigations/m2-product-shadowing.md) records the collision and failing regression. This contract covers the primary model and its full asset dependencies, not UI skin selection.

## What it must do

Approved acceptance criteria:

- [x] A model request carries its authored product/build identity; identical FDIDs from Retail and Forever coexist without overwriting or borrowing bytes.
- [x] Disk files, parsed models and offline CASC resolution state honor that identity. Converted model/appearance paths in `extracted-only` mode read shipped files only. Missing matching assets error explicitly; no local-CASC or unqualified legacy fallback.
- [x] SFID skins, SKID skeletons and external animations inherit their model's identity. Referenced textures retain source identity through decoding/material publication too.
- [x] Preserve current Retail CDI precedence and metadata-source rules for equipment; do not assume every asset attached to a Forever NPC belongs to Forever.
- [x] Both named displays render from extracted-only model/appearance files without type9 failures; a comparable native run shows no new errors for other displays. Model loads do not require access to a WoW installation.

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
- `scripts/import_model_asset_chains.py`: publishes local-CASC staged chains from frozen actual-build provenance and its resolution snapshot, verifying every FDID's content key; writes scoped companions and a per-asset index, keeping metadata hashes separate. Publication must survive install upgrades/removal without re-selecting an active build.
- `cache/outfit_links-v4.sqlite`: source product retained separately for each selected display/model/material resource group. Existing Retail groups keep their rows and ownership; owned item catalogs retain their explicit source.
- `godot/rust/src/world_models.rs`, `assets/{creature,player,equipment,appearance}.rs`: source-qualified model requests, parsed/decode/publication keys and replacement textures. Equipment models, column materials, explicit replacements, capes and body overlays retain independently selected products; the compositor keys item pixels by `(product, FDID)` separately from customization/default pixels.
- Sibling `asset-resolver/src/{lib.rs,paths.rs,casc_resolver.rs}`: resolver configuration, cache-hit policy and CASC state.

## Tests asserting this spec

`godot/rust/tests/resolver_product_shadowing.rs`: enabled explicit-identity resolver IO tests alternate Retail/Forever/Retail within one process and reject tempting unqualified bytes. The `model_asset_` unit subset covers parsed models, companion IO, texture pixels/publication, metadata ownership and extracted-only constructor/parser behavior. `godot/tests/model_product_isolation.gd` renders both named Forever displays, a Retail creature and an equipped Human male; verifies source hashes, equipped-item pixel contribution and the CASC tripwire.

## Known gaps (current cycle)

- [x] Publish verified actual-build per-asset provenance before runtime ownership is consumed. Frozen source publication survives install upgrades and incremental imports retain earlier dependency graphs; no legacy bytes are relabelled.
- [x] Wire converted creature/player/equipment/appearance constructors, parsed/decode/publication keys, material/cape/body-overlay products and nested animation readers. Focused CPU gate: resolver1/core6/native7 tests pass; this is not named-display GPU acceptance.
- [x] Complete authentic named-display equipment/appearance asset availability and render acceptance. Frozen-build local archive extraction recovered all four earlier gaps; no borrowed, fabricated or zero-filled assets replace them.
- [x] Native acceptance 4/4: displays139403/139409/21774 and Human male equipped with item25. ExtractedOnly, tripwire0, no model/texture/type9 errors. Four inspected PNGs at `/syncthing/AgentShared/2026-10-10/modelisolation/`; [proof ledger](../wiki/investigations/m2-product-shadowing.md#native-acceptance-2026-10-10) records exact scope. This is P2, not full-catalog closure or pristine-bundle certification.

## Out of scope

Seven unrelated missing imported profiles, encrypted DB2 recovery, parser relaxation, fabricated textures and shared-cache replacement.
