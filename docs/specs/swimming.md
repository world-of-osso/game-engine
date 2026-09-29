# Swimming

Local swimming in the Godot client: vertical swim input, floating at the water surface, and the Retail breath mirror timer bar. How it works lives in [wiki: swimming](../wiki/systems/swimming.md).

References:
- `Blizzard_FrameXML/Bindings_Standard.xml`: `JUMP` runs `JumpOrAscendStart`/`AscendStop`, `SITORSTAND` runs `SitStandOrDescendStart`/`DescendStop`
- `Blizzard_MirrorTimer/MirrorTimer.xml` and `MirrorTimer.lua`; `UnitDocumentation.lua` `MIRROR_TIMER_START` (timerName, value, maxValue, scale, paused, timerLabel)
- all under `~/.cache/wow-ui-sim/blizzard-ui/`; strings from `data/GlobalStrings.csv` (`BINDING_NAME_SITORSTAND` "Sit/Move Down", `BREATH_LABEL` "Breath", `EXHAUSTION_LABEL` "Fatigue"); atlases from `data/UiTextureAtlasMember.csv` (atlas 1942, `interface/castingbar/uicastingbar.blp`, FDID 4505182)

## What it must do

- [x] Swimming starts where terrain water stands `SWIM_DEPTH` (1.25 yd) over the feet above the ground; a floor above the water keeps the character dry.
- [x] A swimmer has no gravity and no jump: idle it stays where it is.
- [x] Held Jump (default Space) ascends and held Sit/Move Down (`SitOrStand`, default X) descends at `SWIM_SPEED` (4.7222 yd/s) × the server speed modifier ([player movement sync](player-movement-sync.md)); a root holds the swimmer; both held cancel.
- [x] A swimmer rises no higher than feet `SWIM_DEPTH` under the surface, and one floating there follows the surface while swimming level; it sinks no lower than the ground.
- [x] Walking into deep water starts swimming at the floating height (a walk frame that crosses the threshold past it is lifted to it, never below the seabed) and stays at the surface; a spawn or fall into water keeps its depth. Swimming onto a rising shore lifts the feet onto it and walking resumes.
- [x] Horizontal swimming uses `unmodified_speed()` (`SWIM_SPEED` × backpedal/strafe) × the speed modifier, as all prediction does ([player movement sync](player-movement-sync.md)).
- [x] Right-mouse-steered forward/backward swimming follows the camera pitch; strafing and keyboard-only forward stay level.
- [x] `PlayerInput` while swimming: `swimming` true, `jumping` false, `position` the predicted feet; a vertical-only swim step that changed height counts as motion in `report()` and sends an input with zero direction; when it stops (key released, surface or seabed reached) exactly one stop is sent.
- [x] `MirrorTimersData` applies start (value, max, scale, paused), pause and stop per timer (fatigue, breath, feign death) and counts running bars by `scale` ms per ms, clamped to `0..max`; a restart reuses the timer's frame, a new timer takes the first free of three.
- [x] `MirrorTimerContainer` at TOP y -100; shown timers stack 206×32 in frame order; a 195×13 bar at TOP y -2 filled with `MirrorTimerAtlas[timer]` (breath `ui-castingbar-filling-applyingcrafting`) revealing `value/max` of the crop over `ui-castingbar-background`, under `ui-castingbar-frame`, over `ui-castingbar-textbox`, label in white 10 pt under the bar. No timer hides the container.
- [x] The server's `MirrorTimerStart { timer, value_ms, max_value_ms, scale, paused, spell_id }`, `MirrorTimerPause { timer, paused }` and `MirrorTimerStop { timer }` (shared-protocol `MirrorTimerChannel`; timer 0 fatigue, 1 breath, 2 feign death) drive `MirrorTimersData` in send order; an unknown timer fails explicitly. Breath starts when the surface is more than 2.03128 yd over the feet, lasts 180 s and drowns every 1 s at 0 (game-server `docs/specs/breath-and-drowning.md`). No GDScript call starts or stops a bar.
- [x] The server adopts the reported swim height within `[ground, swim_top]` (game-server `docs/specs/player-height.md`); other clients draw the replicated height.
- [x] A swimmer's height changes no faster than `SWIM_SPEED` × the speed modifier, pitched movement and Ascend/Descend combined, the vertical rate the server grants.
- [x] The swim threshold and floating height are shared-protocol `SWIM_DEPTH`, `is_swimming` and `swim_top`, as on the server.
- [ ] Not built: underwater camera/fog, swim-surface bobbing animation, jumping out of the water at a ledge, mount dismount on the client, Undead/Water Breathing breath duration.

## Tests

- `godot/rust/src/gameplay.rs`: `space_ascends_to_the_surface_and_x_descends_to_the_seabed`, `walking_into_deep_water_floats_at_the_surface_at_swim_speed`, `slow_frames_wade_in_to_the_floating_height`, `mouse_steered_forward_swim_follows_camera_pitch` on cached `azeroth_32_48`.
- `godot/core/tests/input_bindings_data.rs`: `SitOrStand` default X, label, section.
- `godot/ui-model/tests/mirror_timer.rs`: countdown/pause/stop/refill, retail layout, atlas crop, label, stacking.
- `godot/rust/src/gameplay.rs` `pitched_swim_with_ascend_or_descend_changes_height_at_most_at_swim_speed`; `godot/rust/src/mirror_timers.rs` server message tests; `godot/rust/src/world.rs` `remote_swimmer_follows_replicated_height_not_the_seabed`.
- `godot/network/src/wire_tests.rs` `native_bridge_receives_mirror_timer_messages_in_order` (loopback Lightyear server).
- `godot/tests/swimming_live.gd`: dev server 127.0.0.1:5000, real Space/X key events, feet below the 2.03 yd head-under line: server breath bar shown, draining at 1/180 per s, hidden after surfacing; server height follows the swimmer. `SWIM_DROWN=1` also waits for breath 0 and a drowning hit; on 2026-09-28 it fails twice: breath reaches 0 about 184 s after diving and the local player's replicated health stays 292 for 10 s (not yet known whether the server deals no damage or its health does not replicate).
- `godot/tests/swimming_input_probe.gd` (loopback `native_input_fixture swimming`): shore crossing floats at the surface.

## Assumptions

- The mouse-steered pitch rule follows Retail play (the character pitches with mouselook); no Blizzard source for it is in the extracted UI.
