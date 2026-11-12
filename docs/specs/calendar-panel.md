# Calendar panel

The in-world Forever Calendar panel is the native `CalendarFrame` opened from `GameTimeFrame`. It follows the Retail Calendar XML layout and shows only events from the existing server calendar snapshot. Rendering details belong in the [calendar panel system page](../wiki/systems/calendar-panel.md).

## What it must do

- [ ] Open and close from the Forever minimap date button.
- [ ] Render the 659×624 CalendarFrame at its XML anchor, with the authored border art, month/year title, weekday headers, and six rows of seven day cells.
- [ ] Use the Sunday-first week order defined by the cached Retail calendar source; highlight the current local day.
- [ ] Navigate months in both directions, including December-to-January year changes.
- [ ] Display only events received in `CalendarStateUpdate`; do not invent holidays or event rows when the server snapshot is empty.
- [ ] Keep the panel limited to the Forever skin. Add no calendar key binding unless the Retail source defines one.

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

## Known gaps (current cycle)

- [ ] Run the targeted behavior tests, helper dev/release builds, and format checks.
- [ ] Prove the open/close and month controls against a fresh private UDP 5995 server.
- [ ] Inspect and publish the Forever panel crops next to the authored Retail calendar art.

## Out of scope

- Creating, editing, inviting, or signing up for calendar events; the panel only displays server snapshots.
- Retail holidays, lockouts, or battleground events that the server does not provide.
- A Modern-skin Calendar panel or a new calendar key binding.
