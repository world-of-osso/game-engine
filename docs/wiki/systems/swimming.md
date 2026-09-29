# Swimming

Godot local swimming: `PlayerMovement` switches from the walk path (gravity, slope, jump) to a swim path once terrain water stands `SWIM_DEPTH` over the feet, and the Retail mirror timer overlay shows breath. Requirements: [swimming spec](../../specs/swimming.md).

## Movement

- shared-protocol `movement.rs` (the server's copy): `SWIM_DEPTH` 1.25, `SWIM_EPSILON`, `is_swimming(feet_y, ground_y, surface)`, `swim_top(surface)`. `godot/rust/src/swim.rs` builds `at_swim_surface` and `swim_height(feet_y, rise, surface, ground, at_surface)` on them; `TerrainGround::swimming` calls `is_swimming`.
- `godot/rust/src/gameplay.rs`: `resolve(bindings, input, yaw, pitch)` picks `SWIM_SPEED` while `swimming` (last frame), tilts the forward part of the direction by camera pitch while the right mouse button is held, and puts Jump/SitOrStand into `MovementFrame::vertical`. `predict` runs `walk` or `swim`, then re-derives `swimming`; entering water zeroes vertical velocity and jumping.
- `swim`: level move through `TerrainGround::validate_swim_move` (WMO wall clamp; ground under the feet lifts them, ground beyond step reach blocks), then `swim_height` with `rise` = pitched travel + `vertical × SWIM_SPEED × dt`, clamped to `±SWIM_SPEED × speed_modifier × dt`: game-server `move_toward_reported` grants vertical travel at that rate only (`max(horizontal/speed, |Δy|/swim speed)` from the movement bank), so a pitched climb with Space held (up to ~1.5–2× before) arrived late on the server. `at_surface` keeps a floating swimmer on a surface that changes height between chunks.
- Wading in: the walk path keeps the feet on the ground until the water is 1.25 over them, the floating height. At low frame rates one walk step crosses the threshold past it (the loopback fixture at ~24 fps then sent a zero-direction input for idle Space as the first swim step moved the feet onto the float height); `enter_water` puts a player that was walking last frame at the float height, never below the seabed (on a seabed up to 0.01 above it, the feet stay on the seabed). A spawn (no previous frame) or a fall keeps its depth. Leaving: the rising shore lifts the feet, the depth drops below 1.25, the walk path resumes grounded.
- `network_input` counts a swim step that changed height (`swim_rose`) as motion in `report()`, so vertical-only swimming sends zero-direction inputs, its end sends the one stop, and Space held at the surface then stays quiet. `MovementFrame::vertical` is `±SWIM_SPEED × speed_modifier`, so snares and roots apply to it.

## Server

game-server `869ce47` adopts the reported height within swim bounds (`[floor, swim_top]`, vertical rate `SWIM_SPEED` × aura) and land bounds; see game-server `docs/specs/player-height.md`. Live (`swimming_live.gd`, 2026-09-28): the replicated height reaches the client's within ~90–190 ms at the surface and on the seabed. Remote players lerp to the replicated `Position` including y (`interpolate_remote_motion`), so other clients see the swimmer's real depth. Once (1 of 4 runs) the server stopped 0.08 yd short of the surface after a slow first rise; not reproduced.

## Mirror timers

- `src/mirror_timer_data.rs`: `MirrorTimersData` (three frames), `MirrorTimerStart` mirrors `SMSG_START_MIRROR_TIMER`.
- `src/ui/screens/mirror_timer_component.rs`: `MirrorTimerContainer` rsx, atlas crops of FDID 4505182 via the unit-frame `AtlasArt`.
- Server: game-server `e489332` sends `MirrorTimerStart`/`Pause`/`Stop` (shared-protocol `5872376`) on `MirrorTimerChannel`: breath 180 s at scale -1 when the surface is more than 2.03128 yd over the feet, refill at scale 10 and stop when full, drowning every 1 s at 0 (game-server `docs/specs/breath-and-drowning.md`).
- `godot/network`: `BridgeConfig::receive_mirror_timers` relays the three types from one system sorted by message id; per-type relays would deliver a frame's messages in system order (a stop before its start).
- `godot/rust/src/account.rs` → `AccountEvent::MirrorTimer` → `mirror_timers.rs` `apply_mirror_timer_message` (timer 0/1/2 → Fatigue/Breath/Feign Death, unknown fails). The `MirrorTimers` RegistryUi overlay is attached on the first message, ticked and shown only InWorld. GDScript reads only `mirror_timer_fraction(timer)`; nothing in GDScript starts or stops a bar.
- Live: the bar appears ~0.6–1 s after the head goes under, drains 0.00552/s against the server's 1/180 = 0.00556/s, and hides after surfacing. Drowning is unproven: with `SWIM_DROWN=1` the bar sat at 0 for 10 s while the replicated health stayed 292 (twice); the server log records no damage either way.

## Sources

- `~/.cache/wow-ui-sim/blizzard-ui/Blizzard_FrameXML/Bindings_Standard.xml` — JUMP/SITORSTAND ascend/descend
- `~/.cache/wow-ui-sim/blizzard-ui/Blizzard_MirrorTimer/` — layout, atlases, event handling

## See Also

- [[player-ground]] — ground sampling the swim path reuses
- [[terrain]] — MH2O water surface sampling
- [[animation]] — Swim clip selection (41–45)
