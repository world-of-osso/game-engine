# Retail M2 cache shadows Forever NPC models

Verified 2026-10-09 at engine `4c7b6cd30`, asset-resolver `80d16d05f`. No engine runtime fix. [Approved contract](../../specs/product-isolated-model-assets.md) owns the 2026-10-09 user decision.

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

## Approved implementation source boundary (2026-10-09)

The importer pins Forever70205/build`842b2e5d11f8d6fe257a5b73bd5cf6c6`; the initial local `.build.info` snapshot selected Forever70291/build`e8dd824cf6c3d96cd01f804ca2ea5a63`. The pinned config and corresponding resolver root are absent. Slot `data/db2/1.60.1.70205/provenance.json` and Retail69933 provenance receipts are absent too. Read-only paths and hashes: slot `data/diagnostics/modelisolation-2026-10-09/source-availability.json`. Existing [pinned shoulder boundary](../systems/forever-data.md#pinned-shoulder-asset-boundary-2026-10-06) records the earlier pinned-root absence.

The predecessor classic-beta model receipts were extracted with current70291, as their extraction log attests. Their type9 difference proves product shadowing, not authored70205 identity. Re-labelling those payloads as70205 would violate the approved contract. The user did not impose a70205-only asset-byte constraint: a fresh authenticated local import can retain its actual build in stable per-asset provenance, separately from unchanged metadata ownership. Missing historical source material is not itself a prohibition on that migration. At that checkpoint no assets had been migrated or overwritten. The continuation below supersedes asset-publication state; full-chain runtime wiring remains open. The earlier NVIDIA userspace/kernel mismatch blocked native proof at that checkpoint. Continuation verified both loaded kernel and userspace at615.78.08; that blocker is cleared, but no native acceptance has run.

Independent resolver work adds `AssetIdentity`, identity-scoped cache paths, checked extraction errors, per-identity CASC state and pinned config selection on sibling branch`modelisolation`. Engine Depot stages that slot with `DEPOT_SIBLING_ASSET_RESOLVER=/home/osso/.worktrees/asset-resolver-mi` (the existing generic sibling override). Test and source ownership propagation are separate gates; this API does not itself fix engine rendering.

## Continuation: authenticated offline chains (2026-10-09)

Fresh local `casc-local` extraction, not copying unqualified caches, recovered both1100087/1100258 under Retail69933/build`dcfc90fffd79ba00406ae46f5f657592` and Forever70291/build`e8dd824cf6c3d96cd01f804ca2ea5a63`. Every extracted payload matches the selected build's resolution.sqlite content-key MD5; extension magic/chunk bounds also validate. Retail closure:172 verified assets, zero failures. Forever closure:174 verified assets, zero failures, including the source-authored NPC bakes7476066/7476072. Receipts and dependency edges live at `data/diagnostics/modelisolation-2026-10-09/offline/<product>/<build-key>/chain-receipts.json`. Both chains are now published as scoped files and `cache/model-asset-index.json`:346 asset receipts plus32 companion aliases. All378 files were copied to canonical scoped paths with zero SHA256/MD5/size mismatches (`canonical-copy.json`). This is authenticated publication, not native rendering proof. Metadata remains source-selected independently;70291 bytes are never called70205.

Engine`e9d8d3450` retains `CreatureDisplay.source_product` through CSV import and SQLite lookup, preserving Retail collision precedence and invalidating the old cache schema. The existing source-owner regression at`7dcd0983c` failed0/1 for the missing source_product column, then the creature_display subset passed7/7 after implementation. Logs: `owner-red-exact.log`, `owner-green.log` in the same diagnostics directory. Parsed-model and companion IO tests pass. Decoder fixture failures were unsupported BC5 followed by reversed palette channels;`a16df0e6e` corrects the supported palettized fixture to the decoder's RGBA order, with GREEN pending. Resolver P1 enforcement is incorporated unchanged at`8fc769b`; engine P1 policy is cherry-picked unchanged. Receipt-index and equipment-group ownership implementations are committed with targeted GREEN pending; runtime identity selection, equipment/compositor source propagation and extracted-only native tripwire acceptance remain open.

During October9 work, the install advanced to Forever70334/build`029dc2e024aadc9e0d9073ac37126868` and removed the70291 config. The first publisher incorrectly re-selected the active build and rejected authentic staged70291 bytes. Exact upgrade/deleted-config regression:3PASS/1FAIL (`frozen-source-red.log`). `a653cb803` instead consumes frozen extraction identity and its resolution snapshot, independent of any installed build at publication. Actual Retail and Forever publishes exit0 (`publish-frozen-*.log`); unit GREEN pending. The70291 payloads retain their actual identity, never70334 or70205.

## Sources

- [Native Skyborne checkpoint](../systems/forever-data.md#native-skyborne-rosterworld-recheck-2026-10-09)
- [Loader](../../../godot/rust/src/assets/creature.rs), [worker](../../../godot/rust/src/world_models.rs), [display row](../../../godot/core/src/game/creatures/creature_display_data.rs)
- Sibling `asset-resolver/src/casc_resolver.rs` lines303–324 and473–516; `src/paths.rs::casc_cache_path`; `src/lib.rs::CascListfileResolver`.
- [Executable regression](../../../godot/rust/tests/resolver_product_shadowing.rs)

## See Also

- [Authored NPC rendering](../../specs/npc-appearance.md)
- [Forever data](../systems/forever-data.md)
