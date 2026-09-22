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
- [x] Render Back, Next and Create with authored Retail red left/center/right slices that retain asymmetric cap proportions; synchronize pressed, disabled and hover-highlight state before native projection. Keep labels above all artwork, preserving wording, font and state colors. Slices must meet on physical-pixel boundaries without gaps or overlap at scale factors 1.0, 1.15, 1.25 and 1.5.
- [x] Lay out dropdown choices column-major: one column through 10 choices, then two through 24, three through 36 and four above that, compacting for the popup anchor, viewport and 100-pixel margin. Use the authored stretched background and subtle authored hover layer.
- [x] Keep choices and primary controls within tested viewport bounds; disabled controls must not emit selection actions.
- [x] Preserve the reference category tabs' 15-pixel hit insets so overlapping artwork does not steal neighboring clicks.

### Customization

- [x] Present available options by authored category/order, including eye color and applicable race-specific options instead of a fixed five-row UI.
- [x] Derive displayed values, labels and swatches from the same class-filtered choices; preserve authored split colors and option/choice IDs.
- [x] Keep six existing core selectors canonical for their original options; additional option/choice pairs must not independently control those same options.
- [x] Preserve additional selections through preview, serialized creation, persistence and roster reload, including upgrades of existing stored characters.
- [x] Race, class, body-type and category changes keep supported selections and preview output coherent; skin/face compatibility is retained.
- [x] Support the dropdown and two-choice checkbox control types present in local data. Unsupported control types are explained rather than represented by inert controls.
- [x] Keep supported material/geoset effects selectable when a choice also contains unimplemented effects; show a partial-support notice. Disable unsupported-only choices explicitly.
- [x] Preserve eligibility metadata without inventing account/unlock rules; disclose the missing general eligibility evaluation below.

### Creation flow

- [x] Preserve typed names through category/popup updates and Back/Next navigation; retain focus and error presentation where applicable.
- [x] Transmit the complete supported appearance through the existing creation path and preserve it after server storage/reopen and roster loading.

## How it works

- [Character creation](../wiki/systems/character-creation.md)
- [UI system](../wiki/systems/ui-system.md)
- [Asset pipeline](../wiki/systems/asset-pipeline.md)

## Implementation inventory

- `src/ui/screens/char_create_component/` — reference views, actions, layout and view models.
- `src/ui/screens/char_create_component/navigation_art.rs` — authored asymmetric navigation layers and live button-state synchronization before projection.
- `src/scenes/char_create/` — input, catalog/view bridge, name draft, preview and masked-icon integration.
- `src/rendering/character/{customization_data,customization_cache,appearance_options,character_customization}.rs` — catalog/cache, disjoint selections and material/geoset application.
- `src/ui/character_creation_icons.rs` — cached authored-alpha-mask composition.
- `../shared-protocol/src/components.rs`, `../game-server/crates/server/src/character_data.rs` — appearance payload and stored-data upgrades.
- `../ui-toolkit/src/atlas/retail.rs`, `../ui-toolkit/src/attrs.rs` — atlas identities/crops and authored hit insets.

## Tests asserting this spec

- `src/ui/screens/char_create_component/mod_tests.rs` — reference geometry, hit areas, choice identity, disabled controls and popup/name stability.
- `tests/unit/charcreate_button_background_tests.rs` — child-art controls project no default root image; Mirror icon tab projects authored pixels without a permanent caption.
- `src/ui/screens/char_create_component/navigation_art_tests.rs` — exact local-CASC slice crops, asymmetric geometry, physical-pixel-contiguous slices at 1.0/1.15/1.25/1.5 scale, native label-over-art ordering/style and live normal/pressed/disabled/hover state synchronization.
- `tests/unit/{char_create_tests,char_create_shared_tests,char_create_response_tests,character_customization_tests}.rs` — selection, request loopback, response and render-effect behavior.
- `src/scenes/char_create/{scene_tests,scene_tests_runtime}.rs` — camera/preview and native mouse-input scheduling.
- `tests/unit/{customization_data_tests,customization_catalog_cache_tests}.rs` — catalog fidelity, filtering, stale-schema autoload and real local-data loading.
- `src/ui/character_creation_icons.rs` — decoded pixel/mask/cache/error regressions.
- Shared/server appearance tests — wire roundtrips, six historical storage schemas, temporary-database reopen and login roster preservation.
- `debug/character-create.js` — real offline controls, eyes/ears, name entry, camera actions and Back/Next, without character submission.

## Known gaps (current cycle)

- [ ] Pixel-perfect Retail visual parity has not been established. The contracts above are source-, layout- and native-layer-tested; no uninspected screenshot comparison or pixel-parity claim is made. Native additive glow, tooltip/hold-repeat details and unsupported effect families are not claimed complete.
- [ ] Local `ChrCustomizationReq.csv` is absent. General account/unlock eligibility is not implemented; existing class filtering is not full retail eligibility parity.
- [ ] Bone sets, conditional/skinned models, voice, animation-kit and other non-material/geoset effects remain unsupported or partial, as shown by the controls.
- [ ] Slider type 2 has no records in the local option data and is not implemented; types 0/1 are the supported contract for this data set.
- [ ] The local install reports build `12.1.0.69875`; independent version provenance of the extracted Interface source is unconfirmed. The local files themselves are the chosen reference.

## Out of scope

Unrelated game systems and replacement of `rsx!`/`Screen` authoring.
