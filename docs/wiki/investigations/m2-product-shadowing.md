# Retail M2 cache shadows Forever NPC models

Initial investigation verified 2026-10-09 at engine `4c7b6cd30`, asset-resolver `80d16d05f`; runtime was unchanged then. Branch continuation and remaining acceptance are recorded below. [Approved contract](../../specs/product-isolated-model-assets.md) owns the 2026-10-09 user decision.

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

## Source-qualified runtime continuation

Branch `modelisolation` now carries selected products through creature/player/equipment/appearance requests. Model and material columns keep independent ownership; explicit replacements, skins, capes and body overlays select their own actual-build receipts. Compositor item pixels use `(product, FDID)` keys rather than merging with customization pixels by FDID. Parsed/decode/GPU caches retain scoped paths.

A second identity-loss boundary existed in `assets/mod.rs::model_asset_resolver`: the parser received a published model path but created an unqualified resolver rooted inside that namespace. A default-mode missing external animation could therefore request active-product bytes there. `08b86f04f` reconstructs the published product/build and original data root for nested acquisition. Fresh-process RED/ GREEN tests exercise namespace errors, tempting legacy animation bytes, scoped reads and tripwire0.

Focused CPU proof at `08b86f04f` / resolver `62bce6c`: resolver1/core6/native7 pass, zero failures (`scoped-final-green.log`). Frozen publication tests pass5/5, including install advance/config deletion and incremental dependency preservation. Host-native compilation exposes inherited Godot macro expression warnings; these are not a clean-warning claim.

`FrozenArchiveReader` acquires encoding keys from an authenticated frozen resolution snapshot without active install/config/root selection; its existing P1 guard rejects runtime use in extracted-only mode. Current offline receipts distinguish valid recovered files from unavailable archive entries; no legacy payloads are relabelled. [Contract gaps](../../specs/product-isolated-model-assets.md#known-gaps-current-cycle) remain the source of truth for authentic closure and named-display native acceptance. CPU success does not close those gaps.

## Independent native comparator

Native Vulkan/cage run of Retail display21774/model126278 through the same `WorldModels` preview path passes the scoped source/hash check, renders two meshes in `ExtractedOnly` mode and reports tripwire0. `native-comparator-1.log` records no model/texture/type9 error; host audio/XWayland/decor warnings remain separate. The requested PNG directory contains `display-21774.png`, inspected at640×360: a textured turkey is visible. This independent comparator consumes one of three native runs; it does not substitute for or close the two named displays' acceptance.

## Native acceptance (2026-10-10)

Verified: 2026-10-10. Rebased engine onto `origin/master` at `602500bb4`; resolver master `22751f4` is already an ancestor of `62bce6c`. Current scoped native tests pass14/14 (resolver1/core6/Godot7); native extension and acceptance metadata fixture builds emit zero compiler warnings. Master's `ab5257289` removes the20 inherited Godot macro warnings; no warning suppression was added. Runtime cage/Wayland capability warnings remain separate.

Re-extraction using authenticated frozen Retail69933 and Forever70291 resolution/root snapshots recovered every requested chain, including formerly unavailable Forever FDIDs7579618(M2),7731246(SKIN),7731197/7961203(BLP). Their payload content-key MD5, magic, SHA256 and actual build identities validate. Publication now holds504 asset receipts plus51 companion aliases, with scoped files copied and hash-checked in canonical data. No legacy bytes are relabelled; Forever bytes remain70291, not70205/70334. The large closure-extract inventory alone was not treated as proof of Forever ownership: each new publication has its own frozen-source local-archive receipt.

The maintained native fixture runs all cases even after a failure, reports exact counts/tripwire, and additionally renders a Human male with item25 through the production player/equipment loader. Framing uses visible mesh bounds, not the oversized authored animation bounds. Final proof at engine`021c39a81`/resolver`62bce6c`, `data/diagnostics/modelisolation-2026-10-10/native-acceptance-framed.log`:

| Case | Observable result | Proof |
|---|---|---|
| Forever139403/1100087 | 15 visible meshes; scoped70291 source/hash matches | Native GPU + inspected PNG |
| Forever139409/1100258 | 18 visible meshes; scoped70291 source/hash matches | Native GPU + inspected PNG |
| Retail21774/126278 | 2 visible meshes; scoped69933 source/hash matches | Native GPU + inspected PNG |
| Human male + item25 | Body1011653 and item148132 match Retail receipts; hiding item changes5282 pixels | Native GPU + inspected PNG |

Final acceptance4/4, zero model/texture/type9 errors, `ExtractedOnly`, CASC tripwire0. Four1280×720 RGB PNGs in `/syncthing/AgentShared/2026-10-10/modelisolation/` decode successfully with ffmpeg; ffprobe dimensions and signalstats logs are in the diagnostics `inspection/` directory. Remaining P2 source gap list: empty. No full-world/catalog, missing historical metadata provenance, P3/P4 closure, pristine-bundle or release/deployment claim. Host XWayland/decor/VSync warnings are not compiler warnings or model failures.

## Terrain collision reproduced (2026-10-21)

Verified against runtime engine base `52e47cd5d`, current local Retail70448 and Forever70450 authenticated roots during [no-install certification](../../handoffs/noinstall-cert-2026-10-21.md). Converted character/equipment acceptance above does not cover terrain's unqualified model loader.

Skyborne map2991's visited nine ADT tiles place model579120 four times. The flat M2 declares101 vertices and primary SFID579121, while its flat SKIN maps125 vertices up to124. Native `m2::submesh_arrays` rejects vertex101; four `WorldObjects: Vertex 101 out of bounds` errors match those four placements.

Authenticated root comparison establishes a cross-product pair, not merely corrupt geometry:

| File | Flat MD5 | Current Retail70448 root | Current Forever70450 root |
|---|---|---|---|
|579120.m2|`7bc5de4defc7715c73327d1d94925cb4`|`ac9d93a9b4b9f3cfbb960331e1575492`|Matches flat|
|579121.skin|`e1fbb53052011d2ee2a009e692f364f0`|Matches flat|`cda5d039e301c0c41efd9a10a0fccf0a`|

A read-only current-root audit of all155 distinct M2s/1,017 placements in those nine tiles finds152 flat primary model/skin pairs matching at least one common current product and three mixed pairs:194286→497258(two placements),440161→501335(eight),579120→579121(four). Each mixed pair is Forever M2/Retail SKIN. `skyborne-pair-audit/{pairs,summary}.json` retains exact per-file matches; matching one product does not establish the placement's authored ownership or certify textures/other companions.

Neither579120/579121 identity had a scoped receipt at this checkpoint. `terrain/objects.rs::WorldObjects::new` creates an unqualified `local_resolver`; worker `load_asset` → `assets/creature.rs::load_model_files/cache_model_files` → `assets/mod.rs::read_model_file` reads flat siblings. The converted `authored_model_resolver` path is not used for terrain. Every required file can therefore exist and CASC tripwire remain0 while a model consumes another product's companion.

Evidence under the certification's private diagnostic root: `skyborne-small-model-candidates.json`, `skyborne-invalid-vertex-candidates.json` and `skyborne-vertex-root-authentication/{comparison,wow-root-keys,wow_classic_beta-root-keys}.json`. These record source bytes, placements and authenticated current roots. Fresh local extraction then authenticated both variants: Retail M2 has374 vertices and its skin's maximum is124; Forever M2 has101 vertices and its skin's maximum is100. Four scoped files (30,558bytes) are staged separately in `staged-product-collision/cache/model-asset-index-noinstall-cert-collision.json`; `round6-authentication/summary.json` records validation. No shared flat file is overwritten. Terrain ownership/runtime propagation remains unresolved, not part of the earlier P2 PASS or a rendering fallback opportunity.

### Terrain qualification implementation, acceptance pending

`75208c162` adds the ignored real-map worker regression and reproduces `Vertex101` (exit101). `7ded89db4` retains selected MapCatalog CSV provenance in WDT; terrain M2 worker/parsed keys include product+FDID, companion acquisition uses `authored_model_resolver`, decoded/material/particle textures keep the model's root, and particle pools key by texture namespace too. WMO geometry/material IO remains separate. Compile-only proof passes; scoped terrain source recovery, guarded GREEN and both-skin Skyborne/Mulgore acceptance remain open. [Current proof and harness incident](../../handoffs/noinstall-cert-2026-10-21.md#terrain-source-qualification-implementation-acceptance-pending) owns exact bounds; do not infer P4 from implementation.

## Sources

- [Native Skyborne checkpoint](../systems/forever-data.md#native-skyborne-rosterworld-recheck-2026-10-09)
- [Loader](../../../godot/rust/src/assets/creature.rs), [worker](../../../godot/rust/src/world_models.rs), [display row](../../../godot/core/src/game/creatures/creature_display_data.rs)
- Sibling `asset-resolver/src/casc_resolver.rs` lines303–324 and473–516; `src/paths.rs::casc_cache_path`; `src/lib.rs::CascListfileResolver`.
- [Executable regression](../../../godot/rust/tests/resolver_product_shadowing.rs)
- [Nested parser regression](../../../godot/rust/src/assets/model_parser_isolation_tests.rs), [source-aware compositor](../../../godot/rust/src/assets/player.rs) and [authored native fixture](../../../godot/tests/model_product_isolation.gd)

## See Also

- [Authored NPC rendering](../../specs/npc-appearance.md)
- [Forever data](../systems/forever-data.md)
