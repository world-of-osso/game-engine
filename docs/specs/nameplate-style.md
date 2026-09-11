# Nameplate style

This spec defines the requested WoW-reference overhead health and spell bars. Rendering lives in `src/rendering/ui/`; [nameplate design](../wiki/design/nameplate-design.md) distinguishes implemented data paths from its separate target-first design.

## What it must do

- [ ] Health defaults to red; spellbar defaults to gold, with reference-matched framing.
- [ ] Game settings expose independent Thin/Thick selectors, persisted across launches. Defaults are Thick health and Thin spellbar (explicit user choices).
- [ ] Thickness changes apply to existing nameplates without restarting; projected dimensions remain stable across camera zoom and DPI.
- [ ] Active replicated casts show the caster's spell name and progress; removal/completion/cancellation hides the bar. Worker-to-main replication preserves cast additions, progress, and removal.
- [ ] Existing UI-disabled, scene-stage, and visibility controls remain effective.

## How it works

- [Nameplate design](../wiki/design/nameplate-design.md)
- [Networking](../wiki/systems/networking.md)

## Implementation inventory

- `src/game/state/client_options.rs` — saved thickness settings.
- `src/scenes/game_menu/options.rs` — settings draft/apply wiring.
- `src/ui/screens/options_menu_active_sections.rs` — thickness selectors.
- `src/rendering/ui/health_bar.rs` — health geometry, material, screen sizing.
- `src/rendering/ui/nameplate.rs` — projected owner names.
- `src/network_runtime/replication.rs` — cast snapshots across worker/main worlds.
- `../game-server/crates/server/src/cast_presentation.rs` — validated player cast-state lifecycle; no spell effect execution or NPC cast source.

## Tests asserting this spec

- `src/network_runtime/replication.rs` — written cast start/progress/removal snapshot test; current passing engine command evidence is pending.
- `src/game/state/client_options_tests.rs` and `src/scenes/game_menu/options_tests.rs` — persisted selector defaults and draft/apply behavior; current passing engine command evidence is pending.
- `../game-server/crates/server/src/cast_presentation_tests.rs` — start/progress/expiry, cancellation, validation; targeted server proof passed.
- `src/rendering/ui/health_bar.rs` and `src/rendering/ui/health_bar_zoom_tests.rs` — health color/dimensions, with current-revision style proof pending.

## Known gaps (current cycle)

- [ ] Complete style integration and rendered comparison.
- [ ] Run focused current-revision behavioral verification.

## Out of scope

- Spell damage/healing execution and NPC AI: this work supplies cast presentation lifecycle, not a new combat system. NPC bars require an actual server-side cast source.
- Unrelated nameplate clutter/targeting redesign and bottom player casting-bar changes.
