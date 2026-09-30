# Character creation

Character creation in `src/scenes/char_create/` and `src/ui/screens/char_create_component/` provides race/class selection, a live preview, customization and character creation. User-selected target: full supported customization and the locally available retail UI source. Architecture and reference provenance live in [character creation](../wiki/systems/character-creation.md).

## What it must do

### Reference and presentation

- [x] Retain `rsx!`/`Screen` authoring and native Bevy UI projection.
- [x] Use source-referenced faction columns, race/class sizes, body-type controls, category tabs, options column and navigation. Native-layout tests cover the reference geometry and shorter viewports; this is not a pixel-parity assertion.
- [x] Provide camera reset, zoom and rotation controls; appearance changes reach the live preview.
- [x] Resolve exact local artwork identities through FileDataIDs, including the authored portrait alpha mask, without machine-specific source directories or substitute images.
- [x] Child-art race, class, body-type and category buttons opt out of the toolkit default skin so only explicitly authored layers paint; camera controls retain their separately authored square treatment.
- [x] Present the DB category `Mirror` (ID 23) as an authored normal/selected icon tab, without a permanent raw-category caption.
- [x] Render push buttons (Back, Customize, Create Character, Randomize, Randomize Name and camera controls) with character select's button skin (`defaultbutton-nineslice-*` up/highlight/pressed/disabled from `src/ui/screens/default_button_atlas.rs`) and its button-text font and state colors, preserving wording, geometry and actions.
- [x] Lay out dropdown choices column-major: one column through 10 choices, then two through 24, three through 36 and four above that, compacting for the popup anchor, viewport and 100-pixel margin. Include MenuStyle2 content insets 3/6/3/7 and element 25590's asymmetric 23/18/23/28 nine-slice background; use subtle hover opacity.
- [x] Size dropdown rows from Retail content: 144 single-column, then 107 multi-column color, 136 multi-column named text, or 70 multi-column numeric-only. Derive columns from those widths.
- [x] Render a first swatch after the selectable 25-pixel number field; use half then full palette artwork for dual colors, support secondary-only colors, and anchor the 51×20 selected outline four pixels before the first effective swatch.
- [x] Center closed dropdown values at their Retail `ResizeLayoutFrame` width: 42 pixels for one effective swatch and 54 for dual swatches; cap named text at 126 pixels. Preserve the 150-pixel control and its input area.
- [x] Size circular hover artwork from the selected checked texture or unselected ring artwork, rather than the button hit area. Hover must remain centered and leave hit areas unchanged.
- [x] Keep choices and primary controls within tested viewport bounds; disabled controls must not emit selection actions.
- [x] Preserve the reference category tabs' 15-pixel hit insets so overlapping artwork does not steal neighboring clicks.

### Races and classes

- [x] Offer the 13 classes in Retail creation order (`classLayoutIndices`, Blizzard_CharacterCreate.lua:912). Monk, Demon Hunter and Evoker use their `ClassIcon_*` FileDataIDs and `ChrClasses.ClassColor` values.
- [x] Gate Monk, Demon Hunter and Evoker by Retail `CharBaseInfo` (FileDataID 1343386, 12.1.0.69933), the same table the server enforces. Monk is on every listed race except Dracthyr. Demon Hunter is on Night Elf, Blood Elf and Void Elf. Evoker is on Dracthyr only. The older classes keep their earlier lists.
- [x] Show Dracthyr (ChrRaces 52 Alliance, 70 Horde) in both allied columns. A Dracthyr selection defaults to Evoker (`ChrRaces.DefaultClassID` 13). The preview and the world model are the dragon form, `character/dracthyr/dracthyrdragon.m2` (ChrModel 89).
- [x] Fit both race columns above the navigation buttons at 1280×720, including six Horde allied races.
- [x] Offer the Demon Hunter horn, tattoo and blindfold choices (ChrCustomizationReq ClassMask 2048) and the Dracthyr dragon-form options.
- [x] Render `ChrCustomizationSkinnedModel` choices. A selected choice's collection M2 (`CollectionsFileDataID`) shows only submesh `GeosetType * 100 + GeosetID`. It is skinned to the character skeleton, with collection bones matched to character joints by `M2CompBone.boneNameCRC`, then by key-bone name. Its replaceable textures use the body skin (type 1, and type 8 by fallback) plus the raw material textures of the layout's other texture types, such as the Demon Hunter blindfold's type 9. This follows wow.export `update_skinned_models`, `buildBoneRemapTable` and `resolve_replaceable_textures`. Verified live in the creation preview: Night Elf Demon Hunter horns and blindfold (7760205), Dracthyr horns and armor pieces (4375631, 4489412), Mechagnome arm upgrade (2628212).

### Authored creation scenes

- [x] Resolve creation-scene FileDataIDs from `ChrRaces.CreateScreenFileDataID`, including Alliance, Horde and neutral Pandaren; keep the selectable roster unchanged.
- [ ] Render all three original scene models with their authored textures, animation and lights instead of the substitute grass plane.
- [ ] Do not add the old preview directional sun or manufacture an unused procedural sky map; retain the scene ambient color and authored point lights. Bevy light-unit calibration is not exact Retail shading parity.
- [ ] Use the authored default camera framing and preserve rotate, zoom, reset and face-focused customization controls.
- [ ] Replace scenes on mapped-scene changes without retaining old geometry/lights; release scene descendants on exit and reuse the backdrop for races sharing it.

### Customization

- [x] Present available options by authored category/order, including eye color and applicable race-specific options instead of a fixed five-row UI.
- [x] Derive displayed values, labels and swatches from the same class-filtered choices; preserve authored split colors and option/choice IDs.
- [x] Keep six existing core selectors canonical for their original options; additional option/choice pairs must not independently control those same options.
- [x] Preserve additional selections through preview, serialized creation, persistence and roster reload, including upgrades of existing stored characters.
- [x] Race, class, body-type and category changes keep supported selections and preview output coherent; skin/face compatibility is retained.
- [x] Support the dropdown and two-choice checkbox control types present in local data. Unsupported control types are explained rather than represented by inert controls.
- [x] Keep supported material/geoset effects selectable when a choice also contains unimplemented effects; show a partial-support notice. Disable unsupported-only choices explicitly.
- [x] Offer only choices whose `ChrCustomizationReq` allows a new character of the selected race and class: ReqType bit 0 set, ClassMask and RaceMask matching, and no achievement, quest or item-appearance unlock. NPC (ReqType 2) and transmog (ReqType 4) choices are never offered. Hide options that have no offered choices.
- [x] Every selection, stepper, randomize and normalize result satisfies `ChrCustomizationReqChoice`. The picked choice is kept and the options it depends on, or that depend on it, are repaired.
- [x] The rendered body matches the selected skin swatch. BlendMode 4/6/7 layers tint and BlendMode 9 blends by source alpha; only TextureType 1 layers compose the body atlas.

### Creation flow

- [ ] Render name-entry text at the Retail `NumberFont_Shadow_Large` Roman size: Arial Narrow, 20 logical pixels. Keep allowed 12-letter names within the unchanged field and preserve input/caret behavior.

- [x] Preserve typed names through category/popup updates and Back/Next navigation; retain focus and error presentation where applicable.
- [x] Place a distinct Randomize Name dice control immediately left of the name editbox. Load the build-pinned authored NameGen catalog once; select only names for the selected race and body type, mapping Pandaren faction IDs 25/26 to neutral ID 24. Exclude names rejected by the existing 2–12 ASCII-letter creation validation, never truncate or synthesize. Repeated clicks change the name where another candidate exists, updating both draft and editbox without altering appearance, category, popup, navigation or submitting a character. Missing/invalid catalog or empty race/body-type group disables the control; forced selection reports an error.
- [x] Transmit the complete supported appearance through the existing creation path and preserve it after server storage/reopen and roster loading.

## How it works

- [Character creation](../wiki/systems/character-creation.md)
- [UI system](../wiki/systems/ui-system.md)
- [Asset pipeline](../wiki/systems/asset-pipeline.md)

## Implementation inventory

- `src/ui/screens/char_create_component/` — reference views, actions, layout and view models.
- `src/scenes/char_create/` — input, catalog/view bridge, authored name catalog and draft, preview and masked-icon integration.
- `src/rendering/character/{customization_data,customization_cache,appearance_options,character_customization}.rs` — catalog/cache, disjoint selections and material/geoset application.
- `src/ui/character_creation_icons.rs` — cached authored-alpha-mask composition.
- `../shared-protocol/src/components.rs`, `../game-server/crates/server/src/character_data.rs` — appearance payload and stored-data upgrades.
- `../ui-toolkit/src/atlas/retail.rs`, `../ui-toolkit/src/attrs.rs` — atlas identities/crops and authored hit insets.

## Tests asserting this spec

- `src/ui/screens/char_create_component/mod_tests.rs` — reference geometry, popup insets, content-dependent columns, swatch/outline placement, dropdown nine-slice projection, hit areas, choice identity, disabled controls and popup/name stability.
- `tests/unit/charcreate_button_background_tests.rs` — child-art controls project no default root image; Mirror icon tab projects authored pixels without a permanent caption; circular hover artwork has native geometry/pixel regressions.
- `godot/rust/src/ui/button_style_tests.rs` — every character-creation push button projects the same skin sources/crops as character select's Back button in normal, hover, pressed and disabled states; navigation labels share its text font and state colors.
- `tests/unit/{char_create_tests,char_create_shared_tests,char_create_response_tests,character_customization_tests}.rs` — selection, request loopback, response and render-effect behavior.
- `src/scenes/char_create/{name_catalog_tests,name_action_tests}.rs` and `src/ui/screens/char_create_component/name_button_tests.rs` — real authored race/body-type coverage, validation, distinct action, native placement, missing data, draft/editbox preservation.
- `src/scenes/char_create/{scene_tests,scene_tests_runtime,neutral_capture_tests}.rs` — authored backdrop lighting/material boundaries, camera/preview presentation, neutral loader-only capture and native mouse-input scheduling.
- `tests/unit/{customization_data_tests,customization_catalog_cache_tests}.rs` — catalog fidelity, filtering, stale-schema autoload and real local-data loading.
- `godot/rust/src/char_create/tests.rs` — real-catalog Human skin eligibility, a rejected Death Knight skin for a warrior, the reported skin 4978/face 27 combination, and a sweep over races 1/2/3/4/10/22 × sex × class checking requirements, randomize and select-any-choice ReqChoice validity.
- `src/scenes/char_create/data.rs` tests: CharBaseInfo gates of the new classes, Dracthyr factions and default class, Retail class order. `godot/ui-model/tests/char_create.rs` `race_columns_end_above_the_navigation_buttons_at_720p`. `godot/rust/src/char_create/tests.rs`: Demon Hunter class choices and the Dracthyr dragon-form options.
- `godot/tests/new_class_live.gd` (live, private server): creates a character through the real screens, enters the world, then casts at a Training Dummy.
- `godot/rust/src/assets/player.rs` `swatch_tests`, `godot/core/tests/char_texture_data.rs` — rendered body chromaticity against every offered skin swatch (Human, Orc, Dwarf, Night Elf, Blood Elf), and exact blend-mode and non-body-layer bytes.
- `src/ui/character_creation_icons.rs` — decoded pixel/mask/cache/error regressions.
- Shared/server appearance tests — wire roundtrips, six historical storage schemas, temporary-database reopen and login roster preservation.
- `debug/character-create.js` — real offline controls, eyes/ears, name entry, camera actions and Back/Next, without character submission.

## Scoped visual verification — 2026-09-22

Reported control/background corrections have native-layer, decoded-asset, interaction and inspected runtime evidence. Final one-column and four-column captures show filled swatches, selected outlines, readable labels and background coverage. Independent checks pass for authored slicing, repeated screen sync and fractional-scale navigation. Evidence: `data/diagnostics/charcreate-button-style-20260922/completion-report.md` and `independent-dropdown-slice-report.md`. Runtime logs retain unrelated local-CASC character-texture misses; this is not full 3D-preview acceptance.

## Known gaps (current cycle)
Closed-value centering and authored circular hover sizes passed independent native pointer/pixel/geometry/input checks and inspected owned-window captures on 2026-09-23. Evidence: `data/diagnostics/charcreate-hover-select-20260922/completion-report.md`; popup styling and hit areas are unchanged.

- [ ] Inspect a new GUI capture after the authored-creation lighting revision. Existing scoped RED/GREEN evidence in `data/diagnostics/charcreate-authored-scenes-20260923/` covers the removed 8,000-lux fill, unbound/removed procedural map, ambient conversion, diffuse backdrop materials, presentation scale/distance, and neutral loader-only GPU capture; it is not final three-backdrop visual acceptance.
- [ ] Pixel-perfect Retail visual parity has not been established. The contracts above are source-, layout- and native-layer-tested; no uninspected screenshot comparison or pixel-parity claim is made. Native additive glow, tooltip/hold-repeat details and unsupported effect families are not claimed complete.
- [ ] Account unlocks (achievement, quest, item appearance) cannot be evaluated locally, so those choices are treated as locked. Visibility requirements are not interpreted, and ineligible-but-selectable choices have no distinct colour.
- [x] Filtering changes only what is offered. Stored core selector indices keep addressing the authored list, with the original Night Elf/Blood Elf face split and the faces it hid appended. Dev-roster appearances (Theron, Elara, Fbcamera) resolve to their original choice IDs.
- [ ] Bone sets, conditional/skinned models, voice, animation-kit and other non-material/geoset effects remain unsupported or partial, as shown by the controls.
- [ ] Slider type 2 has no records in the local option data and is not implemented; types 0/1 are the supported contract for this data set.
- [ ] The local install reports build `12.1.0.69875`; independent version provenance of the extracted Interface source is unconfirmed. The local files themselves are the chosen reference.

## New classes live run — 2026-09-30

`godot/tests/new_class_live.gd` exited 0 three times on private UDP 5100 (game-server `newclasses`), one run per class:

- Human Monk `Fbmonktwo`: level 1 in Northshire; Tiger Palm hit.
- Night Elf Demon Hunter `Fbhavoc`: level 8 in Shadowglen; Demon's Bite left 25/100 Fury.
- Dracthyr Evoker `Fbscales`: level 10, Devastation, at the Stormwind flight master; Living Flame dealt 180 damage and cost 93 mana.

Each character was teleported next to a Northshire Training Dummy before casting. Captures are in `data/diagnostics/newclasses-2026-09-30/`.

Gaps found in this run:
- [x] Demon Hunter blindfolds render with their type-9 texture (ChrCustomizationMaterial 104975/104978 → TextureFileData 7758295/7758292, wago 12.1.0.69933). Linen was checked in a zoomed capture: `fblinen-4-12-0-customize-zoom.png`.
- [ ] Dracthyr visage form (ChrRaces 75, ChrModel 127/128) and the dragon/visage toggle are missing. The dragon-form wings render untextured (white).
- [ ] The Demon Hunter preview body renders black above the waist. This is unexamined.
- [ ] "Demon Hunter" overflows its class tile label.
- [ ] A cross-map teleport logs `Node3D::upcast_ref ... after it has been freed` panics in `GameClient::process()`. The client keeps running.

## Customization data refresh — 2026-09-30

`scripts/export_db2_csv.py` re-exported ChrCustomizationElement, ChrCustomizationMaterial, ChrCustomizationSkinnedModel (new) and ChrModelTextureLayer from local CASC (build 12.1.0.69933 layouts). The previous CSVs were dated 2026-03-12 and lacked the 12.x Demon Hunter rows. The delta is +360/-35 elements, +68/-24 materials and +12 layers. The old files and caches are in `data/pre-12x-customization-20260930/`. `data/cache/customization.sqlite` is now schema 3, with `elements.skinned_model_id` and `skinned_models`. ChrCustomizationChoice and ChrCustomizationOption were not refreshed, so choices added in 12.x without a local Choice row (for example 62817) are not offered.

## Out of scope

Unrelated game systems and replacement of `rsx!`/`Screen` authoring.
