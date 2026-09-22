# Character creation

Character creation in `src/scenes/char_create/` and `src/ui/screens/char_create_component/` provides race/class selection, a live appearance preview, customization and character creation. User-selected target: full supported customization and the locally available retail WoW UI source. See [UI system](../wiki/systems/ui-system.md).

## What it must do

### Reference and presentation

- [ ] Retain `rsx!`/`Screen` authoring and native Bevy UI projection.
- [ ] Match the local `Blizzard_CharacterCreate`, `Blizzard_CharacterCustomize` and `Blizzard_CustomizationUI` source layouts and control artwork: faction race columns, bottom class choices, body-type controls, category tabs, options column and navigation.
- [ ] Provide the reference camera reset, zoom and rotation controls with a live preview; changing appearance updates that preview.
- [ ] Use exact local asset identities, not machine-specific source directories or substituted artwork.
- [ ] Keep labels, selected/disabled states, popup choices and primary actions legible and reachable at the tested viewport.
- [ ] Overlapping category-tab artwork must not steal neighboring clicks: retain the reference's 15-pixel hit insets without changing visual bounds.

### Customization

- [ ] Present supported options by authored category and ordering, including eye color and applicable race-specific options rather than a fixed five-row UI.
- [ ] Use one class-filtered choice sequence for option values, displayed names and swatches.
- [ ] Select by option/choice identity; core selectors and additional selections must not independently control the same option.
- [ ] Preserve additional selections through preview, creation, persistence and roster reload; preserve existing stored characters when the appearance schema gains additional selections.
- [ ] Race, class, body-type and category changes leave valid selections and no stale popup or preview state.
- [ ] Support reference dropdown, discrete-slider and two-choice checkbox behavior where the available data defines those controls.
- [ ] Surface genuinely unsupported choice effects or unresolved eligibility requirements explicitly; do not present inert options as implemented.

### Creation flow

- [ ] Preserve typed names across appearance/category changes, show creation errors and retain working next/back transitions.
- [ ] Preserve existing validated server creation behavior while transmitting the complete supported appearance.

## How it works

- [UI system](../wiki/systems/ui-system.md)
- [Asset pipeline](../wiki/systems/asset-pipeline.md)

## Implementation inventory

- `src/ui/screens/char_create_component/` — declarative controls, actions and view state.
- `src/scenes/char_create/` — selection, input, preview and creation flow.
- `src/rendering/character/customization_data.rs` — local customization catalog.
- `src/rendering/character/customization_cache.rs` — catalog cache and source metadata.
- `src/rendering/character/character_customization.rs` — appearance material/geoset application.
- `../shared-protocol/src/components.rs` — shared appearance payload.
- `../game-server/crates/server/src/character_data.rs` — stored character representation and upgrades.
- `../ui-toolkit/src/atlas.rs` — native texture atlas identities and crops.

## Tests asserting this spec

- `tests/unit/char_create_tests.rs`
- `tests/unit/char_create_shared_tests.rs`
- `tests/unit/char_create_response_tests.rs`
- `tests/unit/charcreate_icon_source_tests.rs`
- `tests/unit/customization_data_tests.rs`
- `src/ui/screens/char_create_component/mod_tests.rs`

## Known gaps (current cycle)

- [ ] Full supported options, reference controls/layout, persistence and rendered interaction acceptance are being implemented; prior texture proof does not close these requirements.
- [ ] The local install reports build `12.1.0.69875`; independent provenance of the extracted Interface source is unconfirmed. The files themselves are the chosen layout reference.
- [ ] Local `ChrCustomizationReq.csv` is absent; general eligibility/unlock parity has not been established.
- [ ] Raw data presence alone does not prove all element effects are supported by the preview renderer.

## Out of scope

Unrelated game systems and replacement of `rsx!`/`Screen` authoring.
