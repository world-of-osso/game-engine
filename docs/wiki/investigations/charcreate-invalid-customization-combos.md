# Invalid character-creation customization combos

Reported on Human male Customize: Face 27 with a tan Skin Color swatch rendered a dark teal/green body. Two separate defects combined: the compositor ignored layer blend modes, and the catalog offered choices that ChrCustomizationReq forbids.

## Symptoms

- Human male skins 4958, 4972, 4978 and 4979 rendered a flat grey/teal body. The screenshot combination is valid retail data: skin 4978 has swatch `#b68056`, and face 27 is choice 15430. Face 15430 relates to skins 4975, 4978 and 4979.
- A Human warrior was offered 24 skin colors. Only 16 are player choices: 1–10, 4957, 4958, 4972, 4975, 4978 and 4979. The extra choices were 13–15 (Death Knight, green with glowing eyes), NPC-only 11, 12, 16 and 18, and transmog-only 17. Randomize could pick any of them.
- Blood Elf female and Draenei bodies rendered one constant colour whatever skin was selected.

## Root causes

1. **Blend modes.** `char_texture_blit.rs` alpha-blended only BlendMode 1 and 15. It copied every other mode opaquely over the canvas. HD Human layout 103 layer 4 is target 30 with BlendMode 6 over the whole canvas. Skins 4958/4972/4978/4979 put a flat-colour tint texture on target 30 (for 4978, FDID 3571522, mean RGB ≈ 80/97/97), so the tint replaced the skin. WMVx `CharacterTextureBuilder::BlendMode` (`src/core/modeling/Texture.h:76-85`, `Texture.cpp:431-441`) defines 4 multiply, 6 overlay, 7 screen and 9 straight alpha. Overlay is applied with the base (destination) as the selector.
2. **Non-body layers in the body atlas.** `target_uses_atlas` excluded only texture types 2, 6 and 19. Type 20 (target 38) and type 8 (target 2) layers covering the full canvas then painted over the Blood Elf and Draenei bodies. Only TextureType 1 layers belong in the body atlas.
3. **Requirements not evaluated.** `ChrCustomizationReq.csv` was missing locally. The class filter was a hard-coded Night Elf/Blood Elf Demon Hunter face hack, and it offered Death Knight-only faces (req 142, ClassMask 32) to other classes.
4. **Element-derived skin/face compatibility.** Face/skin validity came from `RelatedChrCustomizationChoiceID` elements. Retail validity is `ChrCustomizationReqChoice`. For example, Worgen Death Knight skins 46052–46056 (req 322) have no face elements for Death Knight faces, yet req 322 permits faces 2247–2250.

## Retail rules used

- `ChrCustomizationReq`: TrinityCore `WorldSession::MeetsChrCustomizationReq` (`src/server/game/Handlers/CharacterHandler.cpp:547-601`) checks the `HasRequirements` bit (`0x1`), ClassMask bit `class-1`, RaceMask (`RaceMask.h:97-146` bit table), achievement, item appearance and quest. WoWDBDefs `ChrCustomizationReqType` names 2 NPC and 4 Transmog. A new character has no account unlocks, so unlock-gated rows are not selectable.
- `ChrCustomizationReqChoice`: the same function's dependent-choice check. For each option listed under a choice's requirement, the selection must contain one listed choice. Human skin req 291 requires faces 20–31, 295 requires 15416–15427, 296 requires 15428–15439 and 53 requires 20/22/31.
- Retail Lua lets the player select ineligible choices; only disabled or locked choices are blocked (`Blizzard_CustomizationOptionTemplates.lua:256-272`). The shared reducer therefore keeps the picked choice and repairs the other option: picking a legacy face changes the skin to a legacy skin.

## Fix

- `27d02d52`: the catalog loads `ChrCustomizationReq.csv` and `ChrCustomizationReqChoice.csv` (build-pinned Wago `12.1.0.69875`, provenance in `data/diagnostics/charcreate-customize-req-20260928/`). `choices_for_option` returns only choices whose option and choice requirements allow the race/class and whose required groups can be met, so core selector indices address that list. `src/scenes/char_create/appearance.rs` randomizes, normalizes and selects through ReqChoice repair. Rows with no selectable choices are hidden.
- `e18ebee4`: BlendMode 4/6/7 tint by source alpha, 9 blends by source alpha, and only TextureType 1 layers compose the body atlas.

## Evidence

- RED before the fixes: the warrior offered all 24 skins, a Death Knight skin was accepted, the sweeps failed, and the composited body for 4978 had mean RGB ≈ 83/95/93 against swatch 182/128/86.
- GREEN: `godot/rust/src/char_create/tests.rs` (7 tests) and `godot/rust/src/assets/player.rs` `swatch_tests` (2) pass, as does `godot/core/tests/char_texture_data.rs` (8). Every offered skin for Human, Orc, Dwarf, Night Elf and Blood Elf, both sexes, renders within chromaticity distance 14 of its swatch; Human is at most 9.
- Capture `data/diagnostics/charcreate-customize-req-20260928/customize-human-male-4978-face27.png` (dev server, account `fb_camera`, not submitted): Face 27 with Skin Color 15 renders a tan body.

## Remaining limits

- Core selectors are indices into the requirement-filtered list, so a stored character that used a now-hidden index resolves to a different choice.
- Choices whose ReqChoice needs a different option are selectable, as in retail, but the UI has no distinct ineligible colour.
- Pandaren, Draenei and Nightborne skin swatches do not describe the mean body colour (chromaticity distance 15–30), so the swatch sweep excludes them.
- Non-body texture types (8 skin extra, 20 accessory, ...) are not rendered to their own M2 texture slots.

## See Also

- [[character-creation]] — creation architecture
- [[character-texture-compositing]] — single compositor path
