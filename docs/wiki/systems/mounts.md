# Mounts and steady flight (Godot)

Player mounts and Retail "Steady Flight" on the Godot client. Server rules (mount aura, MountCapability, flight speed, can-fly): game-server `docs/wiki/systems/mounts.md`.

## Mount model

`godot/rust/src/world_mount.rs`. A player's replicated `Mounted::mount_display_id` loads that creature display as a model under the unit's node; the unit's own visual is reparented onto the mount's MountMain point (M2 attachment 0, `NpcModel/Skeleton3D/AttachmentBone0/Attachment0`), so it follows the saddle bone. `seat_rider` runs every frame, so a re-dressed rider remounts; a dismount seats the visual back on the unit node and frees the mount. Creature (NPC) mounts from `creature_addon` are not drawn yet.

Animation: the rider holds Mount (91). The mount plays the rider's locomotion clip on the ground; while flying (`PlayerMotion::FLYING` for other players, `PlayerMovement::flying` for the local one) its flight clips: MountFlightIdle 548, MountFlightRun 556, MountFlightBackwards 550, MountFlightLeft 552, MountFlightRight 554 (resolved through `AnimationData.Fallback`).

## Flight controls

`godot/rust/src/gameplay.rs`, after Retail Bindings_Standard.xml:53-65: `JUMP` = `JumpOrAscendStart`, `SITORSTAND` = `SitStandOrDescendStart`.

- The client may fly while its replicated `PlayerMotion` has `CAN_FLY` (`set_can_fly`); losing it ends a flight and the player falls.
- Take off: Space jumps; with Space still held in the air the player starts flying.
- Flying: no gravity; held Space ascends and X descends at `FLIGHT_SPEED` × the server's aura modifier; with the right mouse button (MOVEANDSTEER) forward movement follows the camera pitch, as swimming does; total velocity is capped at the flight speed (the server grants the 3D distance).
- Landing: touching the ground (not ascending) clears `flying`; reaching swimming depth turns into a swim. X does not sit while flying.
- `PlayerInput.flying` reports the flight; a vertical-only climb or descent reports like a swimmer's.
- Using the mount spell while riding it sends `CancelMountAura` (dismount). `use_spell(id)` (`#[func]`) casts through that same path for fixtures.

## Proof

- Depot unit tests (`gameplay::tests`): takeoff/climb at flight speed, hover without gravity, X descent and landing input, mouse-steered pitch, fall after losing `CAN_FLY`, Space without `CAN_FLY` only jumps.
- Live `godot/tests/flying_mount_live.gd` (2026-10-02, private server UDP 5179, game-server `4ac0e91`, character Fbflymount with Expert Riding 34090 and Golden Gryphon 32235 via `game-server-admin learn-spell`): mounts (display 17697, rider on attachment 0), climbs 30 yd in 1.66 s, flies forward, hovers 2 s with no height change, server height within one frame of the client, lands at the terrain height (server speed back to mounted run 14), dismounts. Screenshots and traces: `data/diagnostics/flymount-2026-10-02/`. The client needs ≥ 4 fps: the server applies at most 0.25 s of movement per input; an earlier run at 2 fps (loaded host) left the server half a flight behind.
