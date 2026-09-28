# Camera collision recovery lag

While the player ran, the in-world follow camera fell behind and caught up only after movement stopped. Reported in the Godot client on 2026-09-28; the defect is in the shared follow math (`src/camera_follow_data.rs`) that the Bevy client also uses.

## Root cause

Collision recovery lerped from the *current camera position's* distance to the moving eye target toward the orbit distance, and recovery ended only within 0.05 m of that distance. Follow smoothing already trails a running player by about run speed / follow speed (0.5–0.7 m), so after any pull-in (a mesh ray hit or terrain sample between eye and camera, even for one frame) that distance stayed above the orbit distance while the player kept running. Recovery never finished, and each frame the camera moved `follow_t * recovery_t` of the gap (dt², ~0.07 of the player's 0.12 m step at 60 Hz). At higher frame rates this is worse.

## Fix

`fe9faa73`: `CameraState::collision_distance: Option<f32>` (replacing `collided`) stores the pulled-in orbit distance; recovery lerps from it toward `distance` and clears near it, independent of player motion.

## Evidence

- `godot/tests/world_camera_follow_flow.gd` (dev server `127.0.0.1:5000`, `--fixed-fps 60`, `GODOT_TEST_CARD=1`): held W for 120 frames with a one-frame physics blocker at frame 40. Before: camera lag grew to 5.249 m (still growing). After: 0.553 m, same as the unobstructed run (0.578 m).
- `godot/core/tests/camera_data.rs::collision_recovery_keeps_following_a_player_running_away`.
- Not established: which in-world geometry triggered pull-ins in the user's session, or why it was first noticed on 2026-09-28.
