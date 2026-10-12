# Whole-world NPC appearance coverage

Verified: 2026-10-11. Canonical world.db was read-only. Runtime proof used private UDP5891, fb_npc_appearance and one client at a time. Canonical data/cache remains untouched by this worker; the lead owns publication after the concurrent player publish finishes.

## Census and root causes

All226650 content spawn rows reference24599 displays through id1/id2/id3 template alternatives and explicit modelid, across hosted and instance maps. Census includes inactive events and zero-weight alternatives, not merely today's selected actors. Displays38043/38961 are absent from both catalogs but have zero weight beside valid38754; they are not runtime-selected appearance misses.

| Boundary | Before | Private after | Proof |
|---|---:|---:|---|
| Outside appearance coverage |13547|21|All-world read-only joins and strict cache merge;19 Forever profiles lack pinned-source Extra/Option/Geoset exports,2 unselected source-absent IDs|
| Root/companion/texture bytes |891 legacy-model and1489 variation-texture display misses; file presence did not prove ownership|No required model/texture/companion receipt/file misses in the display-root census|Authenticated local-CASC root MD5, product/build-qualified files and private index fragments; not a native world sweep|
| Ordinary geosets |1581 display selections ignored|CMD declaration retained; raw selection applied|Native Buzzard1105 parts0/101/102 RED →0/101 GREEN;13 focused core tests|
| Selected server model scale |DisplayScale discarded|Selected row's scale survives spawn and aura61 reconciliation|Unstable Corruption40923/display116569 RED None →GREEN0.45|
| Named actors |4213 hair1029851,4203 model1625963,8570 model123465 and13031 virtual-item texture147868 lacked receipts|Cobrahn,Skum,Onyxia rendered in Modern and Forever|Real replication, source-selected models/materials and inspected1280×720 PNGs; not encounter-mechanics/balance certification|

The old importer limited coverage to selected maps. `import_npc_appearance.py --world-db` now selects every world-reference root and reports source-absent IDs instead of substituting a model. Coverage extension initially failed on13 authentic recovered profiles already present without coverage: equal rows now survive; conflicting rows reject output.

Receipt census must include authored armor **and server virtual items**. The initial body/choice/armor graph missed Lucifron's weapon texture147868. The separate virtual-equipment pass joins exact `(ItemID, AppearanceModID)` through ItemModifiedAppearance and ItemAppearance, matching server selection; unresolved source items remain recorded, never invented.

## Ordinary geoset resolution

`CreatureModelData.CreatureGeosetDataID !=0` declares an ordinary display-selection model. CDI `GeosetIndex` is zero-based while M2 groups begin at100: selected mesh ID is `(index +1)*100 +value`. Preserve base0; disable unselected groups1–8, select explicitly named higher groups and leave unrelated higher groups unchanged. Apply only to this model's batches before attaching armor/virtual items. Ordinary displays still have no fabricated race/sex/class/body profile; missing coverage errors remain strict.

Concrete source: Buzzard1105 → CMD10253 → FDID1661349, declaration343, row index0/value1. Actual skin parts0/101/102; both variants previously drew. Boar1967 → CMD10605 → FDID1733750, row index0/value2 selects102 and omits201. Core importer rebuilds older creature-display cache schemas and retains Retail precedence on product collisions.

## Assets, publication and limits

Three local-CASC passes published129908 new private-index receipts and extracted28799 missing payloads, plus source-derived owner skin/skeleton aliases. No CDN, borrowed product bytes, overwritten asset files, fabricated IDs, data commits or canonical world.db writes. Byte graphs retain one unused physics satellite3154867 and four genuine item-material metadata joins as explicit gaps; native M2 loading does not consume that physics satellite.

Final private cache is rebased over the lead's latest canonical cache:16821→30274 coverage rows and8394→16993 profiles, preserving every canonical row including Clerk Gretal117678/race84/female. Expected NPC-cache SHA256 before`f23921c46f3d733aedb93d6cc29dd92e54462bb019968529807a267062c064d6`, after`8f05b8e8d5ce7b739b200ccf1ea9c3c455a1a7054fae930065000c9da2db4b28`. The schema-updated creature-display cache and deduplicated additive receipt fragments are in the same guarded package.

Lead publication command, **not executed by this worker**:

```text
python3 /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/npc-appearance-2026-10-11/apply_cache_package.py --apply
```

Without`--apply` it validates all source/before hashes, SQLite integrity, preserved canonical rows and Gretal. Any canonical drift refuses publication. Diagnostics: engine `data/diagnostics/npc-appearance-2026-10-11/` (census JSON/CSV, immutable before/private caches, manifests, proof logs and native revisions). Requested PNGs only: `/syncthing/AgentShared/2026-10-11/npc-appearance/`.

Unknown-parent encrypted Geoset records127642/127643 remain unassignable. This work is not a complete-profile/no-install certificate, nor evidence that every catalog-ready actor has rendered. Molten Core floor/entry-height failure prevented a stable Lucifron screenshot; the required raid actor proof uses Onyxia/map249. The owning instance-pool worker's transfer/login fixes were consumed from server master, not duplicated here.

## Sources

- [NPC rendering contract](../../specs/npc-appearance.md) and [importer](../../specs/npc-appearance-importer.md).
- `godot/core/src/game/creatures/{creature_display_cache,creature_display_data,npc_appearance_data}.rs` and `godot/rust/src/{world_models,assets/creature}.rs` — actual reader/selection boundaries.
- Local `~/Repos/wow.export/src/js/db/caches/DBCreatures.js:25–63`, `modules/tab_models.js:663–676` — CMD gate, one-based group mapping and ordinary selection. `~/Repos/solarityclient/crates/rendering/src/model/character_component/geoset.rs:64–89` — source group1–8 selection boundary.
- [Server content contract](../../../../game-server/docs/specs/retail-world-content.md) — modelid/weight/trigger selection and native object scale; TrinityCore a352b1fa ObjectMgr::ChooseDisplayId preserves the selected CreatureModel's display and scale.

## See Also

- [[character-rendering]] — character customization stays separate from ordinary model variants.
- [[offline-asset-closure]] — source authentication is distinct from native rendering and full closure.
- [[m2-product-shadowing]] — per-product/build asset identities and owner aliases.
