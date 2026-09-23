# Character Creation

Character creation is registry-authored `rsx!` UI over a 3D preview. The selected local Retail Interface files are a layout/control reference, not proof of complete API or unlock parity.

## Current architecture

- `CharacterAppearance` retains `sex` and six core index selectors. Additional non-core choices are ordered `(option_id, choice_id)` pairs; the core selector remains the only authority for its option.
- The local customization cache imports authored option/category IDs, labels, ordering, icon element IDs, UI type, signed swatches, requirement IDs and choice effect metadata. Cache freshness is version- and source-aware; it rebuilds through the importer rather than requiring manual cache deletion.
- The engine exposes 1,147 options in 59 categories from the local data. This is catalog coverage, not proof that every effect can render. A choice with supported material/geoset effects plus unsupported metadata remains selectable and its category shows a partial-support notice; an unsupported-only choice stays disabled rather than inert.
- Existing material/geoset rendering resolves one effective selected-choice set: core selectors plus disjoint additional pairs. This preserves working partial effects such as face materials while keeping unsupported effects explicit.
- Local Retail XML establishes the intended race/class grid, bottom navigation, category tabs, options column, camera controls, dropdown/checkbox behavior and authored control dimensions. Installed build metadata reports `12.1.0.69875`; independent provenance of the extracted Interface files is not established.

## Persistence boundary

Shared protocol and server persistence preserve additional pairs through creation, redb reopen and login roster construction. Frozen historic bitcode schemas upgrade once to empty additional pairs; unknown bytes fail explicitly. This does not validate catalog eligibility server-side yet.

## Reference assets

Race/class icons and the customization atlas resolve through local CASC FileDataIDs. Build-pinned Wago DB2 CSV exports for `12.1.0.69875` establish the palette bounds on atlas 708 / FDID `1253496`: `charactercreate-customize-palette` is `[519,471..603,491]`; `-half` is `[729,471..813,491]`. Earlier CSV metadata instead selected a yellow ornament. Fresh local-CASC extraction matched the cached BLP byte-for-byte, so `ui-toolkit` `78c67e7` corrects metadata, not asset bytes. The matching slice-data export also gives `common-dropdown-c-bg` element 25590 asymmetric nine-slice margins 23/18/23/28. Engine `83490974` applies that `NineSlice` after either `Screen::sync` path; a whole-image stretch is incorrect. Native source-pixel RED/GREEN is `a55af09f`.

Dropdown rows follow local Retail `MenuStyle2` content insets (left 3, top 6, right 3, bottom 7). Single-column details occupy 116 pixels plus ResizeLayoutFrame padding for a 144-pixel row. Multi-column rows use 107 pixels for colors, 136 for names, and 70 for numeric choices; those widths determine column packing. A selectable color starts after the 25-pixel number field. Dual colors use half-palette then full-palette art, secondary-only colors occupy the first slot, and the selected 51×20 outline starts four pixels before that effective first swatch. Engine `4cfb538f`, `9761008e`, and `627cb5d6` implement these rules.

Closed dropdown `SelectionDetails` uses the same Retail `ResizeLayoutFrame` contract: its 144-pixel XML size is initial, then content is centered at 42 pixels for one effective swatch or 54 for dual swatches; named text is capped at 126. Engine `fa162861` applies this without moving the 150-pixel trigger or its input area. Circular controls follow `RingedMaskedButtonMixin:UpdateHighlightTexture`: checked hover matches `CheckedTexture`; unchecked hover matches the authored `Ring`. Engine `c022b454` authors those sizes; toolkit support and final rendered verification remain pending. Atlas crop and native-layout proof are not full rendered parity, and native additive glow remains unsupported.

## Authored 3D preview backdrops

`ChrRaces.CreateScreenFileDataID` supplies the backdrop model: Alliance `623712`, Horde `623714`, and neutral Pandaren `623716`. Race 25 reuses Alliance and race 26 reuses Horde; neutral race 24 is loader-supported without adding an actor or changing the selectable roster.

The local-CASC cache currently contains the three `.m2` files with 12 `.skin` files and 101 referenced textures. Each backdrop parses camera snapshot zero and root attachment 0, then normalizes the attachment to preview origin while rotating its camera axis to `+Z`; the transformed eye/focus, FOV, near and far values seed the existing orbit camera. This is a first-key/static snapshot, not camera-track evaluation. Presentation `customize_scale` now scales the preview actor, and `camera_distance_offset` adjusts its authored default orbit distance; rotate, zoom, reset and face-focused controls remain intact. At `24fce090`, 30 scene tests passed (one ignored), covering mapped-scene replacement/exit cleanup and the authored camera/presentation lifecycle; `126ab4aa` changes only module ordering and independently passes `cargo check` plus scoped library formatting.

Backdrop ambient uses the authored type-0 M2 light color at time/sequence zero, converted to linear RGB. Its brightness is the reciprocal of Bevy's default camera exposure, preserving the authored dimensionless multiplier before Bevy's BRDF; the camera explicitly retains the same numeric default exposure. The generic 8,000-lux directional fill was removed. The procedural `SkyEnvMapHandle` was never bound to the character-creation camera and is no longer created, so it is not evidence of active IBL or a cause of washout. Backdrop point lights still use the ordinary static-M2 light-spawn pipeline. Backdrop-owned `StandardMaterial`s use `reflectance = 0` and `perceptual_roughness = 1`, matching the M2-effect/UI-model diffuse reference; other materials are unchanged. This approximates PBR/light units and does **not** claim exact WoW or Retail shading parity.

Focused RED/GREEN evidence covers removal of the extra directional light and unused procedural map, authored ambient conversion, backdrop diffuse material settings, presentation scale/distance application, and neutral loader-only GPU capture. See `data/diagnostics/charcreate-authored-scenes-20260923/`, including `neutral-diffuse-lighting-gpu.log`. The name-entry font is now Arial Narrow at 20 logical pixels; its focused glyph test passes at `24fce090`, while a native runtime dump confirms `Donagh` with cursor position 6 in the unchanged 300×38 field. A new GUI capture under the revised lighting and complete three-backdrop visual acceptance remain pending; unreliable visual-helper output is not acceptance or defect evidence, and this does not claim exact Retail parity.

## Known limits

- General `ChrCustomizationReq` evaluation is unavailable locally; requirement and visibility IDs are preserved but not interpreted as Retail account/unlock policy.
- Partial material/geoset output does not prove every element effect for that choice is rendered; unsupported-only choices remain disabled.
- Closed-value centering and circular hover sizing have independent native pointer/pixel/geometry/input proof and inspected owned-window captures. See `data/diagnostics/charcreate-hover-select-20260922/completion-report.md`; popup styling and hit areas remain unchanged.
- Final rendered dropdown comparison, additive swatch glow, and full Retail visual parity remain unproven. Nine-part background projection, opaque center, label ordering and idle-state dirty behavior have native coverage; runtime/final verification remains separate. Catalog/control integration, camera interaction and create/save/reload acceptance have separate evidence boundaries.

## Sources

- [character-creation spec](../../specs/character-creation.md) — product contract and gaps
- `https://wago.tools/db2/{UiTextureAtlas,UiTextureAtlasElement,UiTextureAtlasMember,UiTextureAtlasElementSliceData}/csv?build=12.1.0.69875` — build-pinned metadata exports saved under `data/diagnostics/charcreate-button-style-20260922/build-12.1.0.69875/`
- [UI system](ui-system.md) — registry/native UI and asset projection
- [character rendering](character-rendering.md) — material/geoset application
- [asset pipeline](asset-pipeline.md) — local CASC/FileDataID contract
- `src/scenes/char_create/{background,background_data,scene}.rs` — authored backdrop catalog, framing normalization, lighting/material conversion and scene ownership; 30 passing scene tests plus one ignored at `24fce090`
- `src/ui/screens/char_create_component/{char_create_widgets,name_button_tests}.rs` — 20-logical-pixel Arial Narrow name entry and focused glyph regression at `24fce090`
- `data/diagnostics/charcreate-authored-scenes-20260923/` — scoped RED/GREEN source, GPU and runtime evidence; `NameGen-12.1.0.69875.csv` is the pinned Wago acquisition input, while `data/NameGen.csv` is an untracked generated cache.
- `src/asset/m2_format/{m2_camera,m2_attach}.rs` — snapshot-zero camera and corrected root-attachment parsing
- [shared customization contract](../../../shared-protocol/docs/specs/character-customization.md) — appearance payload
- [server customization storage](../../../game-server/docs/specs/character-customization-storage.md) — historic persistence upgrade
- [Retail atlas contract](../../../ui-toolkit/docs/specs/character-creation-atlases.md) — bounded atlas metadata

## See Also

- [[ui-system]] — authored controls and native projection
- [[character-rendering]] — effective choice application
- [[asset-pipeline]] — FileDataID resolution
