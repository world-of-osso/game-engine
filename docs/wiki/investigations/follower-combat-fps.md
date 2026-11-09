# Follower dungeon combat FPS

Steady-state frame cost in real gameplay: Ruby Life Pools (map 2521) with the four follower NPCs fighting a Primalist pack, measured against the same spot out of combat. The largest combat cost was the unit frames resyncing every frame while the player had a timed aura.

## Scene and method

Linux debug client against the bench server (game-server `adcb9dd81`), account `fb_ruby`, level-70 Human Warrior `Rubyfps`, 1920x1080. `godot/tests/_diag_combat.gd` (uncommitted diagnostic) queues Ruby through the Group Finder controls (`MinimapLauncherButton` -> `LauncherEntryLFDMicroButton` -> follower type, DPS, dungeon 2436, Find Group). The admin `stage-follower-party` command then places the party on the upper plateau at WoW (1450, -295, 330), next to Primalist Cinderweavers, Flamedancers and Ashseers. The script measures 15 s out of combat and then pulls itself with Tab and `use_spell` (Charge 100, Heroic Throw 57755, Auto Attack 6603). The Primalists do not aggro by proximity. Each segment uses `frame_benchmark.gd`'s per-system medians and its main/render thread split. Driver: `fps120close-tools/combat-run.sh`, summary `combat-summary.py`.

Out of combat at that spot: about 2400 draws, 38 units, main thread 16 ms, render thread 12 ms (Linux debug). In combat: about 2500 draws and 39-48 units.

## Unit frames resynced every frame (fixed, 2026-11-09)

The Forever player frame carried the player's auras as `AuraInstance` values with `remaining` seconds, and target aura icons carried their swipe's `elapsed` fraction. Both change every frame. `InWorldUnitFramesState` therefore never compared equal, and `set_state` rebuilt and re-projected the whole unit-frames screen every frame (`ui.set_state` 414 calls in 414 frames). Follower buffs last one hour, so this happened out of combat too. The state now holds what the frames draw: the player icon's timer text (`player_aura_icons`) and a target icon's `timed` flag. The swipe fractions travel beside the state to the existing per-frame swipe pass (`aura_elapsed`). Unit-frame state changes per run fell from 2201 to about 100; the rest are real value changes such as mana.

`Targeting` step median, 3+3 interleaved runs, master `78a0b50f2` vs fix (load about 31): out of combat 2.38/0.30/1.61 -> 0.19/0.09/0.42 ms, in combat 2.75/0.17/4.23 -> 0.38/0.15/0.68 ms. Run 2 of each had few buffs and little combat. Whole-frame medians were dominated by host load and by how long each fight lasted.

The class bar has the same pattern: `ClassBarView` texture `swipe` progress is computed from the clock. It does not affect a Warrior and is not fixed.

## Camera under the ADT terrain (not fixed)

Ruby's lower level (entrance at WoW 1386, -170, 131) is a WMO floor 70-200 yd below the tile's ADT terrain. `follow_camera` samples the ADT height along the orbit and pulls the camera in to 0.5 yd; its ground clamp then lifts it to the first mesh overhead. Every view there is top-down from 18-19 yd above the player. The plateau measurement avoids it.

## Rubyfps model verification (2026-11-09)

VERIFIED on a private copy of bench `adcb9dd81` at loopback UDP 5976: `fb_ruby`'s character 31 `Rubyfps` is a level-70 Human Warrior, race 1, class 1, sex 0. All seven legacy appearance indices are zero, `customization_choices` is empty and `visage` is absent. The authenticated roster and replicated `Player`/`CharacterVisual` agree; no `ModelDisplay` override is present. A fresh private database's `fb_humanmodel` / `Humanproof` has the same race, sex and appearance.

VERIFIED with an independently built engine `7b7aea959` and shared-protocol `7fa253f`: both characters render Human body FDID `1011653` in Forever. Ruby's real follower admission and the same staging point, WoW `(1450, -295, 330)`, retain that Human body. The large foreground figure is the nearby **Ruby Scaleguard**, `creature/drakonid2/drakonid2.m2`, FDID `4217881`, at client `(1449.43, 330.111, 299.977)`; Ruby is at `(1450, 330.0121, 295)`. The nearby Ruby Keeper uses `dragonspawn2.m2`, FDID `4183015`.

VERIFIED paired diagnostic captures at the report's yaw `0`, pitch `-0.35`, distance `15`: hiding only non-player meshes removes the large dragon-like figure and leaves the Human player unchanged. INFERRED: the original screenshot report mistook this foreground NPC for the player. This is not a race/display-resolution defect in the reproduced scene; no model-selection change is needed. These are source/visual checks, not an FPS receipt or a combat-throughput test.

PNG evidence under `/syncthing/AgentShared/2026-11-09/human-model/`:
- `humanproof-front.png` — fresh Human Warrior on the private server.
- `rubyfps-front.png` — copied bench character, without the dungeon scene.
- `rubyfps-party-reported-view.png` and `rubyfps-party-reported-view-player-only.png` — same plateau and camera, all units versus only the player's meshes.
- `rubyfps-party-player-only-front.png` — the same staged Human from the front.

Logs, diagnostic scripts and the independently built extension's SHA-256 are in `/home/osso/.worktrees/logs/human-model-wrong-2026-11-09/` (`ruby-roster-replication.log`, `fresh-roster-replication.log`, `ruby-party-attribution-client.log`). All owned servers and clients were stopped after capture. The protected server ports were not used.

### Portrait-test rule

VERIFIED: `forever_unit_reference::pet_is_below_player_right_with_retail_player_portrait` fails with `missing PlayerPortrait` on `7b7aea959` (one case run). Its October 14 expectation, committed in `6c7f55451`, was superseded by the user's October 30 no-player-portrait decision in `847843192`. That removal updated the product, [HUD specification](../../specs/hud-edit-mode.md) and [item-appearance specification](../../specs/item-definition-sources.md), but missed this test. Keep Forever portrait-free and retain the pet size/position assertions; correct only this stale portrait expectation. The existing committed failure is the RED acceptance case; do not invent a model failure or restore the obsolete portrait.

## Remaining costs

`Culling` stays at 2-3 ms in both states; with profiling, `cull.particles` (M2 particle update for world objects, units and game objects) cost about 3 ms per frame. The render thread rises about 3 ms in combat.

## Sources

- [Auras host](../../../godot/rust/src/auras.rs) — player-frame and target-aura state, swipe pass.
- [Unit frame auras](../../../godot/ui-model/src/ui/screens/inworld_unit_frames_aura.rs) — `player_aura_icons`, `aura_elapsed`, `set_target_auras`.
- [Camera follow](../../../godot/core/src/camera_follow_data.rs) — terrain-sampled orbit and ground clamp.
- [Player model chain](../../../godot/core/src/player_model_data.rs) and [world appearance selection](../../../godot/rust/src/world.rs) — body FDIDs and replicated display overrides.
- [Forever reference test](../../../godot/ui-model/tests/forever_unit_reference.rs) — the stale portrait assertion and retained pet geometry.

FPS evidence: `data/diagnostics/fps-120-close-2026-11-06/profile/combat-{probe*,diag*,ab-*}.log` and `*-shots/`.

## See Also

- [[rebuild-guard]] — per-frame UI rebuilds must be change-driven.
- [[follower-dungeons]] — follower admission and staging.
- [[godot-stormwind-fps]] — idle city costs.
