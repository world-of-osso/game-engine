# Character-select roster

The native character-select roster uses `godot/ui-model/src/ui/screens/char_select_component.rs`. Retail defines the list and input behavior; Forever changes art only. [Host](../wiki/systems/godot-conversion.md).

## What it must do

- [x] Ten- and twelve-character rosters retain equal 95px cards, never compressed by the viewport; visible card portions stay inside the panel's clipped list viewport.
- [x] Create/Delete footer controls never intersect the list or any visible card.
- [x] Wheel movement uses two character pan extents; steppers use one; dragging the proportional thumb reaches both ends. Scrolling alone does not change selection or snap back to it.
- [x] Arrow-key selection keeps the selected card fully visible, including card10 and wraparound to card1.
- [x] Cards render names and level/race/class display text from local ChrRaces/ChrClasses; races95/96 use the active Forever Skyborne data in both skins, without hardcoded names or numeric-ID fallback.

## How it works

- [Native host](../wiki/systems/godot-conversion.md)

## Implementation inventory

- `godot/ui-model/src/ui/screens/char_select_component.rs` — bounded scroll viewport, fixed entries, footer reservation and selection visibility.
- `godot/ui-model/src/char_select_data.rs` — Retail name catalog and Skyborne-only data overlay, independent of skin.
- `godot/rust/src/lib.rs` — cached name loading and session roster projection.
- `godot/rust/src/ui/scroll_lists.rs` — native wheel, steppers and thumb input.
- `godot/rust/src/ui/mod.rs` — character-select `_input` owns its ScrollBox route; other canvases retain their host routes.
- `godot/rust/src/ui/projection.rs` — existing ScrollBox clipping and clipped hit testing.
- `godot/rust/src/ui/rosterfix_preview.rs` — no-argument offline12-entry Modern/Forever previews.

## Tests asserting this spec

- `godot/rust/src/ui/rosterfix_tests.rs` — native model/Taffy layout with10/12, both skins, selection and scroll input.
- `godot/ui-model/tests/roster_mapping.rs` — protocol text, active Skyborne names, edited catalog names and missing-ID errors.
- `godot/tests/capture_ui_screen.gd` (`rosterfix_both`) — real1920×1080 native controls, clipping, full text width, footer separation, card8/footer clicks, native wheel and thumb drag.

## Known gaps (current cycle)

No open roster-layout/name/input gap in this bounded fix. Offline previews do not prove authenticated relog, saved locations or 3D character rendering.

### Bounded proof — 2026-10-09

- `7c8370f57`: native-model RED0/7, reproducing33px(10)/26px(12) cards, footer overlap, absent scrolling and numeric labels.
- `e958e69aa`: native-model GREEN8/8; UI-model character_select6/6 + roster_mapping5/5. Later13px info-font change preserves those fixed-layout/name/scroll-algorithm scopes; native width checks cover the changed typography.
- `7cb5656f5`: extension build/export0 and offline cage capture0. Both skins:12 entries, selected10 visible, full display-text widths, clipped viewport, separate footer, native card8 selection versus footer creation, wheel194px and thumb-to-card12. Inspected1920×1080 PNGs copied byte-identically to `/syncthing/AgentShared/2026-10-09/rosterfix/{modern,forever}.png`.
- Native RED first caught15px label truncation (`28775d3f9` fixes typography), then missing character-select scroll-input ownership (`7cb5656f5` routes input). Neither failed capture was accepted as input proof.
- Logs/scripts/shim/manifest: `data/diagnostics/rosterfix-2026-10-09/`; build logs: `/home/osso/.worktrees/logs/rosterfix-*.log`. CPU tests omit renderer atlas initialization; native capture loaded art. Captures retain host Wayland/libdecor/FIFO/V-Sync/XWayland diagnostics; no unrelated platform fix. Owned slice stopped; no server, UDP5000, push/merge or independent-agent run.

## Out of scope

Roster protocol has no saved location/area field; retain its existing status rather than inventing a location. No server/protocol change, auth or character-model rendering fix.

## Retail references

Local root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectList.xml:53–83` — ScrollBox anchored above separate footer and MinimalScrollBar.
- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectList.lua:150–202,629–637` — element extent,2px spacing, character pan extent and selected-character scrolling.
- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectListElements.xml:100` —347×95 character entry.
- `Blizzard_GlueXML/Mainline/CharacterSelect/CharacterSelectListElements.lua:743–782` —name,level/class and area text (race text is the explicit local requirement).
- `Blizzard_SharedXML/Shared/Scroll/ScrollController.lua:77–99,152–154` and `ScrollBoxListView.lua:637–661` —wheel scalar2, pan including spacing.
