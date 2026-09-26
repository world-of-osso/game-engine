# Nameplate style

This spec defines the requested WoW-reference overhead health and spell bars. Rendering lives in `src/rendering/ui/`; [nameplate design](../wiki/design/nameplate-design.md) distinguishes implemented data paths from its separate target-first design.

## What it must do

- [ ] Pixel-match the supplied reference for every non-glyph nameplate pixel: bar/frame bounds, fill fractions, gaps, endpoints, placement, and colors. Glyph rasterization may differ, but labels must retain white Friz styling at 13px for names and 10px for casts.
- [x] Game settings expose independent Thin/Thick selectors, persisted across launches. Defaults are Thick health and Thin spellbar (explicit user choices).
- [x] One `NameplateStyle` data source drives the renderer (no hard-coded sizes/colors): health bar width/height, health fill color per reaction (hostile/neutral/friendly/player class color toggle), cast bar width/height, cast fill color for normal cast, channel, and uninterruptible casts, border visibility, name/cast font size. Values persist with client options and are editable in Options; the Thin/Thick selectors become presets of it. Addon-style plate content replacement is out of scope (user, 2026-09-24).
- [x] Default fill colours are Retail's: health by the owner's reaction to the local player (`UnitSelectionColor` hostile red, neutral yellow, friendly green), players by class colour (`NamePlateFriendlyFrameOptions.useClassColors`), casts by CastingBar start (1, 0.7, 0), channel (0, 1, 0) and non-interruptible (0.7, 0.7, 0.7) colours. Reaction comes from `FactionTemplate` through `shared::faction_reaction`, the rules the server uses (FactionTemplate 7, the Defias Thug, is neutral to a Human player).
- [x] The name is centred above the health bar, its bottom 2px above the plate's top edge (Retail `CenteredAboveHealthBar`, `HEALTH_BAR_TO_NAME_ABOVE_SPACING`). Plate parts are laid out in overlay units, so they stay aligned under the scaled in-world UI camera.
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

- `src/game/nameplate_style.rs` — `NameplateStyle` (sizes, colours, presets, ranges) and the Options slider model.
- `src/game/state/client_options.rs` — persists `HudOptions.nameplate_style` (`nameplateStyle`).
- `src/scenes/game_menu/options.rs` — settings draft/apply wiring, style sliders and toggles.
- `src/ui/screens/options_menu_active_sections.rs` — HUD Thin/Thick preset selectors.
- `src/ui/screens/options_menu_active_sections_nameplates.rs` — Options > Nameplates page.
- `src/game/faction_reaction.rs` — `FactionTemplate.csv` loader for `shared::faction_reaction`.
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

- `src/game/nameplate_style.rs` — colour per reaction/class and cast type, presets, clamping, slider keys.
- `src/game/state/client_options_tests.rs` — style persistence round trip and missing-field presets.
- `src/rendering/ui/health_bar_facing_tests.rs` — FactionTemplate 7/14/11 fills against a Human player, class colour, edited sizes, border hiding.
- `src/rendering/ui/nameplate_bar_tests.rs` — name centred 2px above the plate for both presets and hidden border; font size.
- `src/rendering/ui/nameplate_projection_tests.rs` — cast fill colours and sizes; alignment under a scaled UI camera.
- `src/scenes/game_menu/options_tests.rs`, `src/ui/screens/game_menu_component_tests.rs` — Options editing and the Nameplates page.
- `src/rendering/ui/nameplate_projection_tests.rs` — late-local-owner exclusion and shared body-distance/fade behavior.
- `src/rendering/ui/target_nameplate_tests.rs` — plate-owner selection, mesh precedence, registry UI precedence, and hidden/local exclusion.
- `src/rendering/ui/nameplate_gpu_tests.rs` — half-size visual comparison fixture.

## Known gaps (current cycle)

- [ ] Reaction is template-only: reputation (forced ranks, at-war, Faction reputation bases) is not consulted, on client or server. Diseased Timber/Young Wolves (FactionTemplate 32) therefore read neutral although Retail shows them hostile.
- [ ] Fills are the reference crops desaturated to their HSV value and tinted, so the default hostile body is (195, 0, 0) where the reference shows (195, 43, 41), and the cast fill loses its white highlights. The pixel-match item above is unaffected in status (still open).

- [ ] Run the focused local-owner, distance, and plate-click tests on the committed implementation.
- [ ] Run an offline `nameplatedebug` rendered smoke/capture for plate visibility, selection, cast/channel progress, and Space pause.
- [ ] `src/rendering/ui/nameplate_gpu_tests.rs` still needs aligned half-size captures for all four thickness combinations. It must mask only glyph regions and prove the specified non-glyph geometry and colors against `data/diagnostics/nameplate-style/reference.png`.
- [ ] Prove server-to-client cast replication over a connected network session.

## Verification status

- [x] 2026-09-25, branch `nameplates`: targeted bin (153) and lib (76) tests pass. Headless in-world captures (`data/diagnostics/nameplate-20260925/`, `before-*` from master `07dc8db1`, `after-*` from the branch): Defias Thug yellow (was red), Deputy Willem green (was red), Mangy Wolf (FactionTemplate 38) red, names centred above the bar (were offset left); Options > Nameplates edits persisted and applied (neutral green channel 0.5, health width 200, name font 15).
- [ ] No current-cycle test execution or rendered runtime verification is claimed for local-owner hiding, shared distance, plate selection, or the offline debug screen.
- [ ] No pixel-match success claim exists yet. Frame alpha is reconstructed from a composited screenshot and therefore ambiguous; the reference-derived skins and UI-overlay calibration are implementation input, not proof. GPU comparison is pending.

## Out of scope

- Spell damage/healing execution and NPC AI: this work supplies cast presentation lifecycle, not a new combat system. NPC bars require an actual server-side cast source.
- Glow/art refinements, the separate target-first clutter state machine, bottom player casting-bar changes, and Windows work.
