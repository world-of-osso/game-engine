# NPC and doodad animation LOD

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Replicated NPC models and placed doodads sample their bone animation at a rate chosen from camera distance and whether they are on screen. The animation clock, sequence selection, and crossfades keep running; only pose sampling and joint writes are skipped. The server interest radius (100 yd) is unchanged. Both clients (Bevy and Godot) share these rules.

## What it must do

- [x] Replicated NPC attachment supplies the real animation runtime and exact display skin textures without adding a player marker or default equipment. Idle playback changes actual joint transforms.
- [x] M2 +X model forward maps to logical network +Z forward below the NPC root; display scale is retained.
- [ ] NPC models within 30 yd of the camera and on screen sample every frame.
- [ ] NPC models between 30 yd and 60 yd and on screen sample every other frame, staggered by entity so roughly half sample each frame.
- [ ] NPC models beyond 60 yd, or whose model bounds lie outside the camera frustum in the previous frame, do not sample; their joints keep the last written pose. The frustum test is the doodads' box-vs-frustum test, so the decision does not depend on renderer state and a headless client classifies NPCs as a rendered one does.
- [x] A new binding samples until Bevy has evaluated it once, so an NPC frozen since spawn holds an authored pose, not the bind pose.
- [ ] A skipped frame writes no joint transform, so the NPC's transform subtree is not dirtied.
- [ ] Models that are neither NPCs nor doodads (the local player, other players, debug scenes) are never rate-limited.
- [x] Doodads use the same distance bands and rates as NPCs, measured from the camera to the doodad (Godot: its transformed render-box center).
- [x] A doodad that is not drawn (beyond its scenery distance) or not on screen does not sample, as in the build-12340 reproduction, which culls static placements from their bounds before advancing their playback (solarityclient `crates/runtime/src/application/terrain_frame/m2.rs`, "ADT/WMO placements were culled from compact immutable bounds before touching instance state").
- [x] A doodad that starts sampling again shows the pose for the current clock time: no restart from the sequence start and no replay of the skipped time.
- [x] A doodad's material (texture and color) animation follows the same sampling rule as its bones and is at the current clock time whenever it samples.
- [ ] Without a `WowCamera` in the world, no model is rate-limited.
- [ ] Applies only in `GameState::InWorld`.

## How it works

- [animation](../wiki/systems/animation.md#npc-animation-lod).

## Implementation inventory

- `src/rendering/model/animation/lod.rs`: thresholds, `AnimationLod`, per-frame assignment from `WowCamera` distance and descendant `ViewVisibility`, including meshes beneath a grounded model root.
- `src/rendering/model/animation/bevy_player.rs`: `sync_m2_animation_players` leaves clips stopped on skipped frames.
- `src/game/networking/npc.rs`: `NpcVisualRoot` marker on the replicated NPC's visual root.
- `src/rendering/camera/culling.rs`: `Doodad` marker; Bevy doodads are rate-limited through it.
- `godot/rust/src/animation/lod.rs`: the Godot thresholds, `AnimationLod`, and `DeferredClock` (time owed to a doodad advanced only on sampled frames).
- `godot/rust/src/terrain/scenery.rs`: doodad drawn/on-screen/distance classification into `AnimationLod`.
- `godot/rust/src/terrain/objects.rs`: the in-world doodad cull advances bone and material animation on sampled frames.
- `godot/rust/src/world.rs`: NPC assignment in the Godot client: `npc_animation_lod` classifies the model's batch-mesh bounds with `SceneryDistance::box_in_frustum` and its camera distance.

## Tests asserting this spec

- `tests/unit/animation_tests/lod.rs`: threshold table, alternate-frame sampling, and App-level joint-write behavior for near, mid, far, off-screen, non-NPC, and camera-less cases, plus first-sample authored pose for NPCs frozen since spawn.
- `src/rendering/model/animation/lod_tests.rs`: nested grounded-root visibility plus near/mid/far and visibility-loss transitions.
- `src/game/networking/npc_animation_tests.rs`: actual sheep and HumanMaleHD attachment/playback, display texture pixels, logical-facing basis, and non-player/equipment isolation.
- `tests/unit/animation_tests/lod.rs` `doodads_follow_the_npc_distance_and_visibility_rates`: Bevy near, mid, far, and off-screen doodad joint writes.
- `godot/rust/src/animation/lod.rs`: thresholds, stagger, and `deferred_clock_resumes_at_the_every_frame_time`.
- `godot/rust/src/terrain/scenery.rs` `drawn_doodads_animate_at_the_shared_lod_rate`: near, mid, far, behind, beside, partly in view, and beyond scenery distance.
- `godot/rust/src/world.rs` `npc_animation_lod_samples_in_the_frustum_and_freezes_outside_or_far`: Godot NPC near, mid, far, behind, beside, and frustum-edge cases.

## Known gaps

- Frozen NPCs that are moving slide in their last pose until they re-enter range or view.
- Thresholds are constants; no client option exposes them.

## Out of scope

Server interest radius, Bevy animation graph size, transform propagation worker changes, player model LOD.
