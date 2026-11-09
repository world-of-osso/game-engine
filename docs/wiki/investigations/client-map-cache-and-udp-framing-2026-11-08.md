# Client map-cache retention and netcode double framing

Two independent session-lifetime failures surfaced together: a client grew past 20 GiB while visiting maps, and the server logged ~239,000 `lightyear_netcode` `SizeMismatch(1200, 1207..1225)` send errors.

## Per-map resources were process-wide

The client keeps four resource caches for the whole process: shared batch/merged `ArrayMesh` objects, uploaded `ImageTexture` objects, scenery/fade `ShaderMaterial` objects, and parsed M2 models. `reset_world` released the world nodes but not those caches; the `LoadTerrain` map-change branch and `NewWorld` transfers released even less. A map change therefore left every visited map's GPU and parsed resources alive.

Measured on the Stormwind pose after the world nodes were gone (engine before this fix): 1677 batch meshes, 1524 textures, 6131 scenery materials, 918 parsed models, 4.13 GiB RSS. After the fix, the same switch to map 2993 left 0 meshes, 3 textures, 0 scenery materials and 0 parsed models at 0.48 GiB before the new map loaded. The offline census then loaded maps 2859 and 2521 with 6 and 765 models, not the accumulated 771.

Fix: `GameClient::release_map_resource_caches` clears the three shared caches and parsed models in `reset_world`, the `LoadTerrain` map-change branch, and `NewWorld` transfers. `StreamedTerrain::reset` sends a `Clear` request so the long-lived terrain worker drops its WMO collision groups and decoded images. `AssetLoader::forget_loaded` drops finished keys together with the per-map model maps, so a revisited map loads again instead of waiting on a `Done` state.

Regression: `godot/tests/map_memory_census.gd` streams four maps in one offline client and prints the object-queue state at each settle; unit tests cover loader forgetting and the worker clear request.

## Ruby collision-only surface index — 2026-11-09 design

VERIFIED: engine `fca0c3ac9` still exceeds a client-only 12 GiB cgroup while directly entering Ruby Life Pools (2521). RSS reached 12.45 GiB at 66 seconds; Godot's frame-driven memory sampler stopped at 6.2 seconds. Stockade (34) peaked at 2.93 GiB and Stormwind at 4.39 GiB, with Stormwind settled through 180 seconds. These runs used a private server and the same debug extension, not release performance receipts.

VERIFIED: a live stack locates the allocation in `TerrainObjects::visible_surface`, called by foot IK. Doodad 36449083 in ADT 4289434 uses M2 4695280 (`world/expansion09/doodads/deleteme/10xp_collision07.m2`). The M2 has no render vertices, an inverted empty render box (minimum +10,000,000; maximum -10,000,000), and separate real collision faces. Transforming all render-box corners converts that empty box into a 28-million-yard world box. The eight-yard visible-surface grid then attempts trillions of cells.

Decided rule: a doodad with no render mesh has no visible render surface and does not enter that grid. Its scene node and camera collision remain. Do not use collision geometry as render geometry, clamp bounds, or change texture, particle, occlusion or map-cache policy. Test the production placement/query using the real M2: a supplied terrain floor remains the answer, zero doodad cells are retained, query static-memory growth stays below 1 MiB, and the camera collision shape survives. Acceptance fixture: `godot/tests/collision_only_surface_memory.gd`; implementation and after measurements are pending.

## Framed datagrams returned to the transport queue

The vendored UDP send pacing put an unwritable datagram back into `LinkSender`. That queue also feeds the netcode server/client plugin, which treats every payload in it as a raw transport packet. A framed 1183-byte transport packet became a 1208-byte netcode datagram on the next tick and was rejected by `Server::send`'s 1200-byte limit. One Stormwind world entry produced 239,122 such errors (17k/minute); the sizes 1207..1225 match framed versions of near-MTU transport packets.

Fix in `shared-protocol/vendor/lightyear_udp`: deferred frames go to a UDP-owned per-peer queue, flushed before newly queued payloads. When the budget is spent or the socket is not writable, every payload behind it moves there too, leaving `LinkSender` empty of framed data. A max 40-player battleground scoreboard wire-size test asserts the reliable channel path only sees bounded fragments.

## Sources

- `data/diagnostics/bg-oom-mtu-2026-11-08/` — before/after server counters, census output and cache samples.

## See Also

- [[networking]] — channel modes and transport layers.
