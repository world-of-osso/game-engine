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

## Unit-frame projection redrew every node (fixed, 2026-11-09)

After the class-bar fix, an animating essence pip still dirtied a few frames every frame, and `UiProjection::sync` then called `update_node` for all 205 unit-frame nodes (about 1.2 ms median for `projection.nodes`). A sync now updates only a node whose frame is render-dirty, whose rect or an ancestor's rect moved, whose parent is render-dirty (highlight layers and stepper arrows read the parent's button state), or that has no projected visual yet (new frame, skin or scale change, arrived art). It sets the z index only when it changes. Evoker combat, interleaved 2+2 runs (heavy host load): nodes updated per unit-frame sync 205 -> median 4. Unit-frame sync CPU fell from 2.94/3.39 to 1.44/1.43 ms per frame, and `Targeting` from 4.69/5.13 to 3.08/2.90 ms. The unit frames render the same (`projection-dirty-only-unitframes-master-vs-fix.jpg`). `ui_projection_parity.gd` compares every node with a fresh projection after each of ten changes (move, recolour, hide, text, add, remove, hover, unhover, patch). It fails when the parent rule is removed.

## Particle update re-read static transforms (fixed, 2026-11-09)

Windows receipt (same session as master 87d82cc38): indoor 294.55/235.40 vs 282.09/226.45, outdoor 299.67/237.81 vs 303.77/235.63 (median/1% low); passes the reference rule. See `data/diagnostics/fps-receipt-e3b50db0b-benchadc-2026-11-09/README.md`.

`cull.particles` cost about 3 ms per frame on the plateau (Evoker run, wall time): world doodads 2.0-2.5 ms, units 0.3-0.6 ms, game objects up to 0.5 ms. In the doodad loop, 160 visible doodads ran 413 emitters. Per frame they spent 251 us probing each node's parent chain for a bone attachment (`current_model_transform`), 382 us reading every emitter's bone pose through `Skeleton3D::get_bone_global_pose`, 137 us reading animation playback, 476 us simulating and 491 us writing quads. No two emitters shared a bone.

A doodad's node never moves, and its bones change only when its `WowAnimationPlayer` writes a pose. A doodad placement now holds its emitter transforms (`PlacedParticles::hold_pose`) and re-reads them only when the player's `pose_writes` count changes. Units, items and game objects still re-read every update. In the same view, 2 placements (6 emitters) re-read per frame, in 13 us instead of about 630 us. Interleaved whole-span A/B timings were inconclusive: host load varied from 11 to 35 ms main-thread CPU per frame. `particle_debug_screen.gd` checks that a held placement matches a re-reading one through 60 animation steps of each debug model; the portal's emitters move on 59 of them.

## Remaining costs

`Culling` stays at 2-3 ms in both states. In `cull.particles`, simulation and quad writing remain (about 1 ms for the plateau view). The render thread rises about 3 ms in combat.

## Particle instance buffer zero-fill (2026-11-11)

VERIFIED: `ParticlePool::push` used to grow each 20-float instance with zeros, then overwrite every value. Commit `5fb11b211987e3ce000acefc6f4d2efcb118855a` returns the same fixed layout and appends it directly, avoiding that redundant initialization. The bitwise test checks all 20 `f32` bit patterns, and the release `particle_debug_screen.gd` fixture passes. Test commit: `0a1e788020408bbd244a0c7494045b44df599728`.

VERIFIED: Before the change, the staged Ruby release segment had 38 units and `pending=0`; across 1,701 frames, `Culling` was 1.532 ms median and `cull.particles` was 1.147 ms median (0.996 ms on-CPU). Segment-end host loads were 19.65/33.23/41.30 (1/5/15-minute averages). This is a baseline profile, not an A/B result.

UNVERIFIED: The quad-packing optimization's performance effect is unknown. At 10:49:15 on November 11, 2026, the pre-run load check read 11.43/13.55/22.77 and rejected the A/B before launching a client. No FPS improvement is established.

## Emitter frustum admission cache (2026-11-11)

VERIFIED: Each doodad's `SceneryDistance` world box is created at spawn and is not changed later in `TerrainObjects`. The new cache reuses its exact frustum result only while every `HalfSpace` plane remains equal; the opacity check and particle update/defer calls still run every frame. The field-dump test compares 25 bounds against fresh culling for three views, repeated unchanged views, opacity changes, and a late-added emitter; all output bytes match.

UNVERIFIED: The cache's frame-time effect has no valid A/B. At 13:21:18 on November 11, 2026, `uptime` reported 14.56/16.09/18.93; no A/B run started because the 1-minute average was above 10.

## Sources

- [Auras host](../../../godot/rust/src/auras.rs) — player-frame and target-aura state, swipe pass.
- [Unit frame auras](../../../godot/ui-model/src/ui/screens/inworld_unit_frames_aura.rs) — `player_aura_icons`, `aura_elapsed`, `set_target_auras`.
- [UI projection](../../../godot/rust/src/ui/projection.rs) — which nodes a sync updates.
- [Particles](../../../godot/rust/src/particles.rs) — placed emitters, held emitter transforms.
- [Terrain objects](../../../godot/rust/src/terrain/objects.rs) — fixed doodad world bounds and cached particle/ribbon frustum admission.
- [Camera follow](../../../godot/core/src/camera_follow_data.rs) — terrain-sampled orbit and ground clamp.
- [Player model chain](../../../godot/core/src/player_model_data.rs) and [world appearance selection](../../../godot/rust/src/world.rs) — body FDIDs and replicated display overrides.
- [Forever reference test](../../../godot/ui-model/tests/forever_unit_reference.rs) — the stale portrait assertion and retained pet geometry.

- `data/diagnostics/culling-frustum-cache-2026-11-11/emitter-frustum-field-test.log` — byte-exact 25-bound culling decisions against fresh checks.
- `data/diagnostics/culling-frustum-cache-2026-11-11/before-libgame_engine_godot.so` and `after-libgame_engine_godot.so` — release artifacts for the pending A/B (SHA-256 `28de7d52b98d6abb017555f4829d3a264d82f42d83eb9d9e2d4f55de3bd00957`, `66cd2eaa8839b0460bb558fecc19935b733512e5e4b266e069d13d45bd27b915`).
- `data/diagnostics/culling-ruby-2026-11-11/ruby-culling-particle-state.log` — staged pre-change release segment and particle counters.
- `data/diagnostics/culling-ruby-2026-11-11/particle-debug-after.log` — release particle-debug fixture after the buffer change.

FPS evidence: `data/diagnostics/fps-120-close-2026-11-06/profile/combat-{probe*,diag*,ab-*}.log` and `*-shots/`.

## See Also

- [[rebuild-guard]] — per-frame UI rebuilds must be change-driven.
- [[follower-dungeons]] — follower admission and staging.
- [[godot-stormwind-fps]] — idle city costs.
- [[release-fps-reprofile-2026-11-11]] — release profile at Ruby out of combat and A/B limits.
