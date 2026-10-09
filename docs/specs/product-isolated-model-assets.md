# Product-isolated model assets

**Approved contract — user decision 2026-10-09: option 1, isolate the full asset chain.** Subsequent shipping direction: extract assets offline with per-asset provenance; converted creature/player/equipment and NPC-appearance paths support explicit `extracted-only` runtime access, with no CASC initialization/extraction or legacy fallback. Keep the global default unchanged while terrain/WMO/spell/UI/sound extraction is handled separately. Displays 139403/139409 select Forever metadata but receive cached Retail M2 bytes. [Investigation](../wiki/investigations/m2-product-shadowing.md) records the collision and failing regression. This contract covers the primary model and its full asset dependencies, not UI skin selection.

## What it must do

Approved acceptance criteria:

- [ ] A model request carries its authored product/build identity; identical FDIDs from Retail and Forever coexist without overwriting or borrowing bytes.
- [ ] Disk files, parsed models and offline CASC resolution state honor that identity. Converted model/appearance paths in `extracted-only` mode read shipped files only. Missing matching assets error explicitly; no local-CASC or unqualified legacy fallback.
- [ ] SFID skins, SKID skeletons and external animations inherit their model's identity. Referenced textures retain source identity through decoding/material publication too.
- [ ] Preserve current Retail CDI precedence and metadata-source rules for equipment; do not assume every asset attached to a Forever NPC belongs to Forever.
- [ ] Both named displays render from extracted-only model/appearance files without type9 failures; a comparable native run shows no new errors for other displays. Model loads do not require access to a WoW installation.

## Decision record (2026-10-09)

1. **Explicit product/build resolver and scoped asset paths — recommended.** Pass ownership from source-selected display/model data into extraction. Use one CASC state per identity and product/build-qualified disk and memory keys. Keep existing filename conventions inside each scoped model directory. Derive the build from verified importer provenance, not whichever build is currently active in the installation.
2. **Content-key-addressed blobs plus product/build/FDID mappings.** Equivalent isolation with deduplication, but requires a new manifest and companion-path adapter. Larger change than this failure justifies.
3. **One product per client process.** Simplifies resolver lifetime but cannot satisfy mixed Retail/Forever world content. Not recommended.

Option 1 is approved for the full referenced render-asset chain, preserving existing per-equipment metadata ownership. Derive build identity from verified importer provenance, never the currently active installation. Re-extract and verify legacy files from local CASC instead of relabeling them or retaining a legacy fallback. A directory-only patch or process-wide `WOW_PRODUCT=wow_classic_beta` is insufficient.

## How it works

- [Current resolver and loader boundaries](../wiki/investigations/m2-product-shadowing.md#collision-boundaries)
- [Existing NPC ownership contract](npc-appearance.md#forever-display-overlay)

## Implementation inventory

- `godot/core/src/game/creatures/creature_display_data.rs`: display row currently lacks source identity.
- `godot/rust/src/world_models.rs`, `assets/creature.rs`: model requests, file paths and parsed cache.
- Sibling `asset-resolver/src/{lib.rs,paths.rs,casc_resolver.rs}`: resolver configuration, cache-hit policy and CASC state.

## Tests asserting this spec

`godot/rust/tests/resolver_product_shadowing.rs`: resolver-boundary regression, to be enabled and updated to the approved explicit-identity API. Real resolver file reads in fresh Retail/Forever subprocesses, concrete IDs and distinguishable cache receipts. RED is expected; this is not M2 parsing or native rendering proof. Update it to the approved explicit-identity API when implemented.

## Known gaps (current cycle)

- [ ] Publish verified per-asset importer provenance before source/build ownership is consumed. Existing [source availability observations](../wiki/investigations/m2-product-shadowing.md#approved-implementation-source-boundary-2026-10-09) do not authorize relabelling current70291 bytes as70205; fresh extraction must retain its actual verified build.
- [ ] Implement approved identity API/build selection, legacy-cache migration and full-chain runtime isolation.
- [ ] Authentic model/companion/render acceptance and post-fix error counts remain pending.

## Out of scope

Seven unrelated missing imported profiles, encrypted DB2 recovery, parser relaxation, fabricated textures and shared-cache replacement.
