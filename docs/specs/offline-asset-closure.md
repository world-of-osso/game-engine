# Offline asset closure

Offline audit tooling in `scripts/asset_closure.py` computes a content-rooted dependency graph without using an install or CASC. [Usage and boundaries](../wiki/systems/offline-asset-closure.md). Northshire is a proof slice, not the shipped-content limit: the product ships all zones, races and items; a large download is accepted.

## What it must do

- [x] Traverse ADT textures/placements, WMO groups/materials/doodads, and M2 skins/skeletons/external animations/textures to a fixed point; retain all incoming reasons and missing leaves.
- [x] Seed Northshire tiles, a male level-1 Human Warrior, starting kit, local world.db spawn displays and spellbook SpellMisc icons.
- [x] Seed selected maps' authored skybox/celestial models through every Light.LightParamsID slot, LightParams.LightSkyboxID and LightSkybox FDIDs, including maps without terrain tiles; retain positive missing joins and traverse model descendants.
- [x] Seed dynamic game-object templates through GameObjectDisplayInfo.FileDataID: explicit tile/all selectors, unspawned templates in the full superset, native type15 transport WMOs versus other model requests, and selected template identities in the world fingerprint.
- [x] Emit deterministic JSON with FDID/type/path, edges, product/build provenance status, presence, sizes and SHA-256; fingerprint every resolver module, not just the entrypoint.
- [x] Report unknown joins, malformed bytes, missing expansion boundaries and conflicting local aliases instead of silently dropping dependencies.
- [x] Resolve named references with the runtime's persisted-local precedence and surviving SQLite import binding, not largest/minimum FDID heuristics; retain displacement gaps. Prefer declared Map.WdtFileDataID, M2 TXID and ADT MDID over stale names; a present TXID/MDID never falls back to names for zero/short slots. Hash the explicit read-only local-listfile snapshot used by the audit.
- [ ] Select full catalogs through configuration without changing the traversal; label approximate full-scope file/byte estimates separately from proven coverage.
- [x] Auditor model roots require a matching product/FDID/kind receipt; read and validate scoped payloads,never unqualified flat or another product's bytes. Model-owned SFID/SKID/animation/texture dependencies use the owning model namespace,including authenticated same-product aliases. Keep different products/build namespaces distinct in one graph. Native flat WMO/ADT inputs require their actual flat bytes' provenance; scoped publication alone does not qualify them or replace their paths. `RawByteGraph` is an explicitly named producer/parser discovery helper,not an auditor or missing-receipt fallback.

## Root ownership

- Auditor map roots follow native MapCatalog: Retail `db2/active/Map.csv`,then Forever70205 rows only when neither ID nor ASCII-case-insensitive directory collides. Placement/model descendants retain the selected map product.
- Player bodies follow the native source-local ChrRaceXChrModel→ChrModel→CreatureDisplayInfo→CreatureModelData chain. Only Skyborne95/96,both sexes,use Forever70205 body/customization data; Retail placeholders and Forever rows for other races do not substitute. Forever customization uses its product-local versioned cache; missing mappings/caches remain gaps. Default Skyborne requirements use the authored playable bits32/33. This body policy never selects owned-item source.
- Creature display roots use `cache/creature_display.sqlite` source_product/model/variation rows. Missing source is unresolved,not an inferred global product or a CSV fallback. NPC equipment uses `cache/outfit_links-v5.sqlite` source ownership independently for model and material resource groups.
- Owned-item selectors require the explicit ItemDefinitionSource,for example `{"id": 2947, "definition_source": "Retail"}` or `Forever70205`. Source-local appearance/resource joins must not borrow the shared NPC cache. Bare item IDs,including every ID selected by `items: all`,remain individually unresolved until their definition source is provided; race and graph product never supply it.
- Source cache rows establish request ownership,not authenticated cache-import provenance. Preserve that separate gap.

## Non-model provenance

- Selected flat texture/sound bytes qualify only against same-product,FDID,path and size/SHA256/content-key matches in the existing `provenance/closure-extract.jsonl` ledger. Scan once for requested identities; preserve matching historical builds instead of last-row precedence. Missing or mismatched receipts remain named gaps. Hash-only UI inventories do not authenticate origin.
- Native flat icon/visual texture requests remain flat; model-owned texture namespaces stay separate. Do not add generic sound/blob/DB2 rows to ModelAssetIndex.
- Each selected DB2 CSV requires its own directory's table export receipt,matching source product and CSV SHA256 with authenticated content. Missing,wrong-product or changed exports are not parsed as valid metadata. Authenticated partial exports expose readable rows but retain exact unreadable-record gaps. Receipt/input drift inside a graph invalidates qualification.

### Encrypted DB2 section policy

- [x] `DB2_ENCRYPTED_SECTION_KEY_UNAVAILABLE` qualifies only readable sections when retained DB2 bytes match the pinned source receipt's SHA256, the CSV matches its export hash, and every excluded section is proven encrypted with an identified key absent from the recorded key census. Missing raw bytes, unexplained unreadable rows, unproven encryption or changed hashes keep the table refused. Never flip whole-file `content_authenticated` to true for partial bytes.
- [x] Exclude unreadable encrypted-section rows as unreachable under this named policy and count them separately with section/key identities. Keep base records and copy-table rows distinct,with their total logical-row count; prove encrypted chunk coverage through the copy table. Never synthesize zero-valued rows. Other missing joins remain explicit gaps, not presumed encrypted content.
- [x] The approved key-availability premise is monotonic: pushed TACT keys accumulate, so absence from the later70529 local key census may establish absence for pinned70448 sections. Hash and disclose the later keyring/external-key census identity without relabelling DB2 bytes; a recovered70448 snapshot may replace it. This is an explicit policy premise, not a decoder-derived guarantee.

## How it works

- [Offline closure audit](../wiki/systems/offline-asset-closure.md).

## Implementation inventory

- `scripts/asset_closure.py` — raw producer/parser graph traversal,manifest and census CLI; the CLI always constructs the receipt-qualified auditor.
- `scripts/closure_receipts.py` — receipt-selected model IO,product/build-qualified graph identities and native flat-reader boundary.
- `scripts/closure_root_products.py` — native map/display/NPC resource ownership and explicit source-local owned-item roots.
- `scripts/closure_player_roots.py` — explicit native Skyborne-only player body/customization overlay; source-local body joins avoid the merged NPC display cache.
- `scripts/closure_nonmodel_receipts.py` — actual flat byte receipts and DB2 export provenance; no extraction or index publication.
- `scripts/closure_db2_policy.py` — pinned WDC5 encrypted-section/key-census proof and named unreachable-row exclusions; never whole-file provenance promotion.
- `scripts/closure_seeds.py` — metadata joins and configurable content roots.
- `scripts/closure-northshire.json` — four Northshire ADT tiles and one Human Warrior.
- `scripts/closure-full-catalog.json` — broad catalog selectors; not a completeness certificate.

## Tests asserting this spec

- `scripts/tests/test_asset_closure.py` — concrete raw producer binary chains,catalog joins,missing/malformed dependencies,cycles,aliases and census.
- `scripts/tests/test_closure_receipts.py` — actual CLI scoped-only descendants,mixed-product same-FDID roots,owning-build companion aliases,wrong-product/legacy rejection,corrupt payload rejection and unchanged native flat WMO material selection.
- `scripts/tests/test_closure_root_products.py` — native Retail ID/directory map precedence,sourced display/model/material roots,missing source refusal and source-local owned item isolation; ignored RED fixtures are pushed before implementation.
- `scripts/tests/test_closure_player_sources.py` — ignored RED-first four Skyborne pair overlays,unchanged Retail Human,missing Forever mapping/cache refusal and authored default requirement bits.
- `scripts/tests/test_closure_nonmodel_receipts.py` — ignored RED-first flat texture/sound product/path/content checks,historical receipt selection,hash-only provenance refusal,native flat icons and authenticated/partial/corrupt/missing DB2 export receipts.
- `scripts/tests/test_closure_db2_policy.py` — ignored RED-first readable rows,per-section exclusion counts,available-key refusal,unproven input rejection and key-census drift.
- `scripts/tests/test_closure_skyboxes.py` — map isolation, all authored lighting slots, skybox7345733/skin7345740, celestial roots, positive missing joins versus zero and tile-free catalog entrypoint.
- `scripts/tests/test_closure_game_objects.py` — observed entry202588/display2690/model200010/skin505852 chain, map/tile selection, transport WMO identity, unspawned roots, named missing rows and selection fingerprint changes.
- `scripts/tests/test_staged_asset_fragment.py` — canonical cross-fragment uniqueness, equal-byte/different-scope ownership, conflicting payload rejection and preserved legacy alias metadata.
- `scripts/tests/test_closure_terrain.py` — repeated MCNK layers, liquid/object joins, global blob, FDIDs versus flags, malformed offsets and not-needed evidence.
- `scripts/tests/test_closure_paths.py` — runtime SQLite displacement, local precedence, explicit FDIDs over names and visible unmapped gaps.
- `scripts/tests/test_closure_kit_effects.py` — concrete beam/texture/emission/barrage descendants, recursive chains, shader-only evidence, missing rows and unknown types.
- `scripts/tests/test_closure_db2_export.py` — local DB2 beam-chain identities, empty tables and isolated reader imports exported by exact-layout schema.
- `scripts/tests/test_closure_emitters.py` — recursive geometry/model cycles, packed texture indices, ribbon arrays and malformed filenames.
- `scripts/tests/test_closure_appearance.py` — authored HD bake selection, zero-bake non-substitution, conditional models, raw choices, NPC item materials and missing Extra rows.
- `scripts/tests/test_closure_wmo_liquid.py` — shared root flags, liquid DB2 descendants, green-lava no-IO, hidden-tile material IO and orphan root boundaries.
- `scripts/tests/test_closure_sound_identity.py`, `test_closure_readonly.py` — real sound headers/unknown bytes and read-transaction closure.
- `scripts/tests/test_closure_extract_rounds.py` — descendant extraction, bounded graph replacement and fixed-point termination excluding recorded failures without falsifying missing summaries.
- `scripts/tests/test_closure_convergence.py` — peer publication exposes new descendants despite zero own files/unchanged missing count; numeric skeleton presence does not satisfy a newly required owner alias.

## Known gaps (current cycle)

Round 4 reached the locally recoverable fixed point on2026-10-10. [Current dataset, residual reasons and proof](../wiki/systems/offline-asset-closure.md#round-4-local-fixed-point--verified-2026-10-10) own counts, input drift, hashes and bounded runtime-index publication. Root-absent dependencies, unreadable metadata, legacy provenance and runtime acceptance remain open; task-only process isolation does not establish production-driver memory acceptance.

- [ ] Legacy extracted files have no authenticated product/actual-build receipts. Hashing proves local bytes, not origin. Manifest identity remains unverified.
- [x] Inspect every ADT MCNK and MH2O instance; resolve MCLY ground effects through GroundEffectTexture/Doodad and liquids through LiquidObject/Type/XTexture, including renderer-global textures. Record inline-only chunks and zero-instance liquids as not-needed with evidence; missing required joins/unknown chunks stay unresolved. Absent optional ground-effect rows are not-needed only when the CSV is present and hashed and the current runtime skips the row before any model request; ignored MPTX/legacy MCLQ chunks are explicitly scoped to current runtime support, not Retail-format completeness.
- [x] Traverse known SpellVisualKitEffect discriminants through pinned local effect tables: attachments, beam chains, emission models, texture blends, decals, screen effects and sound kits. Preserve cycles, missing rows, unknown types and unparsed client-scene script boundaries. Numeric-only effects are scoped to current native file requests, not a claim of full Retail effect rendering.
- [x] Traverse native M2 particle geometry/recursive filenames, packed multitexture indices and ribbon texture-index arrays; preserve malformed arrays and unmapped names. Use the core 272/274 legacy header/stride, not a guessed alternate layout.
- [x] Join extended displays through authored Extra HD/SD material selection, raw customization element choices/conditional models and NPC item-display materials. Never replace a zero HD bake with SD. Resolve legacy unsupported-choice flags through mandatory raw element rows, not an assumed effect type. Missing joins, unknown voice semantics and cache/provenance boundaries remain explicit.
- [x] Collect explicit BFID bone/PFID physics satellite identities and identify unlisted sound formats from actual extracted headers. Preserve unknown bytes and malformed identities; do not claim runtime satellite support.
- [x] Resolve WMO MLIQ through owning MOHD flags and MOGP liquid fields using the core liquid-type translation, including shared/late root contexts. Reuse LiquidType/Material/XTexture and renderer-global inputs. Do not mistake the MLIQ MOMT index for a liquid DB2 identity or skip the material request for hidden tiles.
- [ ] Full spell selector is a conservative catalog superset; complete server reachability needs content policy.
- [ ] Full character selector includes modeled placeholder races; supported playable-pair policy and customization requirement evaluation remain unresolved.

## Recorded local-gap recheck

Separate opt-in `scripts/recheck_local_asset_gaps.py` consumes a recorded product/FDID/path inventory. It resolves the active build through authenticated local root/encoding metadata and IDX bounds, retries payloads in private staging, rejects unknown-key output, and publishes through the existing atomic no-clobber receipt path. Existing files must remain untouched; conflicts and invalid identities remain visible. It never imports server tables or fetches assets remotely.

`scripts/tests/test_recheck_local_asset_gaps.py` covers real extractor-process publication, unknown-key rejection, and conflicting existing-file preservation.

## Local metadata recovery

- ModelFileData export accepts the pinned Retail `2AE4E788` layout, retaining inline FileDataIDs after the bounding box and authored resource/flags/LOD values. Wrong layouts fail before CSV creation.
- Recovered CSVs use distinct paths selected explicitly by the closure config; existing metadata CSVs and display/customization caches remain untouched. Readable encrypted-table sections do not certify complete metadata or authenticate zero-filled records.

## Recorded legacy-byte authentication

`scripts/verify_legacy_product_assets.py` consumes no-install receipt misses and
checks each legacy candidate against authenticated local product root content keys
selected by required explicit `--build PRODUCT=VERSION` pins. Only an exact byte match for the requested
product publishes scoped bytes and a model-asset-index receipt. Build config,
encoding and root metadata hashes must validate first. Existing asset bytes are
never overwritten; root absence, missing bytes, byte mismatch and scoped conflicts
remain explicit results. A match in another product is diagnostic only, never a
substitution. Asset receipt kinds must match the native index contract: `blp`, `m2`, `wmo`,
`skin`, `skel`, `anim`, `bone`. Unsupported kinds fail fragment preparation,
preflight and publication before any payload or index writes; opaque closure
nodes remain audit evidence rather than native index members.
New staged fragments must dedupe against the canonical base index
and every fragment, then reject remaining duplicate asset identities; raw path repairs
must not create another product/FDID/kind owner. An identical authenticated asset
required at another model-build namespace must retain that requested path as a
provenance alias, without changing its unique existing owner. Alias metadata is
not another asset owner: preserved rows can omit build/content-key fields. The
auditor validates selected alias bytes against the same-product unique owner's
size,SHA256 and content key and records that owner's actual build; it never
infers origin from the alias directory or reads model-named legacy aliases in
place of numeric SFID/SKID inputs. Alias dedupe includes
the build namespace; same relative paths in different builds are distinct physical
requirements. Different content remains a conflict, never an alias or runtime fallback.
This bounded inventory does not verify
the entire legacy catalog.

`scripts/tests/test_verify_legacy_product_assets.py` covers authenticated local
metadata, corrupt metadata rejection, actual-build pinning, matching publication,
wrong-product/root-absent rejection and preservation of existing bytes.

## Out of scope

Runtime policy/tripwire (P1), product-isolated runtime keys/identity migration (P2), packaging/release enforcement, audio/UI scenario acceptance. The offline graph audit itself still performs no extraction. Neither tool establishes no-install gameplay acceptance.
