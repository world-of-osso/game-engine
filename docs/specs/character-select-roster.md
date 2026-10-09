# Character-select roster

The native character-select roster uses `godot/ui-model/src/ui/screens/char_select_component.rs`. Retail defines the list and input behavior; Forever changes art only. [Host](../wiki/systems/godot-conversion.md).

## What it must do

- [ ] Ten- and twelve-character rosters retain equal 95px cards, never compressed by the viewport; visible card portions stay inside the panel's clipped list viewport.
- [ ] Create/Delete footer controls never intersect the list or any visible card.
- [ ] Wheel movement uses two character pan extents; steppers use one; dragging the proportional thumb reaches both ends. Scrolling alone does not change selection or snap back to it.
- [ ] Arrow-key selection keeps the selected card fully visible, including card10 and wraparound to card1.
- [ ] Cards render names and level/race/class display text from local ChrRaces/ChrClasses; races95/96 use the active Forever Skyborne data in both skins, without hardcoded names or numeric-ID fallback.

## How it works

- [Native host](../wiki/systems/godot-conversion.md)

## Implementation inventory

- `godot/ui-model/src/ui/screens/char_select_component.rs` — bounded scroll viewport, fixed entries, footer reservation and selection visibility.
- `godot/ui-model/src/char_select_data.rs` — Retail name catalog and Skyborne-only data overlay, independent of skin.
- `godot/rust/src/lib.rs` — cached name loading and session roster projection.
- `godot/rust/src/ui/scroll_lists.rs` — native wheel, steppers and thumb input.
- `godot/rust/src/ui/projection.rs` — existing ScrollBox clipping and clipped hit testing.

## Tests asserting this spec

- `godot/rust/src/ui/rosterfix_tests.rs` — native model/Taffy layout with10/12, both skins, selection and scroll input.
- `godot/ui-model/tests/roster_mapping.rs` — protocol text, active Skyborne names, edited catalog names and missing-ID errors.

## Known gaps (current cycle)

- [ ] GREEN targeted tests and inspected offline12-entry captures pending.

## Out of scope

Roster protocol has no saved location/area field; retain its existing status rather than inventing a location. No server/protocol change, auth or character-model rendering fix.

## Retail references

Local root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectList.xml:53–83` — ScrollBox anchored above separate footer and MinimalScrollBar.
- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectList.lua:150–202,629–637` — element extent,2px spacing, character pan extent and selected-character scrolling.
- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectListElements.xml:100` —347×95 character entry.
- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectListElements.lua:743–782` —name,level/class and area text (race text is the explicit local requirement).
- `Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:77–99,152–154` and `ScrollBoxListView.lua:637–661` —wheel scalar2, pan including spacing.
