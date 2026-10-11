# Native gathering

Verified: 2026-10-11, `gathering-nodes`; not merged into master. The [native contract](../../specs/gathering.md) owns requirements; [server gathering](../../../../game-server/docs/wiki/systems/gathering.md) owns authority, loot and timers.

## Representation and native paths

`GatherableResource.parent_skill` replicates authored GO50 nodes and fully looted skinnable corpses. Existing M2 rendering/picking, `UseGameObject`, cast feedback, loot window and profession snapshots carry the loop. Find Herbs2383 / Find Minerals2580 enable parent182/186 tracking, with Retail atlas647 members17635/17636 in both skins. Skinning does not enter those minimap filters.

The profession catalog records child skill IDs from `SkillLine.ParentSkillLineID`; without a selected recipe, the rank bar selects a learned child rather than a parent holder (`1/0` or Skinning `1/75`). Selected recipes keep their authored skill line. An open tracking menu uses the existing canvas-raise path, preventing the objective header from covering Find Minerals; no art or layout changes.

## Inspected native baseline

Private UDP5720, disposable `fb_gathering_nodes`, fresh private redb, no world.db writes or UDP5000 operations. Each client authenticated only after admin pong and UDP bind. Authored Silverleaf1617/display270 and Copper1731/display310 used real models, casts, loot and positive spawn timers; Mangy Wolf525 was killed through owner combat, normally looted, then skinned with8613. No NPC damage/kill or loot injection. The fixture explicitly teleports/restores actor health to isolate gathering from surrounding combat; this is not combat-balance proof.

| Capability | Modern | Forever |
|---|---|---|
| Exact herb/ore minimap marker, both filter rows visible | Native PNGs; markers separated from player arrow | Native PNGs; same filters/art |
| Herb cast, Silverleaf loot, rank, despawn/respawn | `integrated-modern-herb-a35`: 7→8/300 | `integrated-forever-herb-a36`: 8→9/300 |
| Ore cast, Copper Ore loot, rank, despawn/respawn | `integrated-modern-ore-a34`: 8→9/300 | `integrated-forever-ore-a33`: 7→8/300 |
| Owner kill, ordinary loot, Skinning loot/rank | `integrated-modern-skin-a37`: 4→5/300 | `integrated-forever-skin-a38`: 5→6/300 |

PNG originals: `/syncthing/AgentShared/2026-10-11/gathering-nodes/`. Inspected copies are at most1280px. Logs, readiness receipts, input/state/inventory receipts and `final-live-ledger.json`: canonical engine `data/diagnostics/gathering-live-2026-10-11/`. Copper219514.m2/509861.skin/219513.blp passed the verified-receipt render path; file presence alone was not counted.

## Regression and integration proof

Rank-parent selection: concrete both-skin RED/GREEN, including capped Skinning parent. Server corpse-ordering RED: a prematurely published Skinning marker blocked ordinary corpse loot; the producer now distinguishes completed empty rolls from unrolled corpses. After master integration, a separate RED/GREEN guards pending group rolls/shared slots through the server's full empty-loot predicate.

Merged requested master snapshots into this feature branch only: enginea83992738, server993ad16, protocol1101804. Preserved dependency pins/locks/patches and master chest/quest/battleground/locks handling alongside gathering. Scoped integrated GREEN: server gathering12; game objects6 with1 pre-existing imported-copy test ignored; native GO7; minimap11; profession-art3. Both cargo checks and changed-file formatting passed. The popup-order source change then rebuilt and checked; the six native cases above ran on integrated artifacts. No full suites.

## Limits

This proves the named Classic baseline, not every node/expansion/profession. PlayerCondition/nonpositive timer/underivable node mappings and unrelated ambient NPC asset gaps remain owned by the server and asset-closure pages. Parent owns independent verification and any merge into master.

## Sources

- [Tracking](../../../godot/ui-model/src/minimap_tracking.rs), [native cluster](../../../godot/rust/src/minimap.rs), [canvas raise](../../../godot/rust/src/window_stack.rs).
- [Profession catalog](../../../godot/ui-model/src/professions_catalog.rs), [rank regression](../../../godot/ui-model/tests/professions_art.rs).
- [Live fixture](../../../godot/tests/gathering_live.gd), [posed triangle samples](../../../godot/tests/posed_triangle_points.gd).
- [Server contract](../../../../game-server/docs/specs/gathering.md) and [runtime](../../../../game-server/docs/wiki/systems/gathering.md).

## See Also

- [[minimap]] — cluster and mask.
- [[professions-ui]] — owner snapshots.
- [[offline-asset-closure]] — verified payloads and remaining catalog gaps.
