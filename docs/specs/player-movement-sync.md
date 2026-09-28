# Player Movement Sync

The Godot client predicts the local player's movement and reports each moved position to the server in `PlayerInput`; the server (game-server `networking_movement.rs`) adopts that position only as far as its speed × movement bank allows. The client must predict at the speed the server grants and report the stop, or client and server positions drift apart. Source: `godot/rust/src/gameplay.rs` (`PlayerMovement`). How it works: [networking](../wiki/systems/networking.md#local-player-speed-and-stop-input).

## What it must do

### Speed
- [x] Predicted speed = the server's `compute_movement_speed` without auras: swim (`SWIM_SPEED`, while swimming), run or walk base × direction multiplier (backpedal 0.6, strafe 0.8), × the aura multiplier.
- [x] The aura multiplier is the local player's replicated `MovementSpeed` over the unmodified speed of the newest reported input, re-derived only when the replicated value changes (the server rewrites it only when it applies an input). 3.5 after a forward run input is a 50% snare: run 3.5, backpedal 2.1, strafe 2.8, walk 1.25, swim 2.36.
- [x] A replicated 0 (root) predicts no movement.
- [ ] Live: in deep water the client swims at 4.72 yd/s and the server's position stays with it (`godot/tests/world_speed_walk.gd`, `SPEED_SWIM=1`).
- [ ] Live: under a snare or speed aura the server's position stays with the client's.

### Stop input
- [x] The first idle frame after a moving or jumping input sends exactly one `PlayerInput` with no direction, no jump, the final predicted position and the adopted epoch; later idle frames send nothing.
- [x] A game menu that halts movement sends that one stop too, without resending airborne state.
- [x] Every release in the native input fixture (walk, run, backpedal, strafe, swim, jump landing, menu return) sends exactly one stop.
- [ ] Live: after a backpedal release the server replicates the unmodified speed (only a stop input makes it recompute) and its position matches the client's.

## How it works

- [networking](../wiki/systems/networking.md#local-player-speed-and-stop-input)
- game-server `docs/wiki/systems/networking.md` "Client → Server Inputs": the movement bank

## Implementation inventory

- `godot/rust/src/gameplay.rs` — `PlayerMovement` speed, aura multiplier, `network_input` / `stop_input`; `GameClient::send_player_input`
- `godot/network/src/lib.rs` — `UnitSnapshot::movement_speed` from the replicated `MovementSpeed`
- `godot/rust/src/lib.rs` — `account_state` `local_server_speed`, `local_player_swimming`

## Tests asserting this spec

- `godot/rust/src/gameplay.rs` tests: `replicated_snare_scales_run_walk_backward_strafe_and_swim_speeds`, `swimming_predicts_at_swim_speed`, `replicated_speed_is_read_against_the_newest_reported_input`, `replicated_root_stops_prediction`, `snared_run_predicts_the_server_distance`, `release_sends_exactly_one_stop_input_with_final_position_and_epoch`, `stop_input_reports_a_modal_stop_once`
- `godot/network/src/lib.rs` `unit_snapshot_owns_server_identity_and_component_values`
- `godot/network/examples/native_input_fixture.rs` (`inworld`, `swimming`, `menu`)
- `godot/tests/world_speed_walk.gd` (live dev server)

## Known gaps (current cycle)

- [ ] No solo way to put a speed aura on a live player: `UseItem` casts no item spell, creatures cast no SmartAI spells, no admin aura command; Fast Footwork (level-10 talent) and a duel Hamstring need a talent commit or a second casting client the Godot client lacks.
- [ ] An aura gained while standing still reaches the client only after the next applied input (the server does not rewrite `MovementSpeed` on aura change), so the first ~RTT of movement runs unmodified; the movement bank absorbs it.

## Out of scope

- Mount speed: the server writes `MovementSpeed` on mount but its next input recompute ignores `Mounted`; a server issue.
