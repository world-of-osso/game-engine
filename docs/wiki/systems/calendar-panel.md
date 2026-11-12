# Native Calendar panel

VERIFIED, CODE: The in-world `CalendarFrame` is a Forever-only panel opened from the minimap `GameTimeFrame`; it displays the server calendar snapshot and does not create local holidays or events.

## Entry and lifecycle

- VERIFIED, CODE: `GameClient::toggle_calendar` ignores Modern and toggles the panel from the local date in Forever.
- VERIFIED, CODE: `GameClient::update_calendar` closes the panel outside InWorld or on Modern, consumes native actions, and refreshes the view when the model changes.
- VERIFIED, CODE: `CalendarStateUpdate` replaces the model event list; each start timestamp maps to a local calendar date.

## Presentation

- VERIFIED, CODE: `calendar_frame_component.rs` builds the 659×624 frame, source border slices, month/year text, Sunday-first weekday row, and 42 day buttons.
- VERIFIED, TEST: `calendar_frame_emits_retail_grid_and_anchor_art` checks all 42 cell bounds and atlas quadrants, weekday tiles, year/today anchors, and empty event-background output.
- VERIFIED, CODE: The current-day glow uses the source-sized texture, additive blend, and corrected Retail offset. The year label renders above weekday labels.
- VERIFIED, SOURCE: Retail `CalendarFrame_UpdateDay` shows `CalendarDayButtonDarkFrame` for other-month dates and sets its `CalendarShadows` top/bottom texture coordinates; the template gives both overlays alpha 0.9. The texture is FileDataID 235432.
- VERIFIED, CODE: The model already sets `CalendarDay.in_viewed_month`, but the component currently emits no dark-day overlay.
- VERIFIED, TEST: `calendar_shows_only_server_events_and_the_filter_hides_them` checks an empty snapshot, a concrete server event, and the filter.

## Live proof and remaining visual gaps

- VERIFIED, LIVE: Private UDP `127.0.0.1:5995` automation opened the panel, navigated November 2026 to December and back, closed it, and reopened it from `GameTimeFrame`; the UI tree has no automation errors.
- VERIFIED, IMAGE: The 3× source-versus-engine crop shows day-tile art across the full grid with no world bleed between cells: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-source-vs-engine-3x.png`.
- VERIFIED, IMAGE: The 4× bottom-row crop shows November 29–30 at normal brightness and December 1–12 also at normal brightness; the latter still need the Retail dark-day state: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-out-of-month-before-followup-4x.png`.
- VERIFIED, CODE: The component puts `CalendarMonthBackground` on `OVERLAY` and `CalendarYearName`/`CalendarYearBackground` on `ARTWORK`; their rectangles overlap from y=24 to y=37.
- VERIFIED, IMAGE: The 4× header crop shows the top of `2026` obscured by the month-title art: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-header-before-followup-4x.png`.
- INFERRED: The higher `OVERLAY` month art draws over the `ARTWORK` year label and causes the visible clipping; the required fix must keep the year at its XML anchor and make the full label visible.
- VERIFIED, SOURCE: The inspected Retail XML has no root backdrop; the source defines calendar regions only, and the native frame adds no full-frame background.
- UNVERIFIED, LIVE: The capture has no event title, so it does not prove visible rendering of a non-empty live server snapshot. The model test covers the snapshot data path.
- VERIFIED, CLEANUP: The owned client and server stopped after capture, and private UDP port 5995 is free.

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
- Retail cached source: `Blizzard_Calendar.xml:187–213`, `Blizzard_CalendarTemplates.xml:94–118`, and `Mainline/Blizzard_Calendar.lua:160–1437`.

## See Also

- [[minimap]] — the Forever date-button entry point
- [[ui-system]] — native frame projection and draw layers
- [[calendar-panel-rendering]] — root causes and evidence for the remaining visual gaps
