# Sound

The sound system lives in `src/sound/` and covers three areas: footstep sounds, a music catalog, and zone-based music playback.

## Coverage

- **Footsteps** — surface-triggered footstep sounds for player movement
- **Music catalog** — inventory of available music tracks
- **Zone music** — per-zone ambient music selection and playback

The sound system coexists with the rest of the engine as a Bevy plugin registered from `src/main.rs`.

## Runtime scheduling

Commit `550b637a` removes sound clean-frame maintenance. Footstep trackers attach when relevant player/model entities appear. Ambient and music reconciliation run when their inputs change or playback is removed; they do not poll on otherwise clean render frames. Active playback remains active presentation work. No CPU or FPS improvement is claimed without controlled measurement and user observation.

## Audio backend registration

Commit `463e9e47` (`Disable audio backend without sound flag`) makes no-sound mode omit Bevy's `AudioPlugin`, which `DefaultPlugins` previously included even when project `SoundPlugin` was disabled. `--sound` retains Bevy audio and project `SoundPlugin` exactly. Outside `src/sound/`, the only audio-related consumers are an optional `AudioSink` status query and optional `SoundSettings`; neither requires `AudioPlugin`.

Before the fix, the strict-Empty PID `2402583` profile sampled PipeWire audio conversion at **7.61%** plus CPAL/ALSA output-thread work. RED evidence: `/tmp/claude/game-engine-perf/audio-plugin-registration-red.log`. GREEN evidence: `/tmp/claude/game-engine-perf/audio-plugin-registration-green-3.log`. Formatting: `/tmp/claude/game-engine-perf/audio-plugin-registration-cargo-fmt-3.log`. Rust readability: `/tmp/claude/game-engine-perf/audio-plugin-registration-readability/`.

Post-fix PID `2468254` remained focused, `InWorld`, and connected with one link/player; it retained FPS text and zero world content. Its thread inventory contained no `data-loop.0`, `cpal_alsa_out`, or ALSA/PipeWire thread, and its profile contained no PipeWire/CPAL/ALSA symbol. Twelve passive windows after a 30-second warm-up measured **9.61% mean**, **9.55% median**, and **10.60% maximum** CPU; **8/12** met `<=10.0%`. The audio boundary is verified and materially lowers the measured mean, but the strict all-window CPU gate and Character advancement remain blocked.

## Sources

- AGENTS.md — `src/sound/` structure listing
- [app setup](../../../src/app_setup.rs) — stage and audio-plugin registration

## See Also

- [[terrain]] — zone data that drives zone music selection
- [[networking]] — zone component replicated from server (Zone component in shared crate)
