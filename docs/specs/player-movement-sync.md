# Player Movement Sync

The Godot client predicts the local player's movement and reports each moved position to the server in `PlayerInput`; the server (game-server `networking_movement.rs`) adopts that position only as far as its speed × movement bank allows. The client must predict at the speed the server grants and report the stop, or client and server positions drift apart. Source: `godot/rust/src/gameplay.rs` (`PlayerMovement`). How it works: [networking](../wiki/systems/networking.md#local-player-speed-and-stop-input).

## What it must do

### Speed
- [x] Predicted speed = the server's `compute_movement_speed` without auras: swim (`SWIM_SPEED`, while swimming), run or walk base × direction multiplier (backpedal 0.6, strafe 0.8), × the aura multiplier.
- [x] The aura multiplier is the local player's replicated `MovementSpeed` over the unmodified speed of the newest reported input, re-derived only when the replicated value changes (the server rewrites it only when it applies an input). 3.5 after a forward run input is a 50% snare: run 3.5, backpedal 2.1, strafe 2.8, walk 1.25, swim 2.36.
- [x] A replicated 0 (root) predicts no movement.
- [x] Live: in deep water the client swims at 4.72 yd/s (7.0 before) and the server ends where the client does (`godot/tests/world_speed_walk.gd`, `SPEED_SWIM=1`, `--fixed-fps 60`).
- [ ] Live: under a snare or speed aura the server's position stays with the client's.

### Stop input
- [x] The first idle frame after a moving or jumping input sends exactly one `PlayerInput` with no direction, no jump, the final predicted position and the adopted epoch; later idle frames send nothing.
- [x] A game menu that halts movement sends that one stop too, without resending airborne state.
- [x] Every release in the native input fixture (walk, run, backpedal, strafe, swim, jump landing, menu return) sends exactly one stop.
- [x] Live: after a backpedal release the server replicates the unmodified speed (only a stop input makes it recompute; 2.833 stayed before) and its position matches the client's.

### Slow client
- [x] A client sending fewer inputs than the server's movement bank needs (one per 0.67 s frame) ends, after its stop, where the server's player stands: the server keeps moving the player to a report its bank has not reached (game-server `MovementClocks.unreached`). Live 2026-10-02, private server, `SPEED_REAL_TIME=1 SPEED_FRAME_MS=667 SPEED_RUN_SECONDS=10`: before, server 15.75 yd of the client's 66.24 (gap 50.49); after, 69.57 of 69.57 (gap 0.000; up to 10.2 yd behind while running). Evidence `data/diagnostics/posdrift-2026-10-02/`.

## How it works

- [networking](../wiki/systems/networking.md#local-player-speed-and-stop-input)
- game-server `docs/wiki/systems/networking.md` "Client → Server Inputs": the movement bank

## Implementation inventory

- `godot/rust/src/gameplay.rs` — `PlayerMovement` speed, aura multiplier, `network_input` / `stop_input`; `GameClient::send_player_input`
- `godot/rust/src/gameplay.rs` — the local player's replicated `MovementSpeed` from the host `Replica`
- `godot/rust/src/lib.rs` — `account_state` `local_server_speed`, `local_player_swimming`

## Tests asserting this spec

- `godot/rust/src/gameplay.rs` tests: `replicated_snare_scales_run_walk_backward_strafe_and_swim_speeds`, `swimming_predicts_at_swim_speed`, `replicated_speed_is_read_against_the_newest_reported_input`, `replicated_root_stops_prediction`, `snared_run_predicts_the_server_distance`, `release_sends_exactly_one_stop_input_with_final_position_and_epoch`, `stop_input_reports_a_modal_stop_once`
- `godot/network/src/lib.rs` `unit_snapshot_owns_server_identity_and_component_values`
- `godot/network/examples/native_input_fixture.rs` (`inworld`, `swimming`, `menu`)
- `godot/tests/world_speed_walk.gd` (live dev or private server; `SPEED_FRAME_MS` for a slow client)
- game-server `networking_tests/receiver_rate.rs` `a_low_fps_client_run_ends_where_the_client_stopped`

## Known gaps (current cycle)

- [ ] No solo way to put a speed aura on a live player: `UseItem` casts no item spell, creatures cast no SmartAI spells, no admin aura command; Fast Footwork (level-10 talent) and a duel Hamstring need a talent commit or a second casting client the Godot client lacks.
- [ ] An aura gained while standing still reaches the client only after the next applied input (the server does not rewrite `MovementSpeed` on aura change), so the first ~RTT of movement runs unmodified; the movement bank absorbs it.

- [ ] `InputChannel` is `UnorderedUnreliable`: the one stop input (like any input) can be lost or overtaken by an older input; nothing resends it.
- [ ] With real frame deltas (`SPEED_REAL_TIME=1`) the dev server, simulating at ~0.5x wall time under load (200 ticks per ~20 s), caps even the fixed client: final gap 0.34-1.28 yd (6.67 before the fix).

## Out of scope

- Mount speed: the server writes `MovementSpeed` on mount but its next input recompute ignores `Mounted`; a server issue.
