# Godot Stormwind FPS

The Godot client ran Stormwind at about 13–20 FPS (debug build, 1280x720, headless cage, Vulkan, AMD 890M), with about 6,400–7,400 draw calls. Hiding categories in a probe showed where the cost was. Draw calls were a real cost, but per-frame animation processing was a larger one. Three changes, each taken from the original client or from retail, bring the same views to about 20–25 FPS and about 4,300–5,000 draws. A fourth change (doodad animation LOD plus manual skeleton processing) brings the indoor view to about 31–38 FPS. The rest of the cost is the draws that remain.

## Method

`GameClient.frame_stats()` reports last-frame wall milliseconds for each client step and `client_process_ms`. `nodes_ms` sums each Rust node class's process/physics callbacks since the previous GameClient boundary; GameClient overlaps its steps. `attachment_gates_ms` covers attachment-gate calls in the same interval and overlaps either node callbacks or client steps. These timers add measurement overhead; the overlapping values must not be added together.

- The probe is an untracked scratch script, `godot/tests/_probe_draws.gd`. It enters the world as agent account `fb_fps`/`fbtest`, character `Fbfps`. It waits until every object has spawned (8,059 objects, 0 failures). It then averages the monitors over 30 frames for each variant: `Engine.get_frames_per_second` and the `RENDER_TOTAL_DRAW_CALLS/OBJECTS/PRIMITIVES_IN_FRAME` monitors.
- For each A/B pair, the "before" variant is forced from GDScript in the same session:
  - Doodads: every root is set visible and its `M2Animation` is set processing. The cull only writes when its cached state changes, so these forced values stay.
  - WMO groups: every batch is re-shown from `RenderingServer.frame_pre_draw`.
- Other agents share the host and GPU (load 6–20). One variant varied by up to ±5 FPS between runs, so this page reports ranges. Only differences seen in every run are claimed.
- Two spots:
  - Indoors: Fbfps at (-8785.9, 820.7, 97.65).
  - Stockade exit: (-8766.11, 845.50, 88.00), with the outdoors in view.

## Target and Bevy baseline

Target (user, 2026-10-28): at least 120 FPS (8.3 ms per frame) in Stormwind both outdoors and indoors, in a release build at 1080p. The user described the then-observed 7–55 FPS as unusable. The later Windows measurements below meet 120 FPS for the two tested views under the recorded load; they do not qualify other hardware or scenes. The engine stays Godot.

No Bevy Stormwind FPS measurement was found in `docs/`. The retired Bevy client's recorded in-world numbers:

| Bevy build | Scene | FPS | Source |
|---|---|---|---|
| release `bff94c42`, 1280×685 | Northshire spawn, 9 tiles (~9,900 doodads, 81 WMOs) | 9–24 | [movement-performance](movement-performance.md#release-in-world-fps-profile-2026-09-24) |
| dev `00e7b3b0`, all text enabled | InWorld, 71 remote entities | 12.3 | [rendering-pipeline](../systems/rendering-pipeline.md) |
| dev, Mailbox present fix `89f58874` | InWorld, unfocused | 28.4–29.3 | [rendering-pipeline](../systems/rendering-pipeline.md) |

The user also observed less than 20 FPS for a similar Stormwind scene in the Bevy client (not reproduced here). No measurement compares the two engines in the same scene.

## Material uniform updates (2026-10-28)

A symbolized release profile at the indoor spot attributed about 30% of main-thread time to `MaterialStorage::_update_queued_materials` and 14% to `RenderingServer::free_rid`. Each changing light rebound thousands of material parameters and replaced the terrain cubemap. The fix binds scene-light and fog flags on first use, writes changing water values as global shader parameters, and updates the six faces of one cubemap in place. A changing global texture alone would still queue its dependent materials in Godot's renderer.

Back-to-back release harness samples on the same host were 7–9 FPS / 116.95–145.11 ms before and 21–55 FPS / 19.06–46.03 ms after; a later after sample was 18 FPS / 54.55 ms. These are noisy instantaneous readings, not controlled averages. Both runs logged missing asset receipts, and the cage produced 1280×720 screenshots despite the 1920×1080 request. The captures have 44.51 dB PSNR; visual equivalence and the requested resolution remain unproven.

The 20-second post-change main-thread profile (1,759 samples) showed top self costs: render-list submission 5.42%, `std::thread::current` 4.42%, `pthread_mutex_lock` 4.39%, `Node3D::_propagate_transform_changed` 3.25%, and godot-ffi main-thread checking 2.82%. A loaded offline world fixture changes light time and observes zero light-binding material writes. The water, legacy water, retail fog, and terrain pixel fixtures passed; the offline world fixture also reported missing asset receipts.

## Where the frame went (before)

| Variant (Stockade exit, all objects) | FPS | Draws |
|---|---|---|
| Everything | 13.5–14 | 6,337–6,377 |
| All WMOs hidden | 12–13 | 5,171–5,194 |
| All doodads hidden | 17–19 | 2,583–2,598 |
| Terrain hidden | 14–16 | 5,428–5,433 |
| Units hidden | 12–14 | 5,912–5,913 |
| Nothing drawn, processing on | 21–32 | 2 |
| Nothing drawn, WorldObjects processing off | 43–47 | 2 |
| Nothing drawn, WorldObjects + WorldUnits processing off | 228–307 | 2 |

- **Draw breakdown:** doodad M2 batches are about 3,780 draws, WMO batches about 1,180, terrain chunks about 940, units about 460.
- **Instance counts:**
  - 7,997 doodads (12,393 batch instances), each with a `WowAnimationPlayer`.
  - 62 WMOs (4,597 batch instances).
  - 2,684 terrain chunks.
  - About 50 NPCs with about 9,000 bones.
- **Main result:** with nothing drawn, per-frame processing alone held the client at 21–32 FPS.

## Changes

1. **Retail scenery distance for ADT doodads.** This landed inside `8fdc22d0`, swept in from the shared index; the code is in [scenery.rs](../../../godot/rust/src/terrain/scenery.rs).
   - Rule (build 12340 `CMapObj`): the largest axis of the transformed M2 header render box (`0xA0`) picks a class, with inclusive limits of 1/4/15/100 yd. The class draws the doodad within 30/100/200/750/1250 yd of the box center.
   - Source: as reproduced by solarityclient `crates/runtime/src/application/m2_spatial.rs`, with `environmentDetail` at 1.
   - A hidden doodad also stops animating.
   - Retail's 5/10/15/20/50 yd fade band before the far radius is reproduced ([doodad-scenery-distance](../../specs/doodad-scenery-distance.md)): a fading doodad's opaque batches switch to a blended variant of their shader, and switch back at opacity 1, so fully opaque doodads stay in Godot's opaque pass. The fade variant keeps depth writes, and a fading doodad casts no shadow.
   - In-world check (Fbfps moved to Goldshire, WoW (-9470, 100, 60)): orbiting the camera moved Doodad12555 through opacity 0.099 → 0.539 → 1.0 → 0.369 → 0.157 → 0.091, and hid it once past the far radius. The same frame was captured as drawn, forced to opacity 1, and hidden. Over the doodad's pixels, the drawn frame sat at 0.860 (median) of the way from hidden to opaque for Doodad12266 at opacity 0.861, and at 0.450 for Doodad12267 at opacity 0.449. Before shadows were turned off for fading doodads, their unfaded shadows pulled these readings toward 1.
   - Result: about 1,400 of 7,997 doodads are drawn. Draws fell 6,393–7,490 → 4,269–5,090, and FPS rose 13–14.5 → 19–26 in the same session.
   - Screenshots with the cull and with every doodad forced on differ in 3 pixels indoors and 25 pixels at the Stockade exit, out of 768k. Those pixels are animation timing.
2. **NPC animation LOD** (`0be4373f`), a port of the original client's [npc-animation-lod](../../specs/npc-animation-lod.md).
   - An NPC samples its pose every frame within 30 yd of the camera, every other frame to 60 yd, and never beyond 60 yd.
   - An NPC off screen last frame (`VisibleOnScreenNotifier3D` over its batch bounds) is not sampled either.
   - The clock and crossfades keep running. A change skipped while frozen is written on the next sampled frame.
   - In practice about 11 of about 50 NPCs sample; the rest are frozen (13 far, about 25 off screen).
   - Result: toggling unit animation off went from +9.5 FPS (18.5 → 28) before the LOD to about 0–3 FPS after it.
3. **WMO portal culling** (`1723b9cf`), a port of the original client's `cull_wmo_portal_visibility`, in [portals.rs](../../../godot/rust/src/wmo/portals.rs).
   - The original client's rules are carried over: the Retail/WebWowViewerCpp traversal, the MOGI box plus floor-below camera group, polygon clipping against the frustum, and the 1.5 yd portal-plane rule.
   - About 1,600 of 4,597 WMO batches are hidden, but draws fell by only 60–140. Most hidden groups were already outside Godot's frustum.
   - No FPS change could be separated from the noise.
   - Screenshots with the cull and with it forced off differ in 2 pixels indoors and 57 pixels outdoors (foliage sway).
4. **Doodad animation LOD and manual skeleton processing** (user decision: doodads take the NPC LOD; [npc-animation-lod](../../specs/npc-animation-lod.md)).
   - Cause of the remaining ~20 ms: every doodad `Skeleton3D` (all 7,997, drawn or not) ran an engine internal process each frame. The default `modifier_callback_mode_process` (idle) enables it even with no `SkeletonModifier3D` to run. With the doodad `M2Animation`/`M2MaterialAnimation` nodes not processing, `WorldObjects` still reported 7,997 `Skeleton3D` internal processors, and disabling `WorldObjects` processing still raised FPS.
   - Fix 1: `build_skeleton` sets `MODIFIER_CALLBACK_MODE_PROCESS_MANUAL`. M2 skeletons have no modifiers; pose writes still mark the skeleton dirty and update the skin (Godot 4.7.2 `skeleton_3d.cpp`: `_make_dirty` → `_update_deferred`, independent of the mode).
   - Fix 2: the in-world doodad cull drives doodad animation instead of per-node `_process`. It uses `AnimationLod` from [lod.rs](../../../godot/rust/src/animation/lod.rs) (the NPC thresholds): not drawn, box outside the view frustum, or center beyond 60 yd → no sample; 30–60 yd → every other frame, staggered by unique ID; within 30 yd → every frame. A sampled frame advances by all time owed since the last sample (`DeferredClock`), so a doodad resumes at the current clock time. Material animation samples the shared material clock on the same frames. This matches solarityclient, which culls static placements from their bounds before advancing playback (`terrain_frame/m2.rs`).
   - Campsite doodads are not driven by the cull and keep processing every frame.
   - In practice at the indoor spot: of 1,387–1,389 drawn doodads, 519–523 are beyond 60 yd, 824–831 are off screen, 36–39 are 30–60 yd, and none are within 30 yd and on screen. 149 drawn doodads have a varying pose.
   - A/B (indoor spot, same session, 60-frame averages, 2 runs × 3 cycles, load 5–7): new (LOD + manual skeletons) 31–38 FPS; before (drawn doodads processing every frame + idle skeletons) 25–28.5 FPS. The new variant was faster in all 6 pairs. Splitting the two: LOD with idle skeletons 26.5–37; every-frame processing with manual skeletons 30.5–41. The manual skeleton mode is most of the gain; the LOD's own share is within the noise at this spot, where almost nothing drawn is on screen and near. The "before" variant also advances the LOD-sampled doodads a second time, which overstates its cost slightly.
   - A half-rate doodad still animates: Doodad5820806 at 36 yd, on screen through the doorway, changed its bone poses over 0.5 s and 398 pixels changed in a 120 px box around it (screenshots from the scratch probe).

## Remaining cost (after)

- **Doodad animation:** resolved by change 4. Before it, turning `WorldObjects` processing off and on gave 20.7/21.0/21.1 FPS off versus 14.4/11.3/16.4 on, about 20 ms per frame; most of it was `Skeleton3D` internal processing, not the animation nodes.
- **Draws:** about 4,300–5,000 remain. Of these, doodads are about 1,500, WMO batches about 1,300–1,600, terrain about 950, and units about 450–900.
  - Merging each WMO group's batches into one multi-surface mesh would cut instance count, not draw calls: Godot draws each surface separately.

## WMO doodads (branch `wmodoodadcost`)

WMO doodads (merged `17e44dfa`, `7e839a73`) add 8,694 `WmoDoodad` nodes at the indoor spot, about 49k nodes in all (each has a `Skeleton3D`, its internal `PhysicalBoneSimulator3D`, a `WowAnimationPlayer`, batch meshes, and sometimes attachments); 319 are drawn. The node count goes from about 67.5k (WMO doodads detached) to about 117k.

- **Profile.** `eu-stack -1` sampling of the main thread (150 samples at the hold) and timers around the three cull calls (instrumented build, 120-frame means). None of the WMO doodad nodes processes (`is_processing`/`is_processing_internal` all false); the WMO doodad cost was the Rust cull itself: `cull_doodads` walks all 16.7k doodads each frame, and for WMO doodads hashed each group into the WMO's `HashSet` of visible groups; `cull_wmos` rebuilt `HashSet`s in the portal traversal and read every batch's `is_visible` through the FFI each frame. Godot's render thread (`RenderingServer` draw loop) is the other large bucket, with or without WMO doodads.
- **Change** (`6cb884ec`). The traversal returns per-group flags (`Vec<bool>`); a WMO doodad's group test is an index; `cull_wmos` writes a group's batch visibility only when its drawn state changes. The drawn set is unchanged (`wmo_doodads_flow.gd` passes with the same counts).
- **Result.** The host's load moved between 2 and 16 during the runs, so the NPC LOD pass in the same frames (unchanged code) serves as the load reference. Before (`eac376b6`), two runs: `cull_wmos` 7.6-8.0 ms and `cull_doodads` 11.1-11.5 ms with NPC LOD at 1.1 ms; 3.6 ms and 5.3-5.6 ms with NPC LOD at 0.6 ms. After, three runs: 3.2-3.4 ms and 5.5-5.8 ms at NPC LOD 1.1; 2.9-3.2 ms and 6.5-7.2 ms at 1.2-1.4; 1.4-2.6 ms and 3.8-6.4 ms at 0.7-1.2. Relative to the NPC LOD pass, `cull_wmos` fell from 6.0-6.9x to 2.1-3.0x and `cull_doodads` from 9-10x to 4.9-5.7x, about 6-8 ms less per frame at the higher load. Whole-frame times (45-165 ms) moved with the load by more than that, so no FPS gain is claimed; detaching all WMO doodads changed frame time by -5 to +50 ms between samples in the same run.
- **Tried and dropped.** Skipping hidden doodads until the camera could reach their far radius cut nothing measurable (`cull_doodads` 5.4-6.1 ms either way) and was reverted.
- **Screenshots.** Same spot, `eac376b6` vs `6cb884ec`: 565 of 921,600 pixels differ, all around the animated props next to the player and the FPS counter.
- **Remaining.** About 5 ms of `cull_doodads` (all doodads, fade and animation LOD per doodad each frame) and 2-3 ms of `cull_wmos` (the camera-interior test over each WMO's interior floor triangles) in debug builds. The node count itself costs nothing per frame that the samples show.

## Linux Cage measurements (2026-10-31)

Engine `93ac8d4b5` release, shared-protocol `41d5928`, and private server binary `27d1a48f21`: NVIDIA RTX 5060 Ti, Forever skin, headless Cage/wlroots Vulkan, 1920×1080 at a configured 144 Hz. An output-management diagnostic applied the custom mode before Godot started. Godot requested disabled VSync, but Vulkan logged a fallback to enabled; this FIFO surface caps presentation at 144 Hz. The fixture reported its requested setting, not proof that the driver accepted it. Each result is a 300-frame sample with the Windows comparison poses and no per-frame render-server GPU queries.

| Pose | FPS, three samples | Main-thread CPU ms/frame | Agents.slice CPU cores |
|---|---|---|---|
| Indoor | 57.31 / 57.99 / 54.46 | 11.41 / 11.40 / 12.06 | 9.58 / 9.39 / 10.21 |
| Stockade exit | 46.90 / 70.74 / 74.15 | 13.13 / 9.29 / 8.86 | 23.25 / 12.21 / 10.45 |
| Indoor repeat | 72.59 / 70.00 / 66.72 | 8.72 / 9.15 / 9.87 | 5.17 / 6.16 / 6.77 |

A quiet admission window held new builds, but previously admitted jobs continued. Main-thread runqueue wait for the repeat was 0.00–0.02 ms/frame. One other Godot client was present before the indoor run, and two before the repeat; GPU load was shared and varied. A separate timestamp-instrumented Cage run measured roughly 4–5 ms GPU viewport time indoors, but made two synchronous render-server queries each frame and cannot be used as a query-free FPS baseline or directly paired with these samples. Actual CPU/GPU overlap and the remaining wall time are not established. Evidence, including per-second GPU and scheduler records: `data/diagnostics/fps-120-linux-2026-10-31-cage-query-free/` and `data/diagnostics/fps-120-linux-2026-10-31-cage-144hz-quiet/`.

### Symbolized Linux CPU profile

A separate diagnostic profiled the same two poses with engine `0122c9eae`, an optimized release extension with `debug=1`, and a symbolized Godot 4.7.2 release template. This is **not the shipped runtime**: the symbolized Godot has PRs 123946 and 123546, but lacks the later 123391 and font-atlas patches. Three 300-frame measurements were written per pose with the Forever skin, on 1920×1080 Cage/Vulkan at the 144 Hz FIFO fallback. Screenshots and per-frame render-server timing getters were removed from this profiling fixture. Its completion marker was absent; its log contains thousands of skeleton-disconnect errors and a render-device finalization error. The sampled records therefore qualify a diagnostic profile, not a clean runtime acceptance.

During the three-second perf captures, agents.slice used about 5.1 cores indoors and 3.6 at the exit. Two other Godot clients were present during most indoor samples; one remained at the exit. Main-thread runqueue wait was about 0.02 ms/frame. The first 300-frame sample measured 68.35 FPS indoors and 92.26 at the exit; profiling and the different runtime prevent a direct FPS comparison with the shipped binary. Mean main-thread scheduled CPU was 8.84/7.29 ms per frame. Render-thread schedstat estimated 8.07/6.21 ms per frame over the perf windows. Their combined CPU occupancy exceeded one core in both windows, so the threads did overlap; the measurements do not identify exactly where the main thread waits.

`perf` cycle-weighted self costs, normalized to each sampled thread, rank the following investigation candidates. Approximate milliseconds multiply those shares by scheduled CPU time; they are not measured savings, and the listed costs overlap larger inclusive system timers.

| Candidate | Indoor | Exit | Evidence and boundary |
|---|---|---|---|
| Reduce repeated binding/FFI work, without disabling safety checks | `std::thread::current` 14.64% ≈ 1.29 ms; class-ID lock 3.55% ≈ 0.31 ms | 7.46% ≈ 0.55 ms; lock 6.23% ≈ 0.46 ms | Main-thread self samples. godot-ffi's balanced main-thread check uses `std::thread::current`; the incomplete call stacks do not prove every sample came from that check. Measure call sites before changing them. |
| Spatial candidate indexing for doodads | `cull_doodads` self 10.07% ≈ 0.89 ms | 6.79% ≈ 0.50 ms | The broader Culling timer was 1.79/1.66 ms. The [client comparison](client-performance-techniques.md) describes reference approaches; preserve fade, animation, portal and shadow behavior. |
| Trace physics broad-phase work | BVH self 7.71% ≈ 0.68 ms | Individual AABB intersection self 3.97% ≈ 0.29 ms | The source of these samples is not proven. World-hover timing was 0.58/0.56 ms; it must not be assumed to account for all physics work. |
| Trace render material updates and submission | Uniform-set updates 6.76% ≈ 0.55 ms; render-list submission 4.63% ≈ 0.37 ms | 5.85% ≈ 0.36 ms; 7.74% ≈ 0.48 ms | Render-thread self samples. The producing materials and potential savings remain unknown. |

For context, Windows exact-master `260b3b651` thread counters gave about 6.52–7.07 ms main and 6.09–6.26 ms render CPU indoors, versus 5.49–5.95 and 4.79–5.32 ms at the exit. Those use different hardware, runtime, revision, GPU load and instrumentation; they do not isolate a Linux-specific cost. On Godot's source defaults, frame queue size is two and the requested swapchain image count is three, clamped to surface capabilities; the actual image count was not captured. `RenderingServer::sync()` waits for queued render work before each draw. The extension's `texture_get_rd_texture` lookup is dirty-gated rather than unconditional per frame.

Evidence: `data/diagnostics/fps-120-linux-2026-10-31-cage-profile-noscreenshot/` (perf data, relative per-thread reports, scheduler and GPU samples, and fixture logs), `data/diagnostics/fps-120-linux-server/` (profile binaries), and `data/diagnostics/fps-120-2026-10-30/windows-qualification-master-exact/`. GPU time remains unqualified under shared load.

### Static scenery candidate grid: loaded-host A/B

The index evaluates short-range static placements from a 100-yard horizontal grid and retains the prior candidate set for visibility transitions. Animated and long-range placements still run each frame. Three interleaved A/B pairs used the same non-shipped symbolized Godot and release `debug=1` extension at the two poses, 1920×1080 Cage/Vulkan FIFO 144 Hz and Forever. The A extension was built from `05ae361ae`; B adds the scenery-index series, now landed as `d3131172b` through `a8a435df9`. The test predates later Godot, M2 and game-object changes on master. Each run completed three 300-frame records; the diagnostic runtime logged skeleton-disconnect and render-device-finalization errors and did not emit a completion marker.

| Pose | Pair | Main CPU ms/frame A → B | Render CPU ms/frame A → B | Agents.slice cores A → B |
|---|---:|---:|---:|---:|
| Indoor | 1 | 18.18 → 14.33 | 18.01 → 15.48 | 26.8 → 23.9 |
| Indoor | 2 | 21.73 → 20.14 | 22.42 → 22.03 | 30.5 → 22.5 |
| Indoor | 3 | 18.99 → 20.43 | 20.17 → 22.23 | 30.8 → 29.8 |
| Exit | 1 | 11.57 → 18.65 | 10.28 → 19.05 | 12.2 → 30.2 |
| Exit | 2 | 9.45 → 17.66 | 8.12 → 17.30 | 11.3 → 27.6 |
| Exit | 3 | 17.63 → 15.32 | 16.71 → 14.75 | 28.7 → 31.1 |

Values are medians of each run's three samples. Other GPU clients varied between one and three in the before/after snapshots. The loaded-host results do not establish a whole-frame improvement: CPU load and render time changed substantially between variants. Three-second CPU-clock samples estimate `cull_doodads` self CPU indoors at 1.64/2.07/1.74 → 0.49/0.83/0.57 ms and at the exit at 1.66/0.96/1.78 → 0.78/0.65/0.69 ms. These are approximate sample-share × main-CPU estimates, not measured total savings; index work can be inlined into that symbol.

All three pairs produced identical in-tree visibility and effective shadow-mode receipts for **16,691 real doodads per pose**. First-pair 1080p screenshots, inspected side by side, showed no obvious missing scenery; they do not prove full motion or pixel identity. Synthetic tests additionally cover movement across grid and fade boundaries, far spawns, and more than 64 placements. Evidence: `data/diagnostics/fps-120-linux-grid-{before,after}-{1,2,3}/`, `data/diagnostics/fps-120-linux-grid-comparison/`, and `scripts/tests/scenery_visibility_compare.py`. A fresh check against current master remains required.

A later 499 Hz CPU-clock profile on `05ae361ae` found `std::thread::current` at 4.41%/4.52% of main CPU indoors/at the exit, plus `ensure_main_thread` at 3.84%/3.88%. This differs from the earlier cycle-weighted estimate; neither attributes the call sites reliably. Evidence: `data/diagnostics/fps-120-linux-2026-10-31-cage-profile-hires/`.

## Linux Xvfb diagnostic (2026-10-31)

On agent-server, engine `93ac8d4b5` was built in the release profile with shared-protocol `5fe2ff2` to match a private server binary from the `game-server-neltharus-current` checkout. That checkout was at `82499270`; the binary's source revision was not independently established. The Windows receipt fixture ran with the Forever skin, an NVIDIA RTX 5060 Ti and a 1920×1080 Xvfb X11 surface. Each row is three 300-frame samples after 8,059 objects loaded with zero reported failures. Xvfb is not a native Wayland or Cage measurement.

| Pose | FPS, three samples | Mean main-thread CPU time/frame |
|---|---|---|
| Indoor | 9.40 / 9.52 / 9.34 | 23.16 / 21.78 / 21.50 ms |
| Stockade exit | 11.93 / 16.02 / 17.80 | 16.92 / 12.48 / 10.45 ms |
| Indoor repeat | 8.04 / 11.55 / 12.49 | 23.18 / 19.42 / 17.82 ms |

Two other Godot clients used about 4 GiB of the 8 GiB GPU before and after the first two poses; a third was present after the repeat. GPU contention during the samples was not recorded. These numbers do not isolate rendering from Xvfb presentation or host contention and do not establish the native Wayland FPS. GPU timestamp evidence is still pending. Xvfb startup reported a mailbox-to-enabled VSync fallback; the fixture later reported its requested disabled mode, which does not establish the actual Vulkan present mode.

A separate Cage attempt used the release client with Vulkan and failed the 1920×1080 fixture check before world entry. It also logged that disabled VSync was unavailable and fell back to enabled. It provides no FPS measurement. Evidence: `data/diagnostics/fps-120-linux-2026-10-31-xvfb-frozen/` and `data/diagnostics/fps-120-linux-2026-10-31-retry1/`.

## Windows exact-revision comparison (2026-10-31)

Four Windows release builds were measured consecutively in one desktop session at 1920×1080 with the Forever skin. Each pose used three 300-frame samples after world loading settled; the indoor view was repeated. The exit screenshot looks down the Stockade stairs and does not establish a view of outdoor content.

| Revision | Indoor FPS | Stockade exit FPS | Indoor repeat FPS |
|---|---|---|---|
| `d23d8d8f6` — before split traversal | 136.38 / 136.62 / 130.52 | 166.74 / 157.85 / 164.77 | 132.53 / 136.84 / 129.59 |
| `4b04d5dcb` — split traversal | 130.91 / 137.57 / 130.44 | 167.25 / 165.77 / 176.39 | 126.61 / 136.25 / 139.00 |
| `260b3b651` — master with split traversal and particle RD cache | 140.17 / 148.39 / 143.20 | 178.06 / 172.21 / 168.84 | 148.45 / 149.37 / 145.01 |
| `f4c272add` — other-WMO admission prototype | 140.24 / 139.10 / 140.02 | 171.43 / 169.24 / 178.77 | 145.47 / 139.28 / 144.25 |

The WoW process was absent before and after every run. During the indoor samples, median GPU utilization was 37% / 37% / 36% / 38% and GPU memory was 3,878 / 3,885 / 3,886 / 3,886 MiB, respectively. At the exit these were 40% / 39% / 39% / 39% and 3,885 / 3,885 / 3,879 / 3,886 MiB. The first three builds have identical WMO batch path, tree-visibility and shadow-mode receipts at each pose: 2,990 camera-mode visible batches indoors and 2,983 at the exit, from 4,597 total. The prototype has 727 / 723 camera-mode visible and 2,227 / 2,224 shadow-only batches. All placement receipts match at 16,816 rows. Tree visibility does not prove GPU frustum submission. The split change added no visible groups at these poses, and neither split nor the particle-cache master shows the earlier large FPS drop.

**Decision:** Drop the unlanded other-WMO prototype. At these poses it reduced draw calls but produced no measured FPS gain over master: median FPS was 140.02 versus 143.20 indoors and 171.43 versus 172.21 at the exit. The measured `Culling` step rose from a median 1.548 to 1.727 ms indoors and from 1.416 to 1.596 ms at the exit. The builds also differ in other code, so this comparison does not prove that WMO admission alone caused the difference. The prototype evaluates outdoor admission per WMO batch on each culling pass before its state-change check; it is not a candidate to land on these results. The inspected screenshots at the two poses show no obvious missing content, but do not prove pixel identity, all doorway views or shadow equivalence.

An earlier prototype capture measured only 66–69 FPS indoors and 74–80 at the exit, but WoW was open and sampled GPU memory was near 9 GiB. The earlier 137–139 FPS portal-only capture had no WoW window and sampled about 3.5 GiB. This load difference confounds that comparison; it does not prove a cause. No user applications were closed. Evidence: `data/diagnostics/fps-120-2026-10-30/windows-qualification-{before-split-exact,split-exact,master-exact,candidate-exact}/`, `windows-exact-comparison.json`, and `windows-qualification-particle-split/`.

## Windows fat-LTO trial (2026-10-30)

A Windows release build with `lto = "fat"` and `codegen-units = 1` was compared at 1080p with the default release profile, both including the UI-errors sync change. Three 300-frame samples were 121.17/121.18/120.34 FPS indoors and 168.29/165.58/179.00 at the exit with the default profile; fat LTO produced 122.06/123.17/120.09 and 174.09/170.31/167.51. Median gains were 0.89 and 2.02 FPS, within run-to-run spread. Placements had matching paths and sources but differed in height, and draw counts differed, so the comparison does not isolate LTO. The profile change forced dependency rebuilds and took 1,110.65 seconds; the 137-second default-profile UI build was incremental and is not a cold-build comparison.

**Decision:** Do not adopt fat LTO now. Retry only after structural rendering fixes if frame-time margins remain thin. Evidence: `data/diagnostics/fps-120-2026-10-30/windows-qualification-ui/`, `windows-qualification-lto/`, and `landing/windows-lto/`.

## Linux 120 FPS status and remaining CPU costs (2026-10-31)

The target remains 120 FPS at both Stormwind poses at 1080p. The static scenery grid, NPC material animation LOD, and disabling Godot's unused viewport physics picker have landed. The existing explicit unit-picking rays remain active. The picking fixture passed unit selection, wall occlusion, a direct physics ray, and a Control click. Three loaded A/B pairs showed fewer sampled broad-phase segment-cull stacks in all six pose comparisons (indoor 11→7, 15→11, 15→13; exit 18→8, 19→12, 11→4); 1,304–2,040 samples per run make those small counts noisy. Frame time did not establish a gain under varying shared GPU and host load. Raw evidence: `data/diagnostics/fps-120-linux-picking-{before,after}-{1,2,3}/`.

An eight-second CPU-clock profile after the grid and material changes, but before the picking change, identifies the next costs. It used a frame-pointer release extension and a **non-shipped, older symbolized Godot runtime**, with shared GPU and host contention; its shutdown logged skeleton disconnect and render-device finalization errors. It is attribution evidence, not a shipped-runtime FPS qualification. The indoor render thread previously measured about 8 ms CPU per frame in a different loaded sample; do not add overlapping thread times or assume that value still holds.

1. **Render thread — render-list and draw submission:** 860 of 2,483 indoor render-thread samples include `_render_list`; 222 include vertex-array binding and 211 include Vulkan vertex-buffer binding. These counts overlap. Reducing actual submitted surfaces and state changes, not merely scene nodes, is the leading structural opportunity.
2. **Render thread — material uniform updates:** 68 of those samples include `_update_queued_materials` and 63 include `update_parameters_uniform_set`; these overlap. Identify the changing materials and their causes before changing shader or material sharing.
3. **Main thread — scene transforms, scenery cull, and animation:** top sampled self functions include `Node3D::_propagate_transform_changed`, `cull_doodads`, transform notification flushing, Godot FFI thread checks, and M2 rotation/position sampling. The scenery and material changes have not yet been isolated in a quiet shipped-runtime comparison.
4. **Physics:** the separate Godot viewport ray was removed. The remaining explicit selection, camera, and collision rays still need attribution before further changes.

The exit render profile similarly contains 826/2,435 samples in `_render_list` and 55/2,435 in `_update_queued_materials`. The fixture reports 3,632–3,635 indoor draws; it does not classify these by pass or material. See [client performance techniques](client-performance-techniques.md) for source-backed portal-window admission, compatible-surface consolidation, and shared rigid-instance candidates; none is yet proven to reduce these particular submissions.

For uniform updates, the exit main-thread stacks include both `WaterMaterials::sample_clock` → `LiquidSurface::set_time` and `SceneryFade::set_opacity`. Every liquid surface receives the same clock but writes its own `animation_time_ms` each frame (`godot/rust/src/terrain/water.rs`). A shared global **float** is a candidate: Godot's scalar-global path marks a shared buffer dirty, whereas its global-texture path queues dependent materials (`servers/rendering/renderer_rd/storage_rd/material_storage.cpp`). Per-material animated texture frames must still update when they change. Measure the affected liquid count and rendered impact first; existing water pixel and clock tests read the current material parameter and would need an equivalent observable assertion. Per-instance scenery opacity is a separate candidate with fade, shared-material and shadow behavior to preserve. Neither candidate has a measured A/B.

Evidence and per-thread reports: `data/diagnostics/fps-120-linux-2026-10-31-physics-dwarf/`.

### Shipped master baseline under load

At master `4c9584c99`, a release extension and the checksum-verified pinned `4.7.2-pr123946-pr123546-pr123391-matlist-bonepose` template ran at 1920×1080 in Cage with Forever. Each pose used three 300-frame samples; the private server used `392a3ccd74` / shared protocol `cff29ac`. Both clients printed their completion marker and exited cleanly. GPU and host were shared. `schedstat` values are interpolated across one-second observations; the main and render threads overlap and their CPU times must not be added.

| Pose | FPS, three samples | Main CPU ms/frame | Render CPU ms/frame, estimated |
|---|---|---|---|
| Indoor | 40.90 / 19.15 / 15.88 | 15.49 / 24.61 / 24.23 | 17.14 / 29.57 / 29.79 |
| Exit | 24.80 / 21.39 / 21.14 | 20.36 / 17.27 / 17.37 | 20.82 / 17.64 / 17.78 |

One-minute host load during the samples was 32–47 indoors and 52–59 at the exit. The morning query-free baseline had indoor 57.31 / 57.99 / 54.46 FPS and exit 46.90 / 70.74 / 74.15 FPS, with load 14–23; the server and engine revisions also changed. These loaded runs do not establish a regression or improvement. Evidence, GPU process samples and receipts: `data/diagnostics/fps-120-linux-master-4c-shipped/` and `data/diagnostics/fps-120-linux-2026-10-31-cage-query-free/`.

Within `GameClient.process` wall-time spans, the indoor shares across the three samples were Culling 29–31%, World units 10–12%, World hover 9–11%, Spell visuals 9%, and World lighting 6%. At the exit they were 27%, 11–12%, 9–10%, 9%, and 6–7%, respectively. These are wall-time shares under load, not CPU attribution; node callbacks overlap the client span. The shipped Godot and extension are stripped, so this capture cannot name render-thread hot functions. The older symbolized profile above is a separate attribution, not proof of the exact shipped build. Culling is the largest measured client step; whether its WMO portal work, doodads, particles, or unit LOD dominates still needs current-build attribution.

### Batched shipped master run under a limited quiet window

The same `4c9584c99` release extension and checksum-verified pinned Godot template ran in six independent clients, alternating the indoor and exit poses three times. Each client recorded three 300-frame samples at 1080p, Forever. The admission window prevented new builds but did not stop existing work or reserve the GPU. The batch took 6m14s; the window was cleared when it ended. At launches, one-minute load was 21.96 / 21.75 / 14.34 indoors and 22.97 / 16.57 / 12.84 at the exit. GPU samples showed 2–4 non-Cage graphics processes including this client.

| Pose | Median FPS for each of three launches | Main CPU ms/frame, sample range | Render CPU ms/frame, estimated range |
|---|---|---|---|
| Indoor | 32.92 / 50.94 / 51.63 | 8.65–19.86 | 7.70–21.69 |
| Exit | 83.70 / 71.12 / 88.04 | 6.32–8.58 | 5.39–7.74 |

Across the nine samples per pose, the median wall-time shares within `GameClient.process` were Culling 31.7% / 29.8%, World hover 11.5% / 13.2%, World units 9.2% / 8.1%, Spell visuals 8.9% / 9.6%, and World lighting 6.5% / 7.9%, indoor / exit. These spans are not isolated CPU time; render-thread and main-thread times overlap. The scene remains below 120 FPS. Changes in load, GPU sharing, engine, and server make the morning baseline unsuitable for a causal comparison. The shipped binaries are stripped, so the prior symbolized profile cannot be treated as exact current-build attribution. Evidence: `data/diagnostics/fps-120-linux-master-4c-quiet/`.

A concrete culling candidate at this revision is duplicate portal work: `TerrainObjects::cull_wmos` requests `exterior_frusta` and then `visible_groups`; for an interior camera, `visible_groups` calls `exterior_frusta` again. This performs the exterior traversal twice and interior-group lookup three times for that WMO. Computing the visible groups and exterior windows together could remove the duplicate while retaining per-frame updates. Its share of the measured Culling step and its impact are not established. Any change needs portal/window behavior tests and matching real-scenery visibility and shadow receipts.

### Liquid clock and draw identities

The global liquid clock landed as `99d4f7c4c` / `90e914106`. Pinned Vulkan fixtures passed the one-hour clock wrap and water, eight other liquid, and borrowed-water pixel checks. Three loaded, interleaved A/B pairs used the same older non-shipped symbolized Godot and matched before/after release builds. FPS varied substantially with contention, so the runs do not establish a frame-time gain. Evidence: `data/diagnostics/fps-120-liquid-fixtures2/` and `data/diagnostics/fps-120-linux-liquid-{before,after}-{1,2,3}/`.

A subsequent one-frame Forward+ draw trace used an older instrumented Godot in **safe**, not the shipped separate-thread, render mode. It found 2,425 color and 1,057 shadow submissions indoors, and 1,954 color and 792 shadow submissions at the exit. Exact same-mesh/material/state, non-skinned doodad color repetitions involved 46 draws in 10 groups indoors and 45 in 12 groups at the exit. Their mathematical maximum reductions are 36 and 33 draws, before visibility, fade, animation, and shadow constraints. Skinned repetitions are not proof of rigid eligibility. Shadow repetitions may include different cascades, not distinct instances. Instancing is not justified by this census (lead decision); no FPS gain is claimed. Full per-pass groups and same-process material-signature matching: `data/diagnostics/fps-120-linux-draw-census/`.

## Sources

- `src/rendering/camera/culling.rs`: the original doodad, WMO and portal culling. Its 200/2,000/400 yd distances cite no source (commit `f475000f`), so they were not used.
- `src/rendering/model/animation/lod.rs`: the original NPC LOD thresholds.
- `~/Repos/solarityclient/crates/runtime/src/application/m2_spatial.rs`: build 12340 scenery size classes and radii.
- [solarityclient-performance-comparison](solarityclient-performance-comparison.md): earlier notes on scenery distance fade and cull by size class.

## See Also

- [[client-performance-techniques]]: source comparisons and unmeasured rendering candidates.
- [[solarityclient-performance-comparison]]: the reference client's culling pipeline.
- [[movement-performance]]: Bevy-side CPU measurements and the A/B method.
- [[stormwind-hilly-plaza]]: why the portal traversal ignores antiportals and requires a floor below the camera.
