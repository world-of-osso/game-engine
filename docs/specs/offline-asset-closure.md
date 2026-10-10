# Offline asset closure

Offline audit tooling in `scripts/asset_closure.py` computes a content-rooted dependency graph without using an install or CASC. [Usage and boundaries](../wiki/systems/offline-asset-closure.md). Northshire is a proof slice, not the shipped-content limit: the product ships all zones, races and items; a large download is accepted.

## What it must do

- [x] Traverse ADT textures/placements, WMO groups/materials/doodads, and M2 skins/skeletons/external animations/textures to a fixed point; retain all incoming reasons and missing leaves.
- [x] Seed Northshire tiles, a male level-1 Human Warrior, starting kit, local world.db spawn displays and spellbook SpellMisc icons.
- [x] Emit deterministic JSON with FDID/type/path, edges, product/build provenance status, presence, sizes and SHA-256.
- [x] Report unknown joins, malformed bytes, missing expansion boundaries and conflicting local aliases instead of silently dropping dependencies.
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

## Known gaps (current cycle)

- [ ] Legacy extracted files have no authenticated product/actual-build receipts. Hashing proves local bytes, not origin. Manifest identity remains unverified.
- [ ] Liquid/ground-effect and emitter auxiliary edges are explicit unresolved boundaries, not complete enumeration.
- [ ] Full spell selector is a conservative catalog superset; complete server reachability needs content policy.
- [ ] Full character selector includes modeled placeholder races; supported playable-pair policy and customization requirement evaluation remain unresolved.

## Out of scope

Runtime policy/tripwire (P1), product-isolated runtime keys/identity migration (P2), extraction, packaging/release enforcement, audio/UI scenario acceptance. This change cannot establish no-install gameplay acceptance.
