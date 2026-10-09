# Retail M2 cache shadows Forever NPC models

Verified 2026-10-09 at engine `4c7b6cd30`, asset-resolver `80d16d05f`. No runtime fix. [Pending proposal](../../specs/product-isolated-model-assets.md) owns the design decision.

## Collision boundaries

1. `world_models.rs::load_creature` reads the source-selected display but constructs `local_resolver(data_root)` without product identity. `CreatureDisplay` contains only `model_fdid`, four skin FDIDs and scale. `NpcAppearanceCatalogs` selects customization/compositor by CDI ownership; that decision does not select model bytes.
2. `assets/creature.rs::load_model_files` and `cached_model` key the process-wide `MODELS` map by `data_root/models/<fdid>.m2`. `cache_model_files` also writes skins, skeletons and animations into the unqualified model directory.
3. `asset-resolver::ensure_file_cached_at_path_with_paths` remaps to shared data and immediately returns any existing file. No product/build validation occurs. This is the first disk collision: changing `WOW_PRODUCT` cannot invalidate a cache hit.
4. On a miss, `get_casc` uses a single process-global `OnceLock<Option<CascState>>`. `WOW_PRODUCT` is read while opening the active build; separate resolver instances share that state. Resolution tables already live under `casc/<product>/<build-key>/schema-2`; extracted files do not inherit that isolation.

Decoded textures are additionally tracked by FDID and render materials consume unqualified texture paths. Isolating just the M2 directory leaves dependent asset identity unresolved. The existing equipment metadata contract sometimes retains Retail ownership: inheriting the NPC product blindly would change that contract.

## Two-display evidence

Read-only SQLite lookup independently confirms display139403 → FDID1100087 and display139409 → FDID1100258. Authentic local classic-beta bytes and cached bytes have different SHA256 hashes. Re-read MD21 texture arrays confirm type9 exists only in each cached version, not either classic-beta version.

Exact hashes, sizes, paths and texture arrays: `data/diagnostics/m2isolation-2026-10-09/asset-receipts.json`. These revalidate predecessor extraction receipts; no new extraction or cache overwrite occurred. Hash equality proves unchanged bytes, not that the installed build matches every imported table.

## Proof and remaining boundary

At `2cbd8f19c`, locked Depot native-crate integration test compiled and exited101: fresh Retail child passed1/1; fresh `WOW_PRODUCT=wow_classic_beta` child returned Retail bytes for **both** FDIDs. The parent expected failure is recorded in `data/diagnostics/m2isolation-2026-10-09/resolver-shadowing-red.log`. Small distinguishable payloads exercise real cache IO; they do not pretend to be valid M2s. Test is explicitly ignored in normal suites until design/implementation.

Independent recount of predecessor `world2-high.log` and `world2-wind.log`: each16 NPC error records across9 displays, with6 type9 records (five139403, one139409) and10 imported-coverage records across seven other displays. Receipts: `data/diagnostics/m2isolation-2026-10-09/baseline-counts.json`. No new native run, post-fix count or zero-regression claim: runtime unchanged.

## Sources

- [Native Skyborne checkpoint](../systems/forever-data.md#native-skyborne-rosterworld-recheck-2026-10-09)
- [Loader](../../../godot/rust/src/assets/creature.rs), [worker](../../../godot/rust/src/world_models.rs), [display row](../../../godot/core/src/game/creatures/creature_display_data.rs)
- Sibling `asset-resolver/src/casc_resolver.rs` lines303–324 and473–516; `src/paths.rs::casc_cache_path`; `src/lib.rs::CascListfileResolver`.
- [Executable regression](../../../godot/rust/tests/resolver_product_shadowing.rs)

## See Also

- [Authored NPC rendering](../../specs/npc-appearance.md)
- [Forever data](../systems/forever-data.md)
