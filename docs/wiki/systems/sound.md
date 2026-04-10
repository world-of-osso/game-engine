# Sound

The sound system lives in `src/sound/` and covers three areas: footstep sounds, a music catalog, and zone-based music playback.

## Coverage

- **Footsteps** — surface-triggered footstep sounds for player movement
- **Music catalog** — inventory of available music tracks
- **Zone music** — per-zone ambient music selection and playback

The sound system coexists with the rest of the engine as a Bevy plugin registered from `src/main.rs`.

## Sources

- AGENTS.md — `src/sound/` structure listing

## See Also

- [[terrain]] — zone data that drives zone music selection
- [[networking]] — zone component replicated from server (Zone component in shared crate)
