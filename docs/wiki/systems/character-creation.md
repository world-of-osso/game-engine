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

Race/class icons and the customization atlas resolve through local CASC FileDataIDs. Toolkit has bounded Retail control/category atlas crops and element-ID lookup. The circular portrait mask uses local FDID `130924`; engine-side composed icon support is in progress. Atlas crop proof is not full rendered parity, and native additive glow remains unsupported.

## Known limits

- General `ChrCustomizationReq` evaluation is unavailable locally; requirement and visibility IDs are preserved but not interpreted as Retail account/unlock policy.
- Partial material/geoset output does not prove every element effect for that choice is rendered; unsupported-only choices remain disabled.
- Category/control screen integration, camera interaction, rendered comparison and create/save/reload acceptance are current-cycle work.

## Sources

- [character-creation spec](../../specs/character-creation.md) — product contract and gaps
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
