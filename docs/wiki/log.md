## Ruby emitter frustum cache — 2026-11-11

VERIFIED: `TerrainObjects::update_particles` reuses doodad frustum admission when the exact plane list is unchanged; the bit-field fixture matches fresh AABB culling for 25 bounds across changed and unchanged views. Particle simulation and deferral still run each frame. See [[follower-combat-fps#emitter-frustum-admission-cache-2026-11-11]].

UNVERIFIED: No paired A/B was run because the 1-minute `uptime` average was 14.56 at 13:21:18; no performance effect is claimed.

## GameObject M2 receipt gate closed — 2026-11-11

VERIFIED: The lead-published third fragment has 11 asset rows and 2 companion aliases, including receipts for M2s 166432 and 1697838; all 13 referenced payloads match their receipt hashes. On commit `d4bb12bf0f204a5ad4391c3369a6173d1645e8ec`, an extracted-only race-30 Paladin reached map 1860 at start distance 0.00 with two parsed terrain tiles, zero receipt errors and zero CASC diagnostics. The run used no flat aliases or fallback. Afterward, the private server stopped and all 23 temporary receipt-index links were removed; the slot's original cache path was restored. Evidence: `/home/osso/.worktrees/charcreate-receipts/agent-assets/live/gameobject-final-live-proof.json`. See [[m2-product-shadowing]].

## Initial GameObject M2 receipt boundary before third-fragment publication — 2026-11-11

VERIFIED: `load_visual_files` uses the Retail authored-model resolver. The FDID 1686555 resolver test passes against a receipt-qualified path with no flat file; formatter, focused test, dev build and release build pass with zero compiler warnings. The first extracted-only Paladin run reached map 1860 at start distance 0.00 with zero CASC diagnostics and no missing receipts for three target M2s. Two other map-object M2s (166432 and 1697838) had no row in the 152 canonical model-index files then present, so that checkpoint did not meet the full gate. See [[m2-product-shadowing]].
## Ruby particle quad packing — 2026-11-11

VERIFIED: The release Ruby baseline records `Culling` 1.532 ms/frame and `cull.particles` 1.147 ms/frame at the staged 38-unit pose. `ParticlePool::push` no longer zero-fills each 20-float instance before writing it; a bitwise layout test and the release particle-debug fixture pass. [[follower-combat-fps#particle-instance-buffer-zero-fill-2026-11-11]]

UNVERIFIED: No paired FPS A/B exists. The pre-run `uptime` check at 10:49:15 read 11.43/13.55/22.77 and rejected the run before client launch.

## Release FPS reprofile and surface parity — 2026-11-11

VERIFIED: The pinned release profiles record Stormwind (5,064 frames; p50 4.336 ms) and Ruby Life Pools out of combat (2,706 frames; p50 12.437 ms). The five largest main-system medians and render-attribution limits are in [[release-fps-reprofile-2026-11-11]]. The packed-stream change produced 25/25 byte-identical field outputs.

UNVERIFIED: The fixed-pose Stormwind pair is not a valid FPS comparison: one-minute host load rose from 21.18 to 36.58. Ruby has no paired after run.

## Floating combat-text parity — 2026-11-10

VERIFIED: Retail and Forever share combat-text Lua/XML/constants in the cached source audit. Both skins now render outgoing world-text punctuation, color, size, scaling and per-unit slot eviction, plus option-gated incoming self-frame damage, healing and miss messages; each behavior has targeted tests. The native gravity/min-alpha curve, directional offset, auto-attack merge and several CVar families remain unsupported or undecoded, not claimed as pixel parity. Debug/release checks report zero warnings; combat-text tests pass 26/26. JPEGs cover before/after world set, cross-unit eviction and self messages in each skin; twenty self labels cost 12.468 µs/frame in Forever and 20.518 µs/frame in Modern at the fixture's median 2000-iteration sample. Captures: `/syncthing/AgentShared/2026-11-10/combat-text-parity/`. See [[floating-combat-text-parity]].

## Godot audio bus-details cleanup — 2026-11-10

VERIFIED: the pinned Godot erased retired playback bus details one by one from a SafeList, quadratic in playing 3D sounds; in the 40-player Battle Shout crowd `AudioServer::_cleanup_lists` was 12.7-24.2 % of main-thread samples. Patched pin `-audiobus` retires them per update in O(n), deleting each one update later as before. Headless A/B, interleaved: 200 players 5.1-5.7 -> 1.4-1.7 ms main-thread CPU per frame, 400 players 16-22 -> 2.7-4.8 ms. See [[godot-audio-bus-graveyard]].

## Crowded combat: draping off the freeze — 2026-11-10

VERIFIED: master 4d4a5e7ab runs the 40-warrior Battle Shout phase at 319-493 frames per 20 s (p50 28.3-48.6 ms) where 94224f08d froze 9-24 s; main-thread draping is 0.95-1.97 ms per frame. Every step kept the 25 field-dump hashes and bytes. Cuts: drape cache, render-surface query, footprint floors, one copy per M2 LOS file (each column walked the same tile up to nine times), WMO batches once per column, cached batch streams, worker-thread draping with kits hidden until draped. Retail default settings show all 40 shouts (R19). See [[crowd-combat-fps]].

## Dungeon camera below ADT — 2026-11-09

VERIFIED: Ruby's Forever view changed from 16.169 yd and 17.969 yd camera height to 15.000 yd and 6.943 yd after removing ADT height-only collision; the intended 15 yd physics ray is clear. Native camera tests pass 17/17 and core camera tests pass 9/9. Debug and release workspace checks each report 0 warnings and 0 errors; fmt check passes. The outdoor Forever capture and real ADT-triangle collision test pass. Captures: `/syncthing/AgentShared/2026-11-09/dungeon-camera/{ruby-before,ruby-after,outdoor-after}.png`. Neltharus, Halls of Infusion and Azure Vault entrance samples did not reproduce; this does not qualify those full dungeons. See [[dungeon-camera-adt]].
## Ruby movement correction design — 2026-11-09

Verified the original stop is an authored62.82° rock face, while a centered entrance line walks. A separate1.2s private-server pause reproduces a correction with0 client collision rejections: normal2cm floor projection is counted as rejection while authority is movement-bank delayed. Added paired native/server acceptance and recorded the approved within-step-height projection rule; no collision constants change. [[ruby-movement-correction]]
## Rubyfps Human model and stale portrait test — 2026-11-09

VERIFIED: private copied-bench roster/replication and an independent engine build keep Rubyfps Human; a fresh Human Warrior also renders FDID1011653. Paired plateau captures at the reported camera remove only non-player meshes and identify the large foreground model as Ruby Scaleguard (drakonid2/4217881), not the player. VERIFIED: the named portrait test retains an October14 expectation superseded by the explicit October30 no-portrait decision; retain pet placement and repair only that stale expectation. Evidence and scope: [[follower-combat-fps#Rubyfps model verification (2026-11-09)]].

## Fountain moving-camera flash — 2026-11-09

Game-window-only video grids and tracked water crops establish blue-white basin discontinuities. An exact Linux camera-path A/B plus colourless transparent-list perturbation isolates missing authored batch priority. Binding the signed priority reduces the isolated sweep's maximum basin luma step29.168→0.290/255; the source-backed 120-pose raster acceptance passes with zero pixel mismatches. Debug/release `cargo check` both pass with zero warnings; `cargo fmt --check` passes. See [[fountain-camera-flash]].
## Character-select process fixture readiness — 2026-11-09

VERIFIED: `native_reconnect_fixture` and `native_transfer_fixture` pass in `d631259b9` after waiting for the expected authenticated roster and revealed, enabled Enter World controls. Reconnect reaches `RECONNECTED`; transfer completes all phases and receives one `WorldPortAck`. Debug/release workspace checks report zero warnings; fmt passes. The Forever screenshot and process/check logs are in `data/diagnostics/charselect-fixtures-2026-11-09/`. INFERRED, SOURCE-ONLY: `23c06168e` added early account polling during asset startup; no historical runtime bisect was performed.

## Ruby collision-only surface indexing fix — 2026-11-09

Ruby Life Pools' collision-only M2 carries inverted empty render bounds. Its transformed bounds caused the visible-surface query to build an unbounded dense grid. `TerrainObjects::index_surface_cells` now indexes doodads only when they own a non-empty render mesh surface; camera collision is unchanged. The real-M2 fixture passes (676-byte query delta); a private Ruby entry settled for 180 seconds at 4.99 GiB peak RSS versus 12.45 GiB last sampled before the 12 GiB cgroup OOM. Debug/release checks and fmt pass. [[client-map-cache-and-udp-framing-2026-11-08]]

## No-install certification poses: 11/11 pass — 2026-11-08

[Shipped assets](../specs/shipped-assets.md#p4-no-install-certification-run) defines the P4 zone/pose table and pass criteria. `scripts/noinstall_certification.py` runs one extracted-only process per pose with the install hidden and records missing-asset and fallback lines. All eleven poses passed after local fixes: 28 Retail M2 receipt gaps, 260 flat Forever WMO roots/groups and the 2959 skybox chain. All observed misses were source-present; none was absent from the build or a wrong-ID request. Evidence `data/diagnostics/noinstall-cert-2026-11-08/REPORT.md`. Bounded poses, not whole-catalog P4 closure.

## MH2O terrain water audit: one ocean LVF gap fixed — 2026-11-08

G7 audit against R-3d-terrain-water-mh2o. Matched: the layer's first `u16` is the type id verbatim (no 1..20 rewrite, no type-8 retry on the terrain path), a material-column-10 row is skipped, heights are a 9x9 grid with one 1/9-yard depth byte per vertex, coverage is one bit per tile, ocean type 2 draws at world y = 0, and the water shader's depth scale is `byte/255 = yards/28.333`. Gap fixed in `6cf912f36`: the ocean LVF 2 rule was keyed on object 42 instead of the layer's type with a sentinel format field, so a type 2 layer on a real LiquidObject above 42 would read LVF 0 heights from a depth-only block. New real-ADT test on `azeroth_31_58` (FDID 777877) proves 35 type 2 object 42 layers at y = 0 and 20 type 5/1 water layers with authored heights. No separate MapWater object exists; the per-chunk layers cover the same tiles. Captures and the per-frame A/B are in the page. [[mh2o-terrain-water-audit]]

## Rebuild guard counts per-frame scene and UI rebuilds — 2026-11-07

Counters for sky cubemap, environment, material rebinds, global shader writes, mesh and
ShaderMaterial creation, theme overrides, UI tree and shadow-atlas changes, with first call
sites, in every `BENCH_SEGMENT` and a steady-frame invariant test. It fails 8d0bea36d
outdoors (cubemap and light every frame) and passes master; it also found a double font
override per FontString update. [[rebuild-guard]]

## The 2959 hillside pond is legacy material-1 water with zero colour inputs
## Healer ground rings drawn on the visible surface

Client-only rings now sit under each party member: a clockwise-shrinking health arc
hidden above 92%, a pulsing outer ring for the dispel schools of the player's known
DB2 spells, screen-edge chevrons for off-screen members and click picking on the drawn
arc. Scope defaults on for healer parties, off for other roles and raids (lowest-N
opt-in), and the HUD page carries the controls in both skins. Forever captures and
pixel diffs live in `/syncthing/AgentShared/2026-11-05/healer-ground-ring/`; the
feature's own cost measures 0.12-0.16 ms per debug frame. [[healer-ground-rings]]

## Object-42 A/B on real release builds; streaming and legacy water colour still open
## Liquid colour was bound from the wrong row, so legacy water drew black

The Forever LightData rows leave all four water columns at zero on every zone slot; the only row that authors them is the Light's fixed record slot (0x001E303E close, 0x00001317 far). `river_*`/`ocean_*` therefore bound RGB zero with alpha only. The zone colour now wins where it is authored and the fixed row fills only the zero channels, so retail is unchanged and Forever water is no longer black. [[northshire-pale-water#Liquid colour now comes from the authored row — 2026-11-05]].

Consecration showed nothing because an area trigger replayed only the cast bursts (59536/189337)
and its golden circle lives in the display kit (59234/187938, `SpellVisualEvent` 9 -> 10). DB2
events 9/10 now decode to `DisplayStart`/`DisplayEnd` (cache format 20), the kits are held for
the trigger's lifetime, and an effect that authors `SpellVisualEffectName.EffectRadius` is fitted
to the trigger radius. Measured against a same-frame no-effect diff: Consecration 65,735 ->
331,089 px, Death and Decay 136,200 px at 9 s with no burst in flight. [[spell-visuals#Area trigger ground visuals (2026-11-05)]].
`bevy_to_tile_coords` returns `(row, col)`, which is also the ADT file-name order and MAIN's `(index % 64, index / 64)` order — proven by `azeroth_32_48`'s own first KNCM position and by Stormwind loading 9 tiles on the untouched streamer. The harness pose inversion had row and col swapped, so every snapped pose asked for the transposed tile, which `queue_tile` correctly refuses in silence. A one-sided ordering swap appeared to fix it and then loaded other regions' ADTs into the slots, rendering floating sheets with sky gaps. Harness fixed, streamer unchanged; `WORLD_CENSUS_PLACEMENT` and `core/tests/adt_tile_seams.rs` (129 shared vertices, 0.0 mismatch) now guard it. [[northshire-pale-water#The census harness inverted the tile axes — 2026-11-05]].
Same failure class as the Forever maps but a different shape: the M2 is published and only its `skin` is not (`model 201781 ... FDID 507146.skin`). The client resolves retail product `wow` build 12.1.5.70783, key `bc493ecb2898766c0f8947a99b4b3d65`, from `/syncthing/World of Warcraft`, not `/mnt/c`. `scripts/publish_forever_census.py` now takes `--product/--build/--build-key`; seeding it with the failing M2s published 53 receipts and both Orgrimmar poses went to 0 failures. [[northshire-pale-water#Retail receipts: skin-only gaps at Orgrimmar — 2026-11-05]].
Four MH2O layers at `y=14.7` (`liquid_type 5`, `liquid_object 17921`, `material 1`, `LVF 0`, `water.gdshader`) cover it. The shader mixes the `river_*`/`ocean_*` globals, and those bind as `[0,0,0,0.5]/[0,0,0,1]` and `[0,0,0,0.75]/[0,0,0,1]` — RGB all zero, alpha only. The LiquidType colour triple is zero too, so the surface is specular and foam over black. This is the legacy material-1 gap (per-view environment records, writer untraced), not a liquid-path bug; the ocean layers on the same map bind the same pair. [[northshire-pale-water#The 2959 hillside pond is legacy material-1 water whose colour inputs are all zero — 2026-11-05]].
Paired Linux-release A/B at the Stormwind indoor pose (same client, Forever, 1920x1080, host load recorded) names the two regressions behind the failed 120 FPS gate: `d5f97ee2a` reads every unit's merged model bounds each frame (6.90 ms of a 10.37 ms Culling step for 53 units) and the 2026-11-05 foot IK solve ran every unit within 45 yd, scanning all 16,691 doodads per foot. Caching the bounds at visual/equipment attach and skipping unsampled poses plus an 8-yard doodad column index took Culling 10.70 -> 1.92 ms and Foot IK 6.26 -> 0.25 ms, with 16,691/16,691 placements keeping identical visibility and shadow modes. [[godot-stormwind-fps#Culling and foot IK regression (2026-11-06)]].
The 71621ba9d Windows frame (84.1 FPS) is CPU-bound on two threads, not the GPU: a symbolized Linux profile at the canonical pose shows main 8.6 ms and render 8.1 ms per frame with the GPU at 3.1 ms, and half the render thread is draw submission. WMO exterior batches now merge per material and 256-yard cell (3030 -> 2671 draws, shadow 441 -> 327, identical same-pose capture), and nameplates stop resetting label theme overrides each frame. [[godot-stormwind-fps#Render thread and WMO draws (2026-11-06)]].
ADT water drew one instance per MH2O chunk layer: about 670 of 2650 draws at the Stormwind pose. Layers now merge per liquid and 4x4-chunk block (2650 -> 2045). The canal proof capture exposed `4cd7bc671`: its BlendAdd premultiply read `ALPHA`, which put every opaque M2 batch in Godot's transparent pass with no depth write and no shadows (water over boats, see-through tree canopies, shadowless characters). The premultiply now reads the value. [[godot-stormwind-fps#Water draws and the M2 alpha pass (2026-11-06)]].

Session 2026-11-08: per-map client resource caches were never released between maps (Stormwind baseline 1677 meshes/1524 textures/6131 materials/918 models, 4.13 GiB after a map switch); clearing them on world end plus the terrain worker cache and loader states fixes growth. Separately, UDP send pacing returned netcode-framed datagrams to `LinkSender`, so the netcode layer framed them again (239,122 `SizeMismatch(1200,1207..1225)` at one world entry); deferred frames now use a separate per-peer queue. [[client-map-cache-and-udp-framing-2026-11-08]]
WMO exterior shells now feed Godot occlusion culling: one ArrayOccluder3D per WMO from its opaque EXTERIOR-group batches (interior groups and dungeons never occlude), 128 rays per thread, Low BVH build quality because High stalled the viewport render 20-60 ms per streamed occluder. Linux outdoor 2450 -> 1464 draws, render thread 18.4 -> 13.6 Minstr/frame; indoor 1668 -> 1114. [[godot-stormwind-fps#Occlusion culling from WMO exterior shells (2026-11-08)]].
In-world object streaming had run with the 64-ms loading slice since 9a10e8154 gave the player a loading focus on every frame; restored to the 8-ms in-world budget (spec world-loading). Lakeshire -> Stormwind teleport on Windows: frames over 50 ms 60.7 -> 2.7, worst frame 226 -> 135 ms. [[stormwind-loading#In-world budget regression (November 9, 2026)]].
Replicated game objects no longer load their models inside the replication event: nodes place at once, workers load and decode, visuals attach under an 8 ms budget. Teleport into the Trade District, Linux debug main-thread CPU: worst replication event 110 -> 25 ms, worst frame 142 -> 85 ms. [[stormwind-loading#Game-object visuals off the replication event (November 9, 2026)]].
Follower combat profiling (Ruby Life Pools, four followers vs a Primalist pack): the unit frames resynced every frame while the player had any timed aura, because aura countdowns and swipe fractions lived in the screen state. The state now holds only the timer text and a timed flag. `Targeting` median 1.6-2.4 -> 0.2-0.4 ms out of combat and 2.8-4.2 -> 0.4-0.7 ms in combat (Linux debug). [[follower-combat-fps]].
Class bars (Evoker essence, DK runes) no longer rebuild the unit frames every frame: the state holds each bar at rest and the host patches the live animation onto its frames. Evoker in combat: unit-frame resyncs 817 -> 10 per run, `Targeting` CPU 4.4 -> 2.85 ms per frame (Linux debug). [[follower-combat-fps#Class bars resynced every frame (fixed, 2026-11-09)]].
World doodad particles no longer read their node's parent chain and every emitter's bone pose through Godot each frame: a placement holds its emitter transforms until its animation writes a pose. Ruby plateau, 160 visible particle doodads: 2 placements re-read per frame (13 us) instead of all of them (about 630 us). [[follower-combat-fps#Particle update re-read static transforms (fixed, 2026-11-09)]].
UI projection syncs update only nodes whose frame, rect, ancestors' rects or parent changed. Evoker combat: unit-frame nodes updated per sync 205 -> 4, unit-frame sync CPU about 3 -> 1.4 ms per frame (Linux debug). [[follower-combat-fps#Unit-frame projection redrew every node (fixed, 2026-11-09)]].
rebuild_guard_live.gd flaked on master (about 1 run in 3): the minimap clock-text relayout dirtied MinimapDisplay, and the projection re-uploaded any dirty frame's dynamic texture. The projection now redraws a dynamic texture only for new pixels. [[rebuild-guard#Minimap clock-tick flake — 2026-11-09]].
Crowded combat (40 geared warriors, Battle Shout spam): drape queries no longer call Godot per column, cast game-object rays only where a body meets the column, and skip M2 tiles and render surfaces whose root bounds miss it; same draped bytes, 29.2 -> 17.2 ms per batch (Linux debug). [[crowd-combat-fps]].

## Floating combat-text parity — 2026-11-10

VERIFIED: the cached Retail Lua/XML maps the self-frame scroll, 20-line pool, 1.9 s lifetime, 1.3 s fade and crit scale; the implementation now supports incoming Damage/Heal/Miss events behind the default-off switch and persists mapped CVar defaults. WRONG and fixed: world crit `!` punctuation and cross-unit slot eviction. MISSING: native directional/gravity/min-alpha/random-Z equations and screen-space packing across world units. The full source-line matrix and per-skin JPEG paths are in [[floating-combat-text-parity]].
