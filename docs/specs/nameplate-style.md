# Nameplate style

This spec defines the requested WoW-reference overhead health and spell bars. Rendering lives in `src/rendering/ui/`; [nameplate design](../wiki/design/nameplate-design.md) distinguishes implemented data paths from its separate target-first design.

## What it must do

- [ ] Pixel-match the supplied reference for every non-glyph nameplate pixel: bar/frame bounds, fill fractions, gaps, endpoints, placement, and colors. Glyph rasterization may differ, but labels must retain white Friz styling at 13px for names and 10px for casts.
- [x] Game settings expose independent Thin/Thick selectors, persisted across launches. Defaults are Thick health and Thin spellbar (explicit user choices).
- [ ] `NAMEPLATE_SCALE = 0.5`: effective health width is 188px; health heights are 20px Thick / 10px Thin; cast heights are 10px Thick / 6px Thin. These targets derive from a 376px raw interior; bitmap outer bounds vary by frame. This is not pixel verified.
- [x] Active replicated player casts show spell name and normal-fill/channel-drain progress; removal/completion/cancellation hides the bar. Worker-to-main snapshots preserve additions, elapsed progress, and removal.
- [x] Existing UI-disabled, scene-stage, nameplate, health-bar, and distance controls remain effective.
- [ ] A `LocalPlayer` owner has no projected name, health, or cast visual, including when the marker is added after those visuals already exist; removing the marker restores the remote plate.
- [ ] Name, health, and cast visuals share the health-body distance anchor and fade/hide boundary. `HudOptions.nameplate_distance` remains the configured policy; no native retail cap is asserted.
- [ ] Clicking a visible non-local plate targets its owner before mesh raycasting. Registry UI under the cursor retains input precedence.

## How it works

- [Nameplate design](../wiki/design/nameplate-design.md)
- [Networking](../wiki/systems/networking.md)

## Implementation inventory

- `src/game/state/client_options.rs` — saved thickness settings.
- `src/scenes/game_menu/options.rs` — settings draft/apply wiring.
- `src/ui/screens/options_menu_active_sections.rs` — thickness selectors.
- `src/rendering/ui/nameplate_art.rs` — shared art cache: reference-derived health/cast frame PNGs with transparent live interiors; glyph-free health gradient crop; authored `4505182` cast fill/background. No generic pip is rendered because none appears in the reference.
- `debug/make_nameplate_skins.py` — reproducibly derives frame/fill skins from the supplied screenshot using linear unmatting; `provenance.json` records crops, source hash, and reconstruction limits.
- `debug/compare_nameplates.py` — compares aligned half-size GPU captures against the BOX-resized reference while excluding glyph regions; diagnostics only, not acceptance proof.
- `src/rendering/ui/health_bar.rs` — UI-overlay health sprites and screen sizing; health is not PBR-rendered.
- `src/rendering/ui/nameplate.rs` — projected owner names and health-label placement.
- `src/rendering/ui/nameplate_cast_bar.rs` — projected cast atlas frame/fill, labels, visibility gates, and shared distance boundary.
- `src/rendering/ui/nameplate_picking.rs` — screen-space hit testing for projected nameplate parts.
- `src/rendering/ui/target.rs` — registry-first click routing; a plate hit selects its owner before world mesh raycasting.
- `src/network_runtime/replication.rs` — cast snapshots across worker/main worlds.
- `../game-server/crates/server/src/cast_presentation.rs` — validated player cast-state lifecycle; no spell effect execution or NPC cast source.

## Tests asserting this spec

- `src/rendering/ui/nameplate_projection_tests.rs` — late-local-owner exclusion and shared body-distance/fade behavior.
- `src/rendering/ui/target_nameplate_tests.rs` — plate-owner selection, mesh precedence, registry UI precedence, and hidden/local exclusion.
- `src/rendering/ui/nameplate_gpu_tests.rs` — half-size visual comparison fixture.

## Known gaps (current cycle)

- [ ] Run the focused local-owner, distance, and plate-click tests on the committed implementation.
- [ ] Run an offline `nameplatedebug` rendered smoke/capture for plate visibility, selection, cast/channel progress, and Space pause.
- [ ] `src/rendering/ui/nameplate_gpu_tests.rs` still needs aligned half-size captures for all four thickness combinations. It must mask only glyph regions and prove the specified non-glyph geometry and colors against `data/diagnostics/nameplate-style/reference.png`.
- [ ] Prove server-to-client cast replication over a connected network session.

## Verification status

- [ ] No current-cycle test execution or rendered runtime verification is claimed for local-owner hiding, shared distance, plate selection, or the offline debug screen.
- [ ] No pixel-match success claim exists yet. Frame alpha is reconstructed from a composited screenshot and therefore ambiguous; the reference-derived skins and UI-overlay calibration are implementation input, not proof. GPU comparison is pending.

## Out of scope

- Spell damage/healing execution and NPC AI: this work supplies cast presentation lifecycle, not a new combat system. NPC bars require an actual server-side cast source.
- Glow/art refinements, the separate target-first clutter state machine, bottom player casting-bar changes, and Windows work.
