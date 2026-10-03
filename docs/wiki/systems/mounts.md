# Mounts, steady flight and skyriding (Godot)

Player mounts, Retail "Steady Flight" and Skyriding flight physics on the Godot client. Server rules (mount aura, MountCapability, flight speed, can-fly): game-server `docs/wiki/systems/mounts.md`.

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

## Annotated tree contact

The flying movement path now consumes manually annotated tree contact with a fixed prototype sphere, tangential deflection and per-placement damped branches. The bounded tree prototype has separate native contact and mounted-input proof. See [[elastic-trees]] for authoring and limits.

## See Also

- [[elastic-trees]] — manual annotations, native flight contact and bounded proof.
- [[collision-system]] — terrain, WMO and camera collision boundaries.

## Skyriding (part 1: physics)

Model and constants: shared-protocol `src/skyriding.rs` (`Glider`, FlightCapability 11 from Retail DB2 12.1.0.69933). Server bound and the test-character switch (`game-server-admin flight-style <name> skyriding`): game-server mounts page.

- The client may skyride while its replicated `PlayerMotion` has `CAN_ADV_FLY` (`set_can_adv_fly`, from the Skyriding aura 406095); losing it ends the skyride and the player falls.
- Take off as for steady flight (Space jumps, Space held in the air): the mount launches straight up at 31.5 yd/s on top of its run and rises without lift (about 24 yards), then glides once falling with 7.5 yards of air under it.
- Gliding: no thrust; lift turns the velocity toward the facing and the pitch, gravity trades height for speed (dive faster, climb slower), air friction 1.5 yd/s², gravity stops at 65 yd/s. With the right mouse button the pitch follows the camera pitch (at most 180°/s); without it the pitch stays.
- Landing: coming down onto terrain or a WMO floor clears `flying`; swimming depth ends it into a swim. Every frame of a skyride reports (`PlayerInput.flying`); the server sets `FLYING | ADV_FLYING`.
- Not yet: vigor and Skyriding abilities (part 2), vigor UI, override bar and the Retail flight style toggle (part 3), banking, surface friction, the old-world 85% speed, Skyward Ascent as the real takeoff.

## Proof

- Depot unit tests (`gameplay::tests`): takeoff/climb at flight speed, hover without gravity, X descent and landing input, mouse-steered pitch, fall after losing `CAN_FLY`, Space without `CAN_FLY` only jumps.
- Live `godot/tests/flying_mount_live.gd` (2026-10-02, private server UDP 5179, game-server `4ac0e91`, character Fbflymount with Expert Riding 34090 and Golden Gryphon 32235 via `game-server-admin learn-spell`): mounts (display 17697, rider on attachment 0), climbs 30 yd in 1.66 s, flies forward, hovers 2 s with no height change, server height within one frame of the client, lands at the terrain height (server speed back to mounted run 14), dismounts. Screenshots and traces: `data/diagnostics/flymount-2026-10-02/`. The client needs ≥ 4 fps: the server applies at most 0.25 s of movement per input; an earlier run at 2 fps (loaded host) left the server half a flight behind.
- Skyriding Depot unit tests (`gameplay::tests`): launch apex 23.9 yd with no lift, glide, mouse-steered dive past 25 yd/s and landing; losing `CAN_ADV_FLY` falls; RED with the launch disabled.
- Live `godot/tests/skyriding_live.gd` (2026-10-03, private server UDP 5185, game-server skyride1 `006b750`, character Fbskyride set on the peak at WoW (-8793, 213, 382) with 34090, 32235 and `flight-style skyriding`): launch 24.2 yd, dive 405 → 239 yd at up to ~64 yd/s, pull-up 63.7 → 46.8 yd/s while climbing 25 yd, level glide, landing on the terrain with the server at the same spot. Server off the client's path at most 0.66 yd over 89 samples at ≥ 10 fps (2.09 yd during a 1-2 fps host stall, where the server's catch-up runs straight to the newest report); replication lag about 0.2 s. `data/diagnostics/skyride1-2026-10-02/` (run 6 final; earlier runs: low launch, WMO landing the old test took for a hang, a 1 fps stall that left the server behind and killed the character by falling after the client landed).
