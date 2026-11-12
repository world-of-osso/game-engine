# Calendar panel

The in-world Forever Calendar panel is the native `CalendarFrame` opened from `GameTimeFrame`. It follows the Retail Calendar XML layout and shows only events from the existing server calendar snapshot. Rendering details belong in the [calendar panel system page](../wiki/systems/calendar-panel.md).

## What it must do

- [x] Open and close from the Forever minimap date button.
- [ ] Render the 659×624 CalendarFrame at its XML anchor, with authored border art, month/year title, weekday headers, and six rows of seven 91×91 day cells using the Retail day-atlas subrects at edge-to-edge XML positions.
- [x] Use the Sunday-first week order defined by the cached Retail calendar source; highlight the current local day.
- [ ] Place the year and current-day textures at their XML anchors, and omit event-bar art when the server snapshot is empty.
- [x] Navigate months in both directions, including December-to-January year changes.
- [x] Display only events received in `CalendarStateUpdate`; do not invent holidays or event rows when the server snapshot is empty.
- [x] Keep the panel limited to the Forever skin. Add no calendar key binding unless the Retail source defines one.

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

- [ ] Targeted calendar rendering tests, helper dev/release builds, and format checks pass.
- [ ] Private UDP `127.0.0.1:5995` proof covers opening, both month controls, close, and reopen.
- [ ] Inspect and save the source-texture reference, Forever panel screenshot, and 3× crop under `/syncthing/AgentShared/2026-11-12/calendar-panel/`.

## Known gaps (current cycle)

- [ ] Verify the corrected day tiles hide the world between and around the grid in the private capture.

UNVERIFIED, LIVE: The final screenshot contains no event title, so it does not prove visual rendering of a non-empty live server snapshot; the model test covers event display from a server snapshot fixture.

## Out of scope

- Creating, editing, inviting, or signing up for calendar events; the panel only displays server snapshots.
- Retail holidays, lockouts, or battleground events that the server does not provide.
- A Modern-skin Calendar panel or a new calendar key binding.
