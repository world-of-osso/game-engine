# Godot Stormwind FPS

The Godot client ran Stormwind at about 13–20 FPS (debug build, 1280x720, headless cage, Vulkan, AMD 890M), with about 6,400–7,400 draw calls. Hiding categories in a probe showed where the cost was. Draw calls were a real cost, but per-frame animation processing was a larger one. Three changes, each taken from the original client or from retail, bring the same views to about 20–25 FPS and about 4,300–5,000 draws. A fourth change (doodad animation LOD plus manual skeleton processing) brings the indoor view to about 31–38 FPS. The rest of the cost is the draws that remain.

## Method

- The probe is an untracked scratch script, `godot/tests/_probe_draws.gd`. It enters the world as agent account `fb_fps`/`fbtest`, character `Fbfps`. It waits until every object has spawned (8,059 objects, 0 failures). It then averages the monitors over 30 frames for each variant: `Engine.get_frames_per_second` and the `RENDER_TOTAL_DRAW_CALLS/OBJECTS/PRIMITIVES_IN_FRAME` monitors.
- For each A/B pair, the "before" variant is forced from GDScript in the same session:
  - Doodads: every root is set visible and its `M2Animation` is set processing. The cull only writes when its cached state changes, so these forced values stay.
  - WMO groups: every batch is re-shown from `RenderingServer.frame_pre_draw`.
- Other agents share the host and GPU (load 6–20). One variant varied by up to ±5 FPS between runs, so this page reports ranges. Only differences seen in every run are claimed.
- Two spots:
  - Indoors: Fbfps at (-8785.9, 820.7, 97.65).
  - Stockade exit: (-8766.11, 845.50, 88.00), with the outdoors in view.

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
   - Retail's 5/10/15/20/50 yd fade band before the far radius is reproduced ([doodad-scenery-distance](../../specs/doodad-scenery-distance.md)): a fading doodad's opaque batches switch to a blended variant of their shader, and switch back at opacity 1, so fully opaque doodads stay in Godot's opaque pass.
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

## Sources

- `src/rendering/camera/culling.rs`: the original doodad, WMO and portal culling. Its 200/2,000/400 yd distances cite no source (commit `f475000f`), so they were not used.
- `src/rendering/model/animation/lod.rs`: the original NPC LOD thresholds.
- `~/Repos/solarityclient/crates/runtime/src/application/m2_spatial.rs`: build 12340 scenery size classes and radii.
- [solarityclient-performance-comparison](solarityclient-performance-comparison.md): earlier notes on scenery distance fade and cull by size class.

## See Also

- [[solarityclient-performance-comparison]]: the reference client's culling pipeline.
- [[movement-performance]]: Bevy-side CPU measurements and the A/B method.
- [[stormwind-hilly-plaza]]: why the portal traversal ignores antiportals and requires a floor below the camera.
