# Calendar panel

The in-world Forever Calendar panel is the native `CalendarFrame` opened from `GameTimeFrame`. It follows the Retail Calendar XML layout and shows only events from the existing server calendar snapshot. Rendering details belong in the [calendar panel system page](../wiki/systems/calendar-panel.md).

## What it must do

- [x] Open and close from the Forever minimap date button.
- [x] Render 42 day buttons as 91×91 cells at edge-to-edge positions, using all four Retail day-atlas quadrants.
- [x] Use a Sunday-first week and highlight the current local day.
- [ ] Render the authored outer border and month/year header at their Retail anchors.
- [x] Place year/current-day art at the tested source anchors, keep the year above weekday labels, and omit event-bar art for empty days.
- [x] Navigate both directions, including December-to-January year changes.
- [x] Display only events received in `CalendarStateUpdate`; do not invent holidays or event rows when the server snapshot is empty.
- [x] Keep the panel limited to the Forever skin. Add no calendar key binding unless the Retail source defines one.
- [ ] Draw Retail `CalendarShadows` top/bottom overlays above every out-of-month date; leave viewed-month cells unchanged.
- [ ] Keep the full year plaque and label visible above the month-title art without moving their XML anchors.

## How it works

- [calendar panel system](../wiki/systems/calendar-panel.md)
- [window manager](../wiki/systems/ui-system.md)

## Implementation inventory

- `godot/ui-model/src/calendar_data.rs` — month state, Sunday-first day grid, and server event dates.
- `godot/ui-model/src/ui/screens/calendar_frame_component.rs` — CalendarFrame art, grid, controls, and event presentation.
- `godot/rust/src/calendar.rs` — native window lifecycle and UI actions.
- `godot/rust/src/minimap.rs` — Forever GameTimeFrame click dispatch.
- `godot/network/src/lib.rs` and `godot/rust/src/account.rs` — server snapshot reception and account event dispatch.

## Tests asserting this spec

- `godot/ui-model/tests/calendar.rs`
- `godot/rust/src/account_calendar_tests.rs`

## Verification

- [ ] Re-run the calendar integration tests, native dev and local release helper builds, and format checks after the visual follow-up.
- [ ] Capture the panel on private UDP `127.0.0.1:5995` and confirm the full year label and correct out-of-month shading.
- [ ] Compare the fresh panel capture with the authored texture reconstruction; inspect the 3× grid crop for world bleed.

## Known gaps (current cycle)

- [ ] Verify that the dark-day overlays leave November 29–30 unchanged and dim December 1–12.
- [ ] Verify that `2026` is fully visible above the month-title art at its existing XML anchor.

UNVERIFIED, LIVE: The final screenshot contains no event title, so it does not prove visual rendering of a non-empty live server snapshot; the model test covers display from a concrete server snapshot fixture.

## Out of scope

- Creating, editing, inviting, or signing up for calendar events; the panel only displays server snapshots.
- Retail holidays, lockouts, or battleground events that the server does not provide.
- A Modern-skin Calendar panel or a new calendar key binding.
- A full-frame backdrop; the checked Retail XML defines no root backdrop, and no backdrop may be invented.
