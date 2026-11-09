# Client map-cache retention and netcode double framing

Two independent session-lifetime failures surfaced together: a client grew past 20 GiB while visiting maps, and the server logged ~239,000 `lightyear_netcode` `SizeMismatch(1200, 1207..1225)` send errors.

## Per-map resources were process-wide

The client keeps four resource caches for the whole process: shared batch/merged `ArrayMesh` objects, uploaded `ImageTexture` objects, scenery/fade `ShaderMaterial` objects, and parsed M2 models. `reset_world` released the world nodes but not those caches; the `LoadTerrain` map-change branch and `NewWorld` transfers released even less. A map change therefore left every visited map's GPU and parsed resources alive.

Measured on the Stormwind pose after the world nodes were gone (engine before this fix): 1677 batch meshes, 1524 textures, 6131 scenery materials, 918 parsed models, 4.13 GiB RSS. After the fix, the same switch to map 2993 left 0 meshes, 3 textures, 0 scenery materials and 0 parsed models at 0.48 GiB before the new map loaded. The offline census then loaded maps 2859 and 2521 with 6 and 765 models, not the accumulated 771.

Fix: `GameClient::release_map_resource_caches` clears the three shared caches and parsed models in `reset_world`, the `LoadTerrain` map-change branch, and `NewWorld` transfers. `StreamedTerrain::reset` sends a `Clear` request so the long-lived terrain worker drops its WMO collision groups and decoded images. `AssetLoader::forget_loaded` drops finished keys together with the per-map model maps, so a revisited map loads again instead of waiting on a `Done` state.

Regression: `godot/tests/map_memory_census.gd` streams four maps in one offline client and prints the object-queue state at each settle; unit tests cover loader forgetting and the worker clear request.

## Ruby collision-only surface index — 2026-11-09 design

VERIFIED: engine `fca0c3ac9` exceeded a client-only 12 GiB cgroup while directly entering Ruby Life Pools (2521). Its last RSS sample at 66 seconds was 12.45 GiB; systemd recorded the OOM kill at 68 seconds. Godot's frame-driven memory sampler stopped at 6.2 seconds.

VERIFIED: after the fix, an authenticated direct Ruby entry on the same debug build completed 180 seconds under the same 12 GiB cap. Peak RSS was 4.99 GiB, peak anonymous RSS 4.18 GiB, and systemd's cgroup peak 5.2 GiB. At 178.9 seconds Godot reported 969,293,078 static bytes, 1,236,000,768 texture bytes, 190,771,336 render-buffer bytes, 148,534 objects, 109,168 nodes, 21 resources, 13,414 spawned doodads, 802 spawned WMOs and 665 cached models; pending/waiting queues were zero. The production fixture passed under 12 GiB with 676 bytes of static-memory growth, zero doodad surface cells and collision retained.

VERIFIED controls on the same build: Stockade (34) peaked at 2.93 GiB RSS and settled within 73 seconds; Stormwind (0) peaked at 4.39 GiB and remained settled through 180 seconds. Debug and release workspace checks passed with zero warnings/errors; fmt check passed. These are local debug live runs, not release performance receipts.

VERIFIED: a live stack locates the allocation in `TerrainObjects::visible_surface`, called by foot IK. Doodad 36449083 in ADT 4289434 uses M2 4695280 (`world/expansion09/doodads/deleteme/10xp_collision07.m2`). The M2 has no render vertices, an inverted empty render box (minimum +10,000,000; maximum -10,000,000), and separate real collision faces. Transforming all render-box corners converts that empty box into a 28-million-yard world box. The eight-yard visible-surface grid then attempts trillions of cells.

Rule and fix: `TerrainObjects::index_surface_cells` adds a doodad only when its Node3D root has a descendant MeshInstance3D with a non-empty mesh surface. This excludes collision-only models from a render-surface grid without changing their scene nodes, authored placements or camera collision bodies. The check runs once as each placement enters the index. It does not substitute collision geometry, clamp bounds, or change texture, particle, occlusion or map-cache policy.

Acceptance fixture `godot/tests/collision_only_surface_memory.gd` loads real M2 4695280 and its skin through the production placement path, verifies its camera collision and absence of render nodes, then requires the supplied terrain floor, zero doodad surface cells and less than 1 MiB static-memory growth. The initial implementation produced the expected cgroup OOM; post-fix fixture and authenticated Ruby measurements are recorded when verified.

## Framed datagrams returned to the transport queue

The vendored UDP send pacing put an unwritable datagram back into `LinkSender`. That queue also feeds the netcode server/client plugin, which treats every payload in it as a raw transport packet. A framed 1183-byte transport packet became a 1208-byte netcode datagram on the next tick and was rejected by `Server::send`'s 1200-byte limit. One Stormwind world entry produced 239,122 such errors (17k/minute); the sizes 1207..1225 match framed versions of near-MTU transport packets.

Fix in `shared-protocol/vendor/lightyear_udp`: deferred frames go to a UDP-owned per-peer queue, flushed before newly queued payloads. When the budget is spent or the socket is not writable, every payload behind it moves there too, leaving `LinkSender` empty of framed data. A max 40-player battleground scoreboard wire-size test asserts the reliable channel path only sees bounded fragments.

## Sources

- `data/diagnostics/bg-oom-mtu-2026-11-08/` — before/after server counters, census output and cache samples.
- [Surface-index acceptance fixture](../../../godot/tests/collision_only_surface_memory.gd) — loads the real collision-only M2 and checks the terrain-floor query, zero surface cells, memory growth and camera collision.
- [Surface-index implementation](../../../godot/rust/src/terrain/objects.rs) — excludes doodads without rendered mesh surfaces from visible-surface indexing.

## See Also

- [[networking]] — channel modes and transport layers.
