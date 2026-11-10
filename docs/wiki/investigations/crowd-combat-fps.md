# Crowded combat FPS

Client frame cost with 40 geared player characters fighting at once. Melee alone is affordable; spell ground kits are not: every Battle Shout cast drapes a five-batch ground effect synchronously on the main thread, and 40 casters create the work faster than one frame can finish it.

## Scene and method

Private game server (debug, `adcb9dd81`) on its own port with 40 level-90 Alliance warriors in full tier plus two-handers, on an 8 x 5 grid among the Stormwind Heroic Training Dummies (entry 31146). The server tool `tick_perf_peers` (game-server, `crates/client/examples`) logs them in and casts a live spell list every 2 s; its targets come from `npc-info` positions because most peers receive no NPC replication on the loaded debug server. An observer (`fb_wrf`/`Wrffps`) at WoW (-8770, 350, 101.5), noon, 1920x1080, runs `godot/tests/_diag_crowd.gd` (uncommitted): it waits for the 40 replicated `Crowd*` units, then measures one segment per spell phase (`slam=1464;shout=1464,6673`). Load the observer with Slam only: with Battle Shout active from login it never leaves the loading wait. Linux debug client; the host load varied from 22 to 164, so compare runs in pairs.

## Battle Shout ground kit

Battle Shout (6673) plays SpellVisual 190617: kit 279043 at the caster (TargetType 1, attachment -1) attaches `cfx_warrior_battleshout_castbase.m2` (FDID 1138011; skin: 5 batches, 28 vertices, 14 triangles); kit 279044 attaches FDID 6194303 on each target (an empty skin; the client logs "owner has no attachment Some(34)" for it). The kit chain matches DB2 (`SpellXSpellVisual` 616, `SpellVisualEvent` 562175/562176, `SpellVisualKitModelAttach` 638255/638256).

`drape_ground_effects` turns each flat batch into 930-3366 draped vertices and asks the ground for each of them: a collision floor (terrain, WMO collision, M2 floor/solid tiles, game-object physics ray) and the visible render surface (doodads and WMO batches over the column). Within one kit 97% of the query keys are distinct.

## Query costs (fixed, 2026-11-09)

Per draped batch on master 6cae39789: 72 ms (floor queries 41.7 ms over about 1530 calls, surface queries 29.9 ms), surface-array readback 2.0 ms, mesh build 0.8 ms. Inside them:

- Surface: about 85 candidate surfaces per query; each candidate cloned its `Gd`, looked its `RenderSurface` up by instance id, and the candidate list was cloned per query. Now each doodad and WMO group resolves its surfaces once (`36daa25bb`), and `RenderSurface::highest_upward_below` tests the root bounds before allocating its stack (`bdf5e0426`).
- Game-object floor: one `PhysicsRayQueryParameters3D` allocated per vertex, with 0 hits in 1,000,000 rays. Now one query object per pass, and a ray only where a game-object-layer body's physics-space bounds meet the column (`38c87309e`; `godot/tests/floor_ray_parity.gd`: 0 mismatches over 172,125 points, 84% of rays skipped).
- M2 floors: each sample entered the BVH of all 162 M2 floor/solid tiles around Stormwind. Now a tile whose root bounds miss the column is skipped first (`da9423334`).

`godot/tests/ground_drape_field_dump.gd` hashes every draped batch of the Battle Shout cast base at five fixed points (terrain, rug, WMO stairs, wall foot, training ground); each change produced the same 25 hashes as master.

Crowd A/B (master vs 94224f08d, matched pair): drape cost 29.2 -> 17.2 ms per batch; Slam-only frame p50 27.4 -> 22.1 ms; with Battle Shout one 62 s frame -> four frames at a 3.2 s median. Logs: `data/diagnostics/fps-120-close-2026-11-06/profile/crowd-ab-*.log`; report `crowd-combat.md` beside them.

## Open

With 40 casters about 50 batches per second still need more main-thread time than a second has, so the frame time keeps growing during Battle Shout spam. A per-frame drape budget would show undraped kits for some frames (not accepted); draping off the main thread needs the surface caches out of `Rc`.

## See Also

- [[follower-combat-fps]] — follower dungeon combat, unit-frame costs
- [[spell-visuals]] — kit chain and ground draping
