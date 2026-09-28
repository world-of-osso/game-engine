# Swimming

Local swimming in the Godot client: vertical swim input, floating at the water surface, and the Retail breath mirror timer bar. How it works lives in [wiki: swimming](../wiki/systems/swimming.md).

References:
- `Blizzard_FrameXML/Bindings_Standard.xml`: `JUMP` runs `JumpOrAscendStart`/`AscendStop`, `SITORSTAND` runs `SitStandOrDescendStart`/`DescendStop`
- `Blizzard_MirrorTimer/MirrorTimer.xml` and `MirrorTimer.lua`; `UnitDocumentation.lua` `MIRROR_TIMER_START` (timerName, value, maxValue, scale, paused, timerLabel)
- all under `~/.cache/wow-ui-sim/blizzard-ui/`; strings from `data/GlobalStrings.csv` (`BINDING_NAME_SITORSTAND` "Sit/Move Down", `BREATH_LABEL` "Breath", `EXHAUSTION_LABEL` "Fatigue"); atlases from `data/UiTextureAtlasMember.csv` (atlas 1942, `interface/castingbar/uicastingbar.blp`, FDID 4505182)

## What it must do

- [x] Swimming starts where terrain water stands `SWIM_DEPTH` (1.25 yd) over the feet above the ground; a floor above the water keeps the character dry.
- [x] A swimmer has no gravity and no jump: idle it stays where it is.
- [x] Held Jump (default Space) ascends and held Sit/Move Down (`SitOrStand`, default X) descends at `SWIM_SPEED` (4.7222 yd/s); both held cancel.
- [x] A swimmer rises no higher than feet `SWIM_DEPTH` under the surface, and one floating there follows the surface while swimming level; it sinks no lower than the ground.
- [x] Walking into deep water starts swimming at the floating height (a walk frame that crosses the threshold past it is lifted to it, never below the seabed) and stays at the surface; a spawn or fall into water keeps its depth. Swimming onto a rising shore lifts the feet onto it and walking resumes.
- [x] Horizontal swimming uses `SWIM_SPEED` times the backpedal/strafe multipliers, the speed the server grants a `swimming` input.
- [x] Right-mouse-steered forward/backward swimming follows the camera pitch; strafing and keyboard-only forward stay level.
- [x] `PlayerInput` while swimming: `swimming` true, `jumping` false, `position` the predicted feet; a vertical-only swim step that changed height sends an input with zero direction.
- [x] `MirrorTimersData` applies start (value, max, scale, paused), pause and stop per timer (fatigue, breath, feign death) and counts running bars by `scale` ms per ms, clamped to `0..max`; a restart reuses the timer's frame, a new timer takes the first free of three.
- [x] `MirrorTimerContainer` at TOP y -100; shown timers stack 206×32 in frame order; a 195×13 bar at TOP y -2 filled with `MirrorTimerAtlas[timer]` (breath `ui-castingbar-filling-applyingcrafting`) revealing `value/max` of the crop over `ui-castingbar-background`, under `ui-castingbar-frame`, over `ui-castingbar-textbox`, label in white 10 pt under the bar. No timer hides the container.
- [ ] The server sends no mirror timer and deals no drowning damage. Proposed contract (not in shared-protocol): owner-only `MirrorTimerStart { timer: u8 (0 fatigue, 1 breath, 2 feign death), value_ms: i32, max_value_ms: i32, scale: f32, paused: bool, spell_id: i32 }`, `MirrorTimerPause { timer: u8, paused: bool }`, `MirrorTimerStop { timer: u8 }`; the server starts breath when the head goes under, refills it upward on surfacing, stops it when full, and deals drowning damage every 2 s at 0. Until then only `start_mirror_timer`/`stop_mirror_timer` (GDScript) drive the bar.
- [ ] The server ignores the reported height: it keeps gravity and the ground clamp, so a floating swimmer stays on the seabed for the server and every other client.
- [ ] Not built: underwater camera/fog, swim-surface bobbing animation, jumping out of the water at a ledge, mount dismount on the client, Undead/Water Breathing breath duration.

## Tests

- `godot/rust/src/gameplay.rs`: `space_ascends_to_the_surface_and_x_descends_to_the_seabed`, `walking_into_deep_water_floats_at_the_surface_at_swim_speed`, `slow_frames_wade_in_to_the_floating_height`, `mouse_steered_forward_swim_follows_camera_pitch` on cached `azeroth_32_48`.
- `godot/core/tests/input_bindings_data.rs`: `SitOrStand` default X, label, section.
- `godot/ui-model/tests/mirror_timer.rs`: countdown/pause/stop/refill, retail layout, atlas crop, label, stacking.
- `godot/tests/swimming_live.gd`: dev server 127.0.0.1:5000, real Space/X key events, breath bar drawn under water.
- `godot/tests/swimming_input_probe.gd` (loopback `native_input_fixture swimming`): shore crossing floats at the surface.

## Assumptions

- The mouse-steered pitch rule follows Retail play (the character pitches with mouselook); no Blizzard source for it is in the extracted UI.
- Head under water, for the live fixture's breath start, is 1 yd below the floating height.
