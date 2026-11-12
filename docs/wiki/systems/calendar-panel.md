# Native Calendar panel

VERIFIED, CODE: The in-world `CalendarFrame` is a Forever-only panel that opens from the minimap `GameTimeFrame` and displays the server's calendar snapshot; it does not create holidays or events.

## Entry and lifecycle

- VERIFIED, CODE: `GameClient::toggle_calendar` ignores Modern and toggles the panel from the local date for Forever.
- VERIFIED, CODE: `GameClient::update_calendar` closes the panel outside InWorld or on Modern, consumes its native actions, and refreshes its view when the model changes.
- VERIFIED, CODE: `CalendarStateUpdate` snapshots replace the model's event list; each event's start timestamp maps to a local calendar date.

## Presentation

- VERIFIED, CODE: `calendar_frame_component.rs` builds the 659×624 source-positioned frame, authored border textures, month/year text, Sunday-first weekday row, and 42 day buttons.
- VERIFIED, CODE: The current-day glow uses the source-sized texture and additive blend at the centered Retail offsets. Date text renders above the highlight, and weekday labels use `BACKGROUND` so the `ARTWORK` year remains visible.
- VERIFIED, TEST: `today_glow_and_year_header_match_retail_layers` checks the glow anchor/blend and year/weekday draw layers.
- VERIFIED, TEST: `calendar_shows_only_server_events_and_the_filter_hides_them` checks empty snapshots, a concrete server-event snapshot, and the filter.

## Live proof and limits

- VERIFIED, LIVE: A private client on `127.0.0.1:5995` opened the panel, navigated November 2026 to December and back, closed it, reopened it from `GameTimeFrame`, and returned a live `CalendarFrame` tree without automation errors.
- VERIFIED, LIVE: The final screenshot and inspected crops are in `/syncthing/AgentShared/2026-11-11/calendar-panel/`; the interaction script is `calendar-panel-interaction-live.js`.
- UNVERIFIED, LIVE: The final image has no event title, so it does not prove visual rendering of a non-empty live server snapshot. The model test covers that data path.

## Sources

- [Feature spec](../../specs/calendar-panel.md)
- [Calendar model](../../../godot/ui-model/src/calendar_data.rs)
- [Calendar presentation](../../../godot/ui-model/src/ui/screens/calendar_frame_component.rs)
- [Calendar model tests](../../../godot/ui-model/tests/calendar.rs)
- [Native lifecycle](../../../godot/rust/src/calendar.rs)
- [Minimap entry point](../../../godot/rust/src/minimap.rs)
- [Account event dispatch](../../../godot/rust/src/account.rs)
- [Wire tests](../../../godot/network/src/calendar_wire_tests.rs)
- [Account tests](../../../godot/rust/src/account_calendar_tests.rs)

## See Also

- [[minimap]] — the Forever date-button entry point
- [[ui-system]] — native frame projection and draw layers
