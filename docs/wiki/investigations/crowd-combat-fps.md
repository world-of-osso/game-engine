# Crowded combat FPS

Client frame cost with 40 geared player characters fighting at once. Melee alone is affordable. Spell ground kits were not: every Battle Shout cast draped a five-batch ground effect synchronously on the main thread, and 40 casters made the work faster than a frame could finish it, so the client froze for 9-62 s. Since master 4d4a5e7ab the drapes are cheaper, cached and run on worker threads; draping costs about 1-2 ms of main-thread time per frame during Battle Shout spam.

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

## Later cuts (2026-11-09/10)

Each kept the 25 field-dump hashes and bytes of master; costs are per kit (five batches, 7663 columns), warm, Linux debug build under load 30-50.

- Drape cache (`f85c66af5`): draped meshes by source mesh, exact world-transform bits and kit anchor, cleared on any ground-epoch change (terrain data, drawn doodads/WMOs, colliding game objects; shared with foot IK). It hits for a caster recasting in place; a caster that moved 5 cm misses, and rounding the key would change output.
- Render-surface column query (`e15edeac8`): per-triangle up-facing flag at build, one direction reciprocal per query, a fixed traversal stack. City surface 233 -> 165 ms.
- Footprint floors (`82902ade7`): each kit samples collision through the WMO groups and M2 tiles whose bounds overlap its batches. Training-ground floor 62/43 -> 23/19 ms.
- One copy per M2 LOS file (`2a042d08b`): each parsed tile loaded its 3 x 3 neighbourhood of `.los` files, so with nine parsed tiles a column walked the same tile BVH up to nine times per kind and memory held about 63 MiB of duplicates around Azeroth (32, 48). M2 floor 14-29 -> 2-3 ms.
- WMO batches once per column (`a69fecae2`): a WMO group's box spans the merged exterior batches it belongs to, so a city column tested the same batch surface once per member group (about 660 tests for about 77 groups). City surface 153-215 -> 47-72 ms.
- Batch vertex streams once per source mesh (`aa322e26a`): reading them from Godot cost 0.5 ms per batch per drape.
- Worker draping (`51148fdfd`, `d3897514f`): a kit's uncached batches drape on two `ground-drape` threads over a main-thread copy of what their columns read (footprint floors, terrain chunk heights, render surfaces over the footprint). The kit is hidden until all its batches are back and drops them if it ended meanwhile. A surface not built yet travels as its meshes and is built on the worker, then adopted into the caches. A footprint a game-object floor ray can reach drapes on the main thread as before. The field dump also requires every cast to show draped (never flat) within 30 frames and 500 ms: first sight 3-18 frames (32-96 ms), cache hits the next frame. Main-thread snapshot per kit: 0.2-0.75 ms warm; 22-108 ms at first sight of a location, almost all of it Godot mesh reads for new surfaces.

## Crowd results

Shout phase, timing builds, matched runs (`profile/crowd-r2-*`, `crowd-r3-*`):

| build | load | frames in 20 s | p50 / p95 ms | worst-frame Spell visuals | main-thread drape per frame |
|---|---|---|---|---|---|
| 94224f08d | 36-54 | 1 | 9-24 s freeze | 6.6-17 s | synchronous, 16-29 ms per batch |
| 7232e52d4 | 24-28 | 431-497 | 23.5-27.4 / 105-147 | 711-836 ms | 5-13 ms |
| 4d4a5e7ab | 28-46 | 319-493 | 28.3-48.6 / 55.5-114 | 1.6-37.5 ms | 0.95-1.97 ms |

## Retail reference

DB2 (12.1.5.70448) confirms the kit plays for every cast: kit 279043 has no density filter, fallback or flag that would hide it. Retail's spell-clutter system (`SpellClutter.cpp`, Retail 70688 behaviour handoff R19) rejects other players' kits by a random draw only at effective `spellClutter` levels 1-3. The default `spellClutter` is 4 (rejection off), the Spell Density dropdown writes `graphicsSpellDensity` (density filters, default Full), and the raid side (`RAIDspellClutter` 2) applies only with `RAIDsettingsEnabled` on, default off. At default settings Retail shows all 40 shouts, so this client does not thin them. Neither `SpellVisualKitModelAttach.Flags` bit (4, 8) means draping.

## Packed surface streams (2026-11-11)

VERIFIED: Commit `39a6f13e7` retains the packed streams used to build M2/WMO surfaces; the baseline and after field dumps match byte-for-byte across all 25 batches. Single-run first-use times changed from 62/89/62/57/26 ms to 53/70/50/47/20 ms at flat/rug/stairs/wall/training; this is not a controlled FPS A/B. Full results and caveats: [[release-fps-reprofile-2026-11-11]].

## Open

UNVERIFIED: A clean same-pose FPS A/B is still unavailable. The fixed-pose Stormwind pair's one-minute host load changed from 21.18 to 36.58, and Ruby has no paired after run; do not treat the exploratory p50 delta as a code effect.

## See Also

- [[follower-combat-fps]] — follower dungeon combat, unit-frame costs
- [[spell-visuals]] — kit chain and ground draping
- [[release-fps-reprofile-2026-11-11]] — release pose profiles, system costs and output parity
