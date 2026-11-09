# Follower dungeon combat FPS

Steady-state frame cost in real gameplay: Ruby Life Pools (map 2521) with the four follower NPCs fighting a Primalist pack, measured against the same spot out of combat. The largest combat cost was the unit frames resyncing every frame while the player had a timed aura.

## Scene and method

Linux debug client against the bench server (game-server `adcb9dd81`), account `fb_ruby`, level-70 Human Warrior `Rubyfps`, 1920x1080. `godot/tests/_diag_combat.gd` (uncommitted diagnostic) queues Ruby through the Group Finder controls (`MinimapLauncherButton` -> `LauncherEntryLFDMicroButton` -> follower type, DPS, dungeon 2436, Find Group). The admin `stage-follower-party` command then places the party on the upper plateau at WoW (1450, -295, 330), next to Primalist Cinderweavers, Flamedancers and Ashseers. The script measures 15 s out of combat and then pulls itself with Tab and `use_spell` (Charge 100, Heroic Throw 57755, Auto Attack 6603). The Primalists do not aggro by proximity. Each segment uses `frame_benchmark.gd`'s per-system medians and its main/render thread split. Driver: `fps120close-tools/combat-run.sh`, summary `combat-summary.py`.

Out of combat at that spot: about 2400 draws, 38 units, main thread 16 ms, render thread 12 ms (Linux debug). In combat: about 2500 draws and 39-48 units.

## Unit frames resynced every frame (fixed, 2026-11-09)

The Forever player frame carried the player's auras as `AuraInstance` values with `remaining` seconds, and target aura icons carried their swipe's `elapsed` fraction. Both change every frame. `InWorldUnitFramesState` therefore never compared equal, and `set_state` rebuilt and re-projected the whole unit-frames screen every frame (`ui.set_state` 414 calls in 414 frames). Follower buffs last one hour, so this happened out of combat too. The state now holds what the frames draw: the player icon's timer text (`player_aura_icons`) and a target icon's `timed` flag. The swipe fractions travel beside the state to the existing per-frame swipe pass (`aura_elapsed`). Unit-frame state changes per run fell from 2201 to about 100; the rest are real value changes such as mana.

`Targeting` step median, 3+3 interleaved runs, master `78a0b50f2` vs fix (load about 31): out of combat 2.38/0.30/1.61 -> 0.19/0.09/0.42 ms, in combat 2.75/0.17/4.23 -> 0.38/0.15/0.68 ms. Run 2 of each had few buffs and little combat. Whole-frame medians were dominated by host load and by how long each fight lasted.

## Class bars resynced every frame (fixed, 2026-11-09)

A class bar animates from the clock: an Evoker's filling essence pip changes its swipe, alpha, rotation and offset every frame, and a DK's recharging runes change their swipe. `ClassBarView` lived in `InWorldUnitFramesState` (player frame and personal resource display), so the screen rebuilt every frame while a resource recharged. The state now holds each bar at rest (`class_bar_layout`), and the host patches the live attributes onto the existing frames with `RegistryUi::patch_frames` (`class_bar_texture_patch`). Frames the active skin does not draw are skipped: Forever's player frame has no class bar.

Evoker Evofps on the plateau, Modern layout, one run each with field-diff diagnostics (load 30-60): class-bar state changes 801 -> 0, `ui.set_state InWorldUnitFramesState` 817 -> 10 per run, `Targeting` CPU 4.4 -> 2.85 ms per combat frame. The essence pips render the same (screenshots `cb-evo-mm1`/`cb-evo-mf1`). While a pip animates, the patch still makes `projection.sync` lay out and update all 205 unit-frame nodes, about 1.5 ms per frame; a projection pass that updates only render-dirty nodes would remove it. The DK harness character shows no rune bar (its replicated powers have no runes; not investigated).

## Camera under the ADT terrain (fixed)

Ruby's entrance WMO floor is below its ADT terrain; the 2026-11-09 private Forever capture measured floor 131.0005 and ADT 204.6584. Before the fix, a clear 15 yd orbit sat 16.169 yd from the eye and 17.969 yd above the player. The camera now checks only actual local geometry along the eye-to-camera ray. The matched after capture restores the full 15 yd orbit at 6.943 yd above the player; see [the camera investigation](dungeon-camera-adt.md) for tests, outdoor control and proof.

## Remaining costs

`Culling` stays at 2-3 ms in both states; with profiling, `cull.particles` (M2 particle update for world objects, units and game objects) cost about 3 ms per frame. The render thread rises about 3 ms in combat.

## Sources

- [Auras host](../../../godot/rust/src/auras.rs) — player-frame and target-aura state, swipe pass.
- [Unit frame auras](../../../godot/ui-model/src/ui/screens/inworld_unit_frames_aura.rs) — `player_aura_icons`, `aura_elapsed`, `set_target_auras`.
- [Camera follow](../../../godot/core/src/camera_follow_data.rs) — terrain-sampled orbit and ground clamp.
- Evidence: `data/diagnostics/fps-120-close-2026-11-06/profile/combat-{probe*,diag*,ab-*}.log` and `*-shots/`.

## See Also

- [[rebuild-guard]] — per-frame UI rebuilds must be change-driven.
- [[follower-dungeons]] — follower admission and staging.
- [[godot-stormwind-fps]] — idle city costs.
