# Swimming

Godot local swimming: `PlayerMovement` switches from the walk path (gravity, slope, jump) to a swim path once terrain water stands `SWIM_DEPTH` over the feet, and the Retail mirror timer overlay shows breath. Requirements: [swimming spec](../../specs/swimming.md).

## Movement

- `player_physics_data` (`src/`, shared into `godot/core`): `SWIM_DEPTH` 1.25, `is_swimming(feet_y, ground_y, surface)`, `at_swim_surface`, `swim_height(feet_y, rise, surface, ground, at_surface)`.
- `godot/rust/src/gameplay.rs`: `resolve(bindings, input, yaw, pitch)` picks `SWIM_SPEED` while `swimming` (last frame), tilts the forward part of the direction by camera pitch while the right mouse button is held, and puts Jump/SitOrStand into `MovementFrame::vertical`. `predict` runs `walk` or `swim`, then re-derives `swimming`; entering water zeroes vertical velocity and jumping.
- `swim`: level move through `TerrainGround::validate_swim_move` (WMO wall clamp; ground under the feet lifts them, ground beyond step reach blocks), then `swim_height` with `rise` = pitched travel + `vertical × SWIM_SPEED × dt`. `at_surface` keeps a floating swimmer on a surface that changes height between chunks.
- Wading in: the walk path keeps the feet on the ground until the water is 1.25 over them, which is exactly the floating height, so the switch has no pop. Leaving: the rising shore lifts the feet, the depth drops below 1.25, the walk path resumes grounded.
- `network_input` also sends a zero-direction input when the swim step changed height (`swim_rose`), so Space held at the surface stays quiet.

## Server

`game-server` `apply_movement_input` adopts only the reported x/z (speed-limited, `SWIM_SPEED` for `swimming` inputs); `apply_terrain_gravity` owns y and clamps to the ground. Live trace (`/tmp/claude/swimming-live-1.log`): client y 140.07 → 142.74 (surface) → 140.07 while the replicated server y stays 140.069. Other clients see a swimmer on the seabed. The local player is unaffected: `follow_server_motion` keeps the predicted pose until a new control epoch.

## Mirror timers

- `src/mirror_timer_data.rs`: `MirrorTimersData` (three frames), `MirrorTimerStart` mirrors `SMSG_START_MIRROR_TIMER`.
- `src/ui/screens/mirror_timer_component.rs`: `MirrorTimerContainer` rsx, atlas crops of FDID 4505182 via the unit-frame `AtlasArt`.
- `godot/rust/src/mirror_timers.rs`: `MirrorTimers` RegistryUi overlay, attached on the first start, ticked and shown only InWorld. GDScript: `start_mirror_timer(timer, value, max, scale, paused)`, `stop_mirror_timer(timer)`, `mirror_timer_fraction(timer)`.
- No server source: game-server and shared-protocol have no breath, drowning or mirror timer code.

## Sources

- `~/.cache/wow-ui-sim/blizzard-ui/Blizzard_FrameXML/Bindings_Standard.xml` — JUMP/SITORSTAND ascend/descend
- `~/.cache/wow-ui-sim/blizzard-ui/Blizzard_MirrorTimer/` — layout, atlases, event handling

## See Also

- [[player-ground]] — ground sampling the swim path reuses
- [[terrain]] — MH2O water surface sampling
- [[animation]] — Swim clip selection (41–45)
