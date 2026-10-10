# Offline asset closure

Offline audit tooling in `scripts/asset_closure.py` computes a content-rooted dependency graph without using an install or CASC. [Usage and boundaries](../wiki/systems/offline-asset-closure.md). Northshire is a proof slice, not the shipped-content limit: the product ships all zones, races and items; a large download is accepted.

## What it must do

- [x] Traverse ADT textures/placements, WMO groups/materials/doodads, and M2 skins/skeletons/external animations/textures to a fixed point; retain all incoming reasons and missing leaves.
- [x] Seed Northshire tiles, a male level-1 Human Warrior, starting kit, local world.db spawn displays and spellbook SpellMisc icons.
- [x] Emit deterministic JSON with FDID/type/path, edges, product/build provenance status, presence, sizes and SHA-256; fingerprint every resolver module, not just the entrypoint.
- [x] Report unknown joins, malformed bytes, missing expansion boundaries and conflicting local aliases instead of silently dropping dependencies.
- [x] Resolve named references with the runtime's persisted-local precedence and surviving SQLite import binding, not largest/minimum FDID heuristics; retain displacement gaps. Prefer declared Map.WdtFileDataID, M2 TXID and ADT MDID over stale names; a present TXID/MDID never falls back to names for zero/short slots. Hash the explicit read-only local-listfile snapshot used by the audit.
- [ ] Select full catalogs through configuration without changing the traversal; label approximate full-scope file/byte estimates separately from proven coverage.

## How it works

- [Offline closure audit](../wiki/systems/offline-asset-closure.md).

## Implementation inventory

- `scripts/asset_closure.py` — extracted-byte parser, graph traversal, manifest and census CLI.
- `scripts/closure_seeds.py` — metadata joins and configurable content roots.
- `scripts/closure-northshire.json` — four Northshire ADT tiles and one Human Warrior.
- `scripts/closure-full-catalog.json` — broad catalog selectors; not a completeness certificate.

## Tests asserting this spec

- `scripts/tests/test_asset_closure.py` — concrete binary chains, catalog joins, missing/malformed dependencies, cycles, aliases and census.
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

- [ ] Legacy extracted files have no authenticated product/actual-build receipts. Hashing proves local bytes, not origin. Manifest identity remains unverified.
- [x] Inspect every ADT MCNK and MH2O instance; resolve MCLY ground effects through GroundEffectTexture/Doodad and liquids through LiquidObject/Type/XTexture, including renderer-global textures. Record inline-only chunks and zero-instance liquids as not-needed with evidence; missing required joins/unknown chunks stay unresolved. Absent optional ground-effect rows are not-needed only when the CSV is present and hashed and the current runtime skips the row before any model request; ignored MPTX/legacy MCLQ chunks are explicitly scoped to current runtime support, not Retail-format completeness.
- [x] Traverse known SpellVisualKitEffect discriminants through pinned local effect tables: attachments, beam chains, emission models, texture blends, decals, screen effects and sound kits. Preserve cycles, missing rows, unknown types and unparsed client-scene script boundaries. Numeric-only effects are scoped to current native file requests, not a claim of full Retail effect rendering.
- [x] Traverse native M2 particle geometry/recursive filenames, packed multitexture indices and ribbon texture-index arrays; preserve malformed arrays and unmapped names. Use the core 272/274 legacy header/stride, not a guessed alternate layout.
- [x] Join extended displays through authored Extra HD/SD material selection, raw customization element choices/conditional models and NPC item-display materials. Never replace a zero HD bake with SD. Resolve legacy unsupported-choice flags through mandatory raw element rows, not an assumed effect type. Missing joins, unknown voice semantics and cache/provenance boundaries remain explicit.
- [x] Collect explicit BFID bone/PFID physics satellite identities and identify unlisted sound formats from actual extracted headers. Preserve unknown bytes and malformed identities; do not claim runtime satellite support.
- [x] Resolve WMO MLIQ through owning MOHD flags and MOGP liquid fields using the core liquid-type translation, including shared/late root contexts. Reuse LiquidType/Material/XTexture and renderer-global inputs. Do not mistake the MLIQ MOMT index for a liquid DB2 identity or skip the material request for hidden tiles.
- [ ] Full spell selector is a conservative catalog superset; complete server reachability needs content policy.
- [ ] Full character selector includes modeled placeholder races; supported playable-pair policy and customization requirement evaluation remain unresolved.

## Out of scope

Runtime policy/tripwire (P1), product-isolated runtime keys/identity migration (P2), extraction, packaging/release enforcement, audio/UI scenario acceptance. This change cannot establish no-install gameplay acceptance.
