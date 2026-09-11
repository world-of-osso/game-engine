# Nameplate style

Overhead health and spell bars match the supplied WoW reference. Rendering lives in `src/rendering/ui/`; [nameplate design](../wiki/design/nameplate-design.md) records context.

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

## Tests asserting this spec

- `src/network_runtime/replication.rs` — cast start/progress/removal snapshot test.
- `src/rendering/ui/health_bar.rs` — default health color.
- `src/rendering/ui/health_bar_zoom_tests.rs` — projected dimensions.

## Known gaps (current cycle)

- [ ] Complete style integration and rendered comparison.
- [ ] Run focused current-revision behavioral verification.

## Out of scope

- Spell damage/healing execution and NPC AI: this work supplies cast presentation lifecycle, not a new combat system. NPC bars require an actual server-side cast source.
- Unrelated nameplate clutter/targeting redesign and bottom player casting-bar changes.
