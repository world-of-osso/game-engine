# Nameplate debug screen

`--screen nameplatedebug` is an offline inspection scene using the runtime nameplate renderers. Source: `src/scenes/nameplate_debug.rs`.

## What it must do

- [ ] Enter directly without login, server connection, terrain, or character-model spawning.
- [ ] Display stationary named units with health values and continuously looping cast/channel progress.
- [ ] Space pauses/resumes progress without hiding castbars.
- [ ] Retain current nameplate size, artwork and HUD thickness settings.
- [ ] Remove preview owners, camera, labels and linked visuals on exit.

## How it works

- [Nameplate design](../wiki/design/nameplate-design.md)

## Implementation inventory

- `src/game/state/game_state_enum.rs` — state and CLI route.
- `src/app_setup.rs` — debug plugin registration.
- `src/scenes/nameplate_debug.rs` — offline owners, playback and instruction text.

## Tests asserting this spec

- `src/scenes/nameplate_debug_tests.rs` — routing, pause/resume, looping and state cleanup.

## Known gaps (current cycle)

- [ ] Parent integration must enable the shared renderers and nameplate targeting in this state.
- [ ] Focused tests and rendered smoke verification pending.

## Out of scope

- New nameplate art, mock renderer, NPC AI, server simulation and world loading.
