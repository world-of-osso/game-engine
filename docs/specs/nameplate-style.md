# Nameplate style

This spec defines the requested WoW-reference overhead health and spell bars. Rendering lives in `src/rendering/ui/`; [nameplate design](../wiki/design/nameplate-design.md) distinguishes implemented data paths from its separate target-first design.

## What it must do

- [ ] Pixel-match the supplied reference for every non-glyph nameplate pixel: bar/frame bounds, fill fractions, gaps, endpoints, placement, and colors. Glyph rasterization may differ, but labels must retain the reference-sized white Friz styling: 26px name labels and 20px cast labels.
- [x] Game settings expose independent Thin/Thick selectors, persisted across launches. Defaults are Thick health and Thin spellbar (explicit user choices).
- [ ] Current working calibration is a 384px health bar at 40px Thick / 20px Thin, and a cast bar at 20px Thick / 12px Thin. This is not pixel verified.
- [x] Active replicated player casts show spell name and normal-fill/channel-drain progress; removal/completion/cancellation hides the bar. Worker-to-main snapshots preserve additions, elapsed progress, and removal.
- [x] Existing UI-disabled, scene-stage, nameplate, health-bar, and distance controls remain effective.

## How it works

- [Nameplate design](../wiki/design/nameplate-design.md)
- [Networking](../wiki/systems/networking.md)

## Implementation inventory

- `src/game/state/client_options.rs` — saved thickness settings.
- `src/scenes/game_menu/options.rs` — settings draft/apply wiring.
- `src/ui/screens/options_menu_active_sections.rs` — thickness selectors.
- `src/rendering/ui/nameplate_art.rs` — shared authored-art cache: `6704514` health background/fill atlas, `4505182` cast fill atlas, and `7241122` important-cast frame atlas.
- `src/rendering/ui/health_bar.rs` — UI-overlay health sprites and screen sizing; health is not PBR-rendered.
- `src/rendering/ui/nameplate.rs` — projected owner names and health-label placement.
- `src/rendering/ui/nameplate_cast_bar.rs` — projected cast atlas frame/fill, labels, and visibility gates.
- `src/network_runtime/replication.rs` — cast snapshots across worker/main worlds.
- `../game-server/crates/server/src/cast_presentation.rs` — validated player cast-state lifecycle; no spell effect execution or NPC cast source.

## Verification status

- [ ] `src/rendering/ui/nameplate_gpu_tests.rs` still needs a deterministic reference-sized fixture with aligned four-state captures. It must mask only glyph regions and prove the specified non-glyph geometry and colors against `data/diagnostics/nameplate-style/reference.png`.
- [ ] No pixel-match success claim exists yet. The current atlas/UI-overlay calibration is working implementation input, not proof.
- [ ] Prove server-to-client cast replication over a connected network session.

## Out of scope

- Spell damage/healing execution and NPC AI: this work supplies cast presentation lifecycle, not a new combat system. NPC bars require an actual server-side cast source.
- Unrelated nameplate clutter/targeting redesign, bottom player casting-bar changes, and Windows work.
