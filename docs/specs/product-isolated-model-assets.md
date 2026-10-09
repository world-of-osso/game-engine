# Product-isolated model assets — proposal

**Decision pending; runtime unchanged.** Displays 139403/139409 select Forever metadata but receive cached Retail M2 bytes. [Investigation](../wiki/investigations/m2-product-shadowing.md) records the collision and failing regression. This proposal covers the primary model and its asset dependencies, not UI skin selection.

## What it must do

Proposed acceptance criteria, not approved implementation requirements:

- [ ] A model request carries its authored product/build identity; identical FDIDs from Retail and Forever coexist without overwriting or borrowing bytes.
- [ ] Disk files, parsed models and CASC resolution state honor that identity. A missing matching asset errors explicitly; unqualified legacy files are not evidence of ownership.
- [ ] SFID skins, SKID skeletons and external animations inherit their model's identity. Referenced textures retain source identity through decoding/material publication too.
- [ ] Preserve current Retail CDI precedence and metadata-source rules for equipment; do not assume every asset attached to a Forever NPC belongs to Forever.
- [ ] Both named displays render without type9 failures; a comparable native run shows no new errors for other displays.

## Decision options

1. **Explicit product/build resolver and scoped asset paths — recommended.** Pass ownership from source-selected display/model data into extraction. Use one CASC state per identity and product/build-qualified disk and memory keys. Keep existing filename conventions inside each scoped model directory. Derive the build from verified importer provenance, not whichever build is currently active in the installation.
2. **Content-key-addressed blobs plus product/build/FDID mappings.** Equivalent isolation with deduplication, but requires a new manifest and companion-path adapter. Larger change than this failure justifies.
3. **One product per client process.** Simplifies resolver lifetime but cannot satisfy mixed Retail/Forever world content. Not recommended.

Approval must settle the identity/build contract and migration scope: isolate only the primary model chain first, or all referenced render assets in the same change. Recommendation: explicit identity throughout the referenced chain; preserve existing per-equipment metadata ownership. Re-extract/verify legacy files instead of relabeling them or retaining a legacy fallback. A directory-only patch or process-wide `WOW_PRODUCT=wow_classic_beta` is insufficient.

## How it works

- [Current resolver and loader boundaries](../wiki/investigations/m2-product-shadowing.md#collision-boundaries)
- [Existing NPC ownership contract](npc-appearance.md#forever-display-overlay)

## Implementation inventory

- `godot/core/src/game/creatures/creature_display_data.rs`: display row currently lacks source identity.
- `godot/rust/src/world_models.rs`, `assets/creature.rs`: model requests, file paths and parsed cache.
- Sibling `asset-resolver/src/{lib.rs,paths.rs,casc_resolver.rs}`: resolver configuration, cache-hit policy and CASC state.

## Tests asserting this spec

`godot/rust/tests/resolver_product_shadowing.rs`: explicitly ignored pending-design regression; run with `--ignored`. Real resolver file reads in fresh Retail/Forever subprocesses, concrete IDs and distinguishable cache receipts. RED is expected; this is not M2 parsing or native rendering proof. Update it to the approved explicit-identity API when implemented.

## Known gaps (current cycle)

- [ ] Identity API/build selection, legacy-cache migration and runtime implementation require approval.
- [ ] Authentic model/companion/render acceptance and post-fix error counts remain pending.

## Out of scope

Seven unrelated missing imported profiles, encrypted DB2 recovery, parser relaxation, fabricated textures and shared-cache replacement.
