# Character-select roster overflow

Ten-character native rosters compressed cards to33px, overlapped names and let the Create Character footer intercept card8. The [roster contract](../../specs/character-select-roster.md) owns requirements, Retail references and bounded proof.

## Root cause

The card requested95px but lived in a420px flex column with10px gaps. Native Taffy applied flex shrink (33px for10;26px for12), while absolutely positioned text/art retained their authored dimensions. The list extended into the footer and had neither scroll geometry nor clipping.

Roster projection formatted numeric race/class IDs. After replacing those IDs with catalog names, a native font-width assertion caught truncation at15px. A further native input assertion caught a separate ownership gap: character-select had no host forwarding into `scroll_list_input`; adding a scroll list alone did not enable wheel/drag input.

## Correction

Entries have absolute positions and fixed95px height, independent of list length. One panel-bounded ScrollBox clips drawing and hit testing; its MinimalScrollBar uses the existing native handlers. Character-select's RegistryUi owns this input route; other canvases retain their host routes. Footer controls sit outside the list.

Selection changes pan just enough to reveal the whole selected card; wheel/drag rebuilds with unchanged selection do not snap back. Retail2px spacing gives a97px pan and194px wheel movement.

The cached local ChrRaces/ChrClasses name catalog uses Retail data plus the same95/96 Forever Skyborne overlay as body-model resolution, independent of art skin. Missing names are explicit errors, not numeric/Unknown fallback text. The13px info label fits full Skyborne race/class text; preview checks native glyph width.

## Sources

- [Character-select roster contract](../../specs/character-select-roster.md) — Retail source citations, code/test inventory, exact RED/GREEN/capture proof and exclusions.
- `godot/ui-model/src/ui/screens/char_select_component.rs` — layout and selection visibility.
- `godot/ui-model/src/char_select_data.rs` — data-backed names.
- `godot/rust/src/ui/mod.rs` — native character-select input ownership.

## See Also

- [[godot-conversion]] — native host.
- [[forever-data]] — Skyborne data ownership; art skin does not change character identity.
