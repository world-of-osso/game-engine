# NPC animation LOD

Replicated NPC models sample their bone animation at a rate chosen from camera distance and last frame's frustum result. The `M2AnimPlayer` clock, sequence selection, and crossfades keep running every frame; only Bevy clip sampling and joint writes are skipped. The server interest radius (100 yd) is unchanged.

## What it must do

- [ ] NPC models within 30 yd of the camera and on screen sample every frame.
- [ ] NPC models between 30 yd and 60 yd and on screen sample every other frame, staggered by entity so roughly half sample each frame.
- [ ] NPC models beyond 60 yd, or with no mesh visible in the previous frame, do not sample; their joints keep the last written pose.
- [ ] A skipped frame writes no joint transform, so the NPC's transform subtree is not dirtied.
- [ ] Models not parented to an NPC visual root (the local player, doodads, debug scenes) are never rate-limited.
- [ ] Without a `WowCamera` in the world, no model is rate-limited.
- [ ] Applies only in `GameState::InWorld`.

## How it works

- [animation](../wiki/systems/animation.md#npc-animation-lod).

## Implementation inventory

- `src/rendering/model/animation/lod.rs`: thresholds, `AnimationLod`, per-frame assignment from `WowCamera` distance and child `ViewVisibility`.
- `src/rendering/model/animation/bevy_player.rs`: `sync_m2_animation_players` leaves clips stopped on skipped frames.
- `src/game/networking/npc.rs`: `NpcVisualRoot` marker on the replicated NPC's visual root.

## Tests asserting this spec

- `tests/unit/animation_tests/lod.rs`: threshold table, alternate-frame sampling, and App-level joint-write behavior for near, mid, far, off-screen, non-NPC, and camera-less cases.

## Known gaps

- Frozen NPCs that are moving slide in their last pose until they re-enter range or view.
- Thresholds are constants; no client option exposes them.

## Out of scope

Server interest radius, Bevy animation graph size, transform propagation worker changes, player model LOD.
