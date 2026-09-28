# Sound

The sound system lives in `src/sound/` and covers three areas: footstep sounds, a music catalog, and zone-based music playback.

## Coverage

- **Footsteps** — surface-triggered footstep sounds for player movement
- **Music catalog** — inventory of available music tracks
- **Zone music** — per-zone ambient music selection and playback

The sound system coexists with the rest of the engine as a Bevy plugin registered from `src/main.rs`.

## Shared catalog boundary

`5f74a859` extracts the root music-zone row parser, `music_manifest.csv` ambient reader, and music/ambient overlap removal into Bevy-free `catalog_data`, exposed to `godot/core`. The root music cache delegates row parsing; root ambient and overlap adapters use the same source. Both readers retain per-area first-encounter track order while deduplicating track indices. `d60a3037` separately shares `AreaTable` `ID` → nonzero `ParentAreaID` parsing and root-ancestor traversal: it stops at a missing parent, limits bad-data traversal to 16 links, and leaves an unknown ID unchanged. This complements `959112e9`'s MCNK `area_id` exposure; it does not connect that area ID to catalog selection. The catalog and AreaTable core targeted proofs are each 5/5. Root adapter compilation and native catalog consumption/playback remain pending independent verification. Native audio source at `ba028e7c` is not build or integration proof, so this is neither audio parity nor native playback evidence.

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

## See Also

- [[terrain]] — zone data that drives zone music selection
- [[networking]] — zone component replicated from server (Zone component in shared crate)
