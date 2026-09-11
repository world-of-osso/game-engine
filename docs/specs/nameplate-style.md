# Nameplate style

This spec defines the requested WoW-reference overhead health and spell bars. Rendering lives in `src/rendering/ui/`; [nameplate design](../wiki/design/nameplate-design.md) distinguishes implemented data paths from its separate target-first design.

## What it must do

- [x] Health defaults to red; spellbar defaults to authored gold. Both use reference-style framing, not a pixel-identical atlas replacement.
- [x] Game settings expose independent Thin/Thick selectors, persisted across launches. Defaults are Thick health and Thin spellbar (explicit user choices).
- [x] Health is 190×20 logical pixels when Thick and 190×10 when Thin; spellbars are 190×14 and 190×6. Existing plates update without restart and retain their logical dimensions across tested zoom/DPI.
- [x] Thick labels are left-inset within their bars; thin health labels sit above health and thin spell labels sit below spellbars.
- [x] Active replicated player casts show spell name and normal-fill/channel-drain progress; removal/completion/cancellation hides the bar. Worker-to-main snapshots preserve additions, elapsed progress, and removal.
- [x] Existing UI-disabled, scene-stage, nameplate, health-bar, and distance controls remain effective.

## How it works

- [Nameplate design](../wiki/design/nameplate-design.md)
- [Networking](../wiki/systems/networking.md)

## Implementation inventory

- `src/game/state/client_options.rs` — saved thickness settings.
- `src/scenes/game_menu/options.rs` — settings draft/apply wiring.
- `src/ui/screens/options_menu_active_sections.rs` — thickness selectors.
- `src/rendering/ui/health_bar.rs` — health geometry, material, screen sizing.
- `src/rendering/ui/nameplate.rs` — projected owner names and health-label placement.
- `src/rendering/ui/nameplate_cast_bar.rs` — projected gold cast bars, spark, labels, and visibility gates.
- `src/network_runtime/replication.rs` — cast snapshots across worker/main worlds.
- [`windows-development.md`](../windows-development.md) — default dev feature and GNU Windows workflow; native engine proof remains pending.
- `../game-server/crates/server/src/cast_presentation.rs` — validated player cast-state lifecycle; no spell effect execution or NPC cast source.

## Tests asserting this spec

- `src/network_runtime/replication.rs` — cast start/progress/removal snapshot: 1 passing engine case.
- `src/game/state/client_options_tests.rs` and `src/scenes/game_menu/options_tests.rs` — persisted defaults and draft/apply behavior: 9 passing engine cases.
- `src/rendering/ui/health_bar.rs`, `health_bar_zoom_tests.rs`, `nameplate_bar_tests.rs`, and `nameplate_cast_bar.rs` — color, dimensions, placement, lifecycle, and gates: 30 passing engine cases.
- `src/rendering/ui/nameplate_gpu_tests.rs` — four thickness-combination captures plus health/name/zoom GPU assertions: 4 passing GPU cases.
- `../game-server/crates/server/src/cast_presentation_tests.rs` — start/progress/expiry, cancellation, validation: 5 passing targeted server cases.

## Known gaps (current cycle)

- [ ] Replace procedural health/cast frames with exact authored frame atlases if visual comparison requires it. Current captures were inspected; no pixel-identical claim.
- [ ] Prove server-to-client cast replication over a connected network session.
- [ ] Complete native Windows GNU engine build and launch proof.

## Out of scope

- Spell damage/healing execution and NPC AI: this work supplies cast presentation lifecycle, not a new combat system. NPC bars require an actual server-side cast source.
- Unrelated nameplate clutter/targeting redesign and bottom player casting-bar changes.
