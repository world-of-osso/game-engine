# Sound

The sound system lives in `src/sound/` and covers three areas: footstep sounds, a music catalog, and zone-based music playback.

## Coverage

- **Footsteps** — surface-triggered footstep sounds for player movement
- **Music catalog** — inventory of available music tracks
- **Zone music** — per-zone ambient music selection and playback

The sound system coexists with the rest of the engine as a Bevy plugin registered from `src/main.rs`.

## Shared footstep policy

`src/sound/footstep_data.rs` is the Bevy-free source for footstep creature/race/model and surface classification, animation-to-movement mapping, catalog entry interpretation, and ranked seeded selection. Both the root library and `godot/core` expose it via `#[path]`; `src/sound/footsteps.rs` reexports its API and retains Bevy handles, listfile/cache loading, bucket limits, and byte loading unchanged. Core tests cover the existing classification/ranking examples plus tied seed and no-eligible-entry selection. This is a policy prerequisite only: it adds no native playback and does not change adapter fallback behavior. Independent verifier835 is pending.

`FootstepPhaseTracker` in the same shared file now owns the legacy half-cycle observer: it emits only when the observed movement clip changes half (seed = sequence index shifted 8 bits OR half), resets the half on movement sequence changes, retains it through nonmovement clips, and skips zero-duration clips. It never replays missed steps. Bevy carries it in `FootstepTracker`; native `WowAnimationPlayer::footstep_phase` read-only exposes the selected—not outgoing crossfade—clip index/ID, duration, and clock. `70e047ff` separately shares the ADT dominant-effect/texture surface decision through `terrain_surface_data`; existing root terrain and footstep loaders remain unchanged. The native terrain stream now consumes the shared surface policy per tile; native footstep playback remains unwired. The original extraction's verifier839 remains a separate pending claim.

`src/sound/ground_effect_data.rs` shares the fixed-layout WDC5 GroundEffectTexture and TerrainTypeSounds row readers and ordered terrain-name surface classification with root and `godot/core`. The root `ground_effects` adapter still owns local DB2 loading/cache and clutter, and reexports the same `GroundEffectEntry` type. Core byte fixtures cover the two ground-effect and three terrain-sound layouts, rows, strings, errors, and keyword precedence. Native terrain now reads these two DB2s once per asset reader from local CASC/cache, resolves effect → sound → surface, and stores chunk classifications at tile ingestion. Footstep playback remains unwired.

## Shared catalog boundary

`5f74a859` extracts the root music-zone row parser, `music_manifest.csv` ambient reader, and music/ambient overlap removal into Bevy-free `catalog_data`, exposed to `godot/core`. The root music cache delegates row parsing; root ambient and overlap adapters use the same source. Both readers retain per-area first-encounter track order while deduplicating track indices. `d60a3037` separately shares `AreaTable` `ID` → nonzero `ParentAreaID` parsing and root-ancestor traversal: it stops at a missing parent, limits bad-data traversal to 16 links, and leaves an unknown ID unchanged. This complements `959112e9`'s MCNK `area_id` exposure.

At `741344ca`, native `NativeSound` consumes those catalogs: two owned Godot players decode local MP3/Ogg/WAV bytes, reject FLAC, select music and ambience by zone, cache streams, and apply live volume/mute/music-enable options. Its targeted headless fixture exits 0 for playback state, natural completion, sequence, zone and option changes, expected missing-catalog/FLAC errors, and lifecycle. `cda9a648` independent 811 verification passes Godot `fmt --check`, native `cargo check -p game-engine-godot`, and root `cargo check --locked -p game-engine --bin game-engine`; the existing `NativeWmoGroup::fdid` and `skeleton_afid` warnings remain, and the protected root lock hash is unchanged.

`ce2a8c92` adds a real authenticated `GameClient` fixture. The retained run reaches MCNK area 9 → root zone 12, observes Music player track `53492`, confirms no zone-12 ambient track, then drives the authored Options Sound controls and observes native player volume/mute/music-enable state. The fixture log is [native-sound-client-final.log](../../../data/diagnostics/native-sound-client-final.log). This is volume/player-state proof only: no audible-output, full audio parity, or independent-verifier claim follows; verifier817 is pending. Unexpected fixture markers are ordering/discovery sentinels, not production REDs, and mutation assertions are harness-sensitivity checks only. Its verbose exit reports 16 leaked stream/playback/Ogg-packet objects but no native sound/player nodes; see [[native-audio-shutdown-leaks]]. Godot 4.7.2 remains pinned; the documented engine-level shutdown-timing issue is not being patched now, and client work continues without a full-parity waiver.

## Native UI click

`9145a605` moves the legacy normalized-phase 1,764-sample mono 44.1 kHz click generator and 0.55 gain into `src/sound/ui_click_data.rs`, shared by Bevy and Godot. A projected `STOP` control records only left-pointer-down; `RegistryUi` walks the hit frame's ancestry for `onclick` and rejects any disabled button on that path. Keyboard submit, programmatic action, right-button input, release, and nonactionable controls do not enqueue a click. `GameClient` drains queued pointer clicks before action dispatch, so an action that closes its screen does not discard its click; its owned `NativeSound` Effects player receives generated PCM WAV at `master × effects × 0.55` (zero when muted), independent of music enablement. Independent pinned-Godot verification at `618d153c` passes a fresh owned `GameClient/LoginUI/ConnectButton` left-down path: default FPS observes the owned Effects player active, and 5 FPS observes its completion. The earlier failure was fixture timing: a 40 ms click ended before a frame observed it. `618d153c` corrects only that GDScript test ordering; the root/native check proof and unchanged lock at `9145a605` remain applicable because no Rust source changed. This does not prove audible or hardware output, whole-buffer PCM parity, or full audio parity.

`5274d0c0` strengthens `godot/tests/world_sound_flow.gd`: it first sets master volume to `0.25` and expects Music/Ambient/Effects linear volumes `0.1125`/`0.075`/`0.025`, then invokes Sound Defaults and expects Music/Ambient restoration to `0.45`/`0.3`. Independent authenticated-fixture verification exits 0, proving Defaults after a material sound-state mutation rather than only initial values. No audible-output or full-parity claim follows.

## Runtime scheduling

Commit `550b637a` removes sound clean-frame maintenance. Footstep trackers attach when relevant player/model entities appear. Ambient and music reconciliation run when their inputs change or playback is removed; they do not poll on otherwise clean render frames. Active playback remains active presentation work. No CPU or FPS improvement is claimed without controlled measurement and user observation.

## Audio backend registration

Commit `463e9e47` (`Disable audio backend without sound flag`) makes no-sound mode omit Bevy's `AudioPlugin`, which `DefaultPlugins` previously included even when project `SoundPlugin` was disabled. `--sound` retains Bevy audio and project `SoundPlugin` exactly. Outside `src/sound/`, the only audio-related consumers are an optional `AudioSink` status query and optional `SoundSettings`; neither requires `AudioPlugin`.

Before the fix, the strict-Empty PID `2402583` profile sampled PipeWire audio conversion at **7.61%** plus CPAL/ALSA output-thread work. RED evidence: `/tmp/claude/game-engine-perf/audio-plugin-registration-red.log`. GREEN evidence: `/tmp/claude/game-engine-perf/audio-plugin-registration-green-3.log`. Formatting: `/tmp/claude/game-engine-perf/audio-plugin-registration-cargo-fmt-3.log`. Rust readability: `/tmp/claude/game-engine-perf/audio-plugin-registration-readability/`.

Post-fix PID `2468254` remained focused, `InWorld`, and connected with one link/player; it retained FPS text and zero world content. Its thread inventory contained no `data-loop.0`, `cpal_alsa_out`, or ALSA/PipeWire thread, and its profile contained no PipeWire/CPAL/ALSA symbol. Twelve passive windows after a 30-second warm-up measured **9.61% mean**, **9.55% median**, and **10.60% maximum** CPU; **8/12** met `<=10.0%`. The audio boundary is verified and materially lowers the measured mean, but the strict all-window CPU gate and Character advancement remain blocked.

## Sources

- AGENTS.md — `src/sound/` structure listing
- [app setup](../../../src/app_setup.rs) — stage and audio-plugin registration
- [area_zone_data](../../../src/area_zone_data.rs) — shared AreaTable parent parsing and bounded ancestor traversal
- [footstep_data](../../../src/sound/footstep_data.rs) — shared footstep classification and catalog selection
- [ground_effect_data](../../../src/sound/ground_effect_data.rs) — shared WDC5 row parsing and terrain-name surface classification
- [ui_click_data](../../../src/sound/ui_click_data.rs) — legacy click PCM and gain
- [real-client fixture log](../../../data/diagnostics/native-sound-client-final.log) — authenticated area/zone/music and Options-state observations at `ce2a8c92`

## See Also

- [[terrain]] — zone data that drives zone music selection
- [[networking]] — zone component replicated from server (Zone component in shared crate)
- [[native-audio-shutdown-leaks]] — bounded native and pure-GDScript shutdown evidence
