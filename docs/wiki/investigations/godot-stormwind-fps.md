# Godot Stormwind FPS

The Godot client ran Stormwind at about 13–20 FPS (debug build, 1280x720, headless cage, Vulkan, AMD 890M), with about 6,400–7,400 draw calls. Hiding categories in a probe showed where the cost was. Draw calls were a real cost, but per-frame animation processing was a larger one. Three changes, each taken from the original client or from retail, bring the same views to about 20–25 FPS and about 4,300–5,000 draws. The rest of the cost is doodad animation for doodads still drawn, and the draws that remain.

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
   - Retail's 5–50 yd fade band is not reproduced: a doodad stays opaque until its far radius.
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

## Remaining cost (after)

- **Doodad animation for drawn doodads:**
  - The drawn doodads are 1,387–1,440 processing `M2Animation` nodes, plus 346 `M2MaterialAnimation` nodes that are never culled.
  - Turning `WorldObjects` processing off and on in the same session gave 20.7/21.0/21.1 FPS off versus 14.4/11.3/16.4 on, about 20 ms per frame.
  - This is not the Rust sampling: static poses already skip their writes. The cost is in the engine, from skeleton and skin updates plus per-node process calls, which matches the eu-stack samples of the stripped Godot binary.
  - The original client never rate-limits doodads ([npc-animation-lod](../../specs/npc-animation-lod.md): "doodads … are never rate-limited"). Freezing off-screen doodads, or not animating static doodads at all, would therefore be a new rule.
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
