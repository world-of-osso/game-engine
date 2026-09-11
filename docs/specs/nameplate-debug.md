# Nameplate debug screen

`--screen nameplatedebug` (also `--screen nameplate-debug`) is an offline inspection scene using the runtime nameplate renderers. Source: `src/scenes/nameplate_debug.rs`.

## What it must do

- [ ] Enter directly without login, server connection, terrain, or character-model spawning.
- [ ] Display stationary plain preview owners with names, health values, normal casts, and channels that loop continuously.
- [ ] Space pauses/resumes progress without hiding castbars.
- [ ] Clicking a visible plate selects its preview owner without requiring NPC or remote-player identity.
- [ ] Retain the approved half-size artwork and independent HUD thickness settings: Thick health and Thin spellbar defaults.
- [ ] Remove preview owners, camera, labels and linked visuals on exit.

## How it works

- [Nameplate design](../wiki/design/nameplate-design.md)

## Implementation inventory

- `src/game/state/game_state_enum.rs` — state and CLI route.
- `src/app_setup.rs` — debug plugin registration.
- `src/scenes/nameplate_debug.rs` — offline owners, looping playback, Space pause, selection annotation, and instruction text.
- `src/rendering/ui/nameplate.rs` — shared name projection active in this state.
- `src/rendering/ui/health_bar.rs` — shared health projection active in this state.
- `src/rendering/ui/nameplate_cast_bar.rs` — shared cast projection active in this state.
- `src/rendering/ui/target.rs` — shared plate-owner click selection active in this state.

## Tests asserting this spec

- `src/scenes/nameplate_debug_tests.rs` — routing, pause/resume, looping and state cleanup.

## Known gaps (current cycle)

- [ ] Run focused routing, playback, lifecycle, and plate-click tests on the committed implementation.
- [ ] Run a rendered offline smoke/capture; no runtime verification is claimed yet.

## Out of scope

- New nameplate art/glow work, mock renderer, NPC AI, server simulation, world loading, and an asserted retail distance cap.
