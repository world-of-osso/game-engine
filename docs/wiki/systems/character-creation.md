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

Dropdown rows follow local Retail `MenuStyle2` content insets (left 3, top 6, right 3, bottom 7). Single-column details occupy 116 pixels plus ResizeLayoutFrame padding for a 144-pixel row. Multi-column rows use 107 pixels for colors, 136 for names, and 70 for numeric choices; those widths determine column packing. A selectable color starts after the 25-pixel number field. Dual colors use half-palette then full-palette art, secondary-only colors occupy the first slot, and the selected 51×20 outline starts four pixels before that effective first swatch. Engine `4cfb538f`, `9761008e`, and `627cb5d6` implement these rules. Atlas crop and native-layout proof are not full rendered parity, and native additive glow remains unsupported.

## Known limits

- General `ChrCustomizationReq` evaluation is unavailable locally; requirement and visibility IDs are preserved but not interpreted as Retail account/unlock policy.
- Partial material/geoset output does not prove every element effect for that choice is rendered; unsupported-only choices remain disabled.
- Final rendered dropdown comparison, additive swatch glow, and full Retail visual parity remain unproven. Nine-part background projection, opaque center, label ordering and idle-state dirty behavior have native coverage; runtime/final verification remains separate. Catalog/control integration, camera interaction and create/save/reload acceptance have separate evidence boundaries.

## Sources

- [character-creation spec](../../specs/character-creation.md) — product contract and gaps
- `https://wago.tools/db2/{UiTextureAtlas,UiTextureAtlasElement,UiTextureAtlasMember,UiTextureAtlasElementSliceData}/csv?build=12.1.0.69875` — build-pinned metadata exports saved under `data/diagnostics/charcreate-button-style-20260922/build-12.1.0.69875/`
- [UI system](ui-system.md) — registry/native UI and asset projection
- [character rendering](character-rendering.md) — material/geoset application
- [asset pipeline](asset-pipeline.md) — local CASC/FileDataID contract
- [shared customization contract](../../../shared-protocol/docs/specs/character-customization.md) — appearance payload
- [server customization storage](../../../game-server/docs/specs/character-customization-storage.md) — historic persistence upgrade
- [Retail atlas contract](../../../ui-toolkit/docs/specs/character-creation-atlases.md) — bounded atlas metadata

## See Also

- [[ui-system]] — authored controls and native projection
- [[character-rendering]] — effective choice application
- [[asset-pipeline]] — FileDataID resolution
