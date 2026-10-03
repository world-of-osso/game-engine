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
- Take off as for steady flight (Space jumps, Space held in the air), which is Retail's Skyward Ascent 372610 (JUMP, JUMP: Blizzard_Tutorials_RPE.lua:391-395): the client sends that cast once (`PlayerMovement::take_takeoff_request`) and keeps falling; the server spends a Skyriding Charge and its `SpellGo` (`skyriding_spell`) launches the mount straight up at 31.5 yd/s on top of its run and rises without lift (about 24 yards), then glides once falling with 7.5 yards of air under it.
- Gliding: no thrust; lift turns the velocity toward the facing and the pitch, gravity trades height for speed (dive faster, climb slower), air friction 1.5 yd/s², gravity stops at 65 yd/s. With the right mouse button the pitch follows the camera pitch (at most 180°/s); without it the pitch stays.
- Landing: coming down onto terrain or a WMO floor clears `flying`; swimming depth ends it into a swim. Every frame of a skyride reports (`PlayerInput.flying`); the server sets `FLYING | ADV_FLYING`.
- Flight style: Switch Flight Style 436854 (5 s cast) swaps Steady and Skyriding on the server, saved per character; the next mount flies the new style (game-server `docs/wiki/systems/mounts.md`).
- Not yet: banking, surface friction, the old-world 85% speed.

## Skyriding (part 2: vigor and abilities)

Vigor is the Skyriding Charges the server keeps (game-server mounts page: ChargeCategory 2391, 6 charges, 10.35 s each while on the Skyriding aura). The client casts the abilities through the normal spell path (`use_spell`; part 3 puts them on the override bar) and flaps only on the server's `SpellGo` for the local player (`combat_visuals.rs` → `PlayerMovement::skyriding_spell` → shared `Glider::cast`), so a refused cast (no charge, cooldown) never moves it:
- Surge Forward 372608 and Whirling Surge 361584: +31.5 yd/s along the facing and pitch; Skyward Ascent 372610: +31.5 yd/s up. The speed an impulse reaches is capped at FlightCapability 11 `AddImpulseMaxSpeed` 100; above `MaxVel` 65 the mount loses `OverMaxDeceleration` 7 yd/s² more. The impulse sizes are assumptions in one block of shared-protocol `src/skyriding.rs` (the effects are DUMMY server scripts), listed in game-server `docs/specs/skyriding.md`.
- Aerial Halt 403092: air friction × 100 (its MOD_ADV_FLYING_AIR_FRICTION 10000%) for 0.5 s, which stops a mount at up to 75 yd/s; its reduced gravity (4 s, amount unpublished) is not modelled.
- Not yet: Whirling Surge's spiral visual.

## Skyriding (part 3: bar and vigor)

- Skyriding bar: while an ADV_FLYING aura is up (Skyriding 406095) `GetBonusBarOffset` is 5, so the main bar shows bonus bar 5, action slots 120..131 (`player_spells::bonus_bar_offset`; Retail's Skyriding tutorial finds Surge Forward past `(NUM_ACTIONBAR_PAGES + GetBonusBarOffset() - 1) * 12`, Blizzard_Tutorials_RPE.lua:446). The server fills those slots when the rider gains ADV_FLYING (game-server mounts page). Dismounting shows page 1 again. Action slots are 0..179 (`MAX_ACTION_BUTTONS`).
- Vigor: `SpellChargesUpdate` carries its `ChargeCategory`; the client keeps charges per category and recovers them locally between updates. `vigor.rs` shows the Retail `FillUpFrames` widget (`dragonriding_vigor`, atlas FDID 4730866: 42×45 frames, first and last padded -20, decor wings 8 lower) for Skyriding Charges 2391 above the main bar while the Skyriding bar is up; unreported charges count as full.
- Assumed: the `dragonriding_vigor` texture kit (Retail also has `dragonriding_sgvigor` themes; the widget's kit is server data we do not have) and the bar order Surge Forward, Skyward Ascent, Aerial Halt, Whirling Surge, Second Wind. Not modelled: the widget's flash/flipbook animations and the spark mask.

## Proof

- Live `godot/tests/skyriding_bar_live.gd` (2026-10-03, private server UDP 5196, game-server skyride3 `62cbfca`, character Fbskybar with 34090, 32235, 403092, 361584, 425782 and `flight-style skyriding`): page 1 → mounted bar `[372608, 372610, 403092, 361584, 425782]` with vigor 6/6, key 1 casts Surge Forward → 5/6 with the sixth filling, dismount → page 1, no vigor. `data/diagnostics/skyride3-2026-10-03/` (run 1 failed: the transport did not receive `SpellChargesUpdate`).

- Depot unit tests (`gameplay::tests`): takeoff/climb at flight speed, hover without gravity, X descent and landing input, mouse-steered pitch, fall after losing `CAN_FLY`, Space without `CAN_FLY` only jumps.
- Live `godot/tests/flying_mount_live.gd` (2026-10-02, private server UDP 5179, game-server `4ac0e91`, character Fbflymount with Expert Riding 34090 and Golden Gryphon 32235 via `game-server-admin learn-spell`): mounts (display 17697, rider on attachment 0), climbs 30 yd in 1.66 s, flies forward, hovers 2 s with no height change, server height within one frame of the client, lands at the terrain height (server speed back to mounted run 14), dismounts. Screenshots and traces: `data/diagnostics/flymount-2026-10-02/`. The client needs ≥ 4 fps: the server applies at most 0.25 s of movement per input; an earlier run at 2 fps (loaded host) left the server half a flight behind.
- Skyriding Depot unit tests (`gameplay::tests`): launch apex 23.9 yd with no lift, glide, mouse-steered dive past 25 yd/s and landing; losing `CAN_ADV_FLY` falls; RED with the launch disabled.
- Live `godot/tests/skyriding_live.gd` (2026-10-03, private server UDP 5185, game-server skyride1 `006b750`, character Fbskyride set on the peak at WoW (-8793, 213, 382) with 34090, 32235 and `flight-style skyriding`): launch 24.2 yd, dive 405 → 239 yd at up to ~64 yd/s, pull-up 63.7 → 46.8 yd/s while climbing 25 yd, level glide, landing on the terrain with the server at the same spot. Server off the client's path at most 0.66 yd over 89 samples at ≥ 10 fps (2.09 yd during a 1-2 fps host stall, where the server's catch-up runs straight to the newest report); replication lag about 0.2 s. `data/diagnostics/skyride1-2026-10-02/` (run 6 final; earlier runs: low launch, WMO landing the old test took for a hang, a 1 fps stall that left the server behind and killed the character by falling after the client landed).
- Skyriding abilities Depot unit test (`gameplay::tests::skyriding_abilities_flap_the_glider`, RED without `skyriding_spell`): Surge Forward adds 31.5 yd/s along the facing, the next frame moves 25+ yd/s faster, Aerial Halt stops the horizontal motion within 0.4 s, nothing flaps on the ground.
- Live `godot/tests/skyriding_abilities_live.gd` (2026-10-03, private server UDP 5194, game-server skyride2 `7e9c2b6`, shared-protocol `8065440`, character Fbskyridetwo on the Northshire peak with 34090, 32235, 403092, 425782 and `flight-style skyriding`): launch, two Surge Forwards through the spell pipeline (mean speed 43.4 → 70.9 and 64.9 → 93.2 yd/s, server on the client's path within 0.00 yd at about 0.1 s lag), landing mounted at (-8793, 145, 515), six more ground casts (two charges had recovered in flight), "No charges remain.", a charge back 7.0 s later (10.35 s after the previous one). `data/diagnostics/skyride2-2026-10-03/` (run 3 final; run 1 never landed within 40 s, run 2 landed in the lake and swam off the mount).
