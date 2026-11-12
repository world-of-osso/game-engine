# Native Calendar panel

VERIFIED, CODE: The in-world `CalendarFrame` is a Forever-only panel opened from the minimap `GameTimeFrame`; it displays the server calendar snapshot and does not create local holidays or events.

## Entry and lifecycle

- VERIFIED, CODE: `GameClient::toggle_calendar` ignores Modern and toggles the panel from the local date in Forever.
- VERIFIED, CODE: `GameClient::update_calendar` closes the panel outside InWorld or on Modern, consumes native actions, and refreshes the view when the model changes.
- VERIFIED, CODE: `CalendarStateUpdate` replaces the model event list; each start timestamp maps to a local calendar date.

## Presentation

- VERIFIED, CODE: `calendar_frame_component.rs` builds the 659×624 frame, source border slices, month/year text, Sunday-first weekday row, and 42 day buttons.
- VERIFIED, TEST: `calendar_frame_emits_retail_grid_and_anchor_art` checks the 42 cell bounds and atlas quadrants, weekday tiles, year/today anchors, empty event art, and both previous- and next-month dark overlays.
- VERIFIED, CODE: `CalendarDay.in_viewed_month` gates `CalendarDayButtonDarkFrame`. The native frame uses FileDataID 235432 `CalendarShadows`, with source top/bottom regions, 91×45/46 bounds, 0.9 tint alpha, and draw order above date labels.
- VERIFIED, LIMIT: The native frame uses the base previous/next-month UV patches; Retail's side- and corner-specific dark-day UV variants are not implemented.
- VERIFIED, CODE: `CalendarYearBackground` and `CalendarYearName` now use `OVERLAY` at their unchanged XML positions. This keeps both above the overlapping month plaque. The current-day glow keeps its source size, additive blend and corrected anchor.
- VERIFIED, SOURCE: The inspected Retail Calendar XML has no root backdrop. The panel does not add a full-frame background.
- VERIFIED, TEST: `calendar_shows_only_server_events_and_the_filter_hides_them` checks an empty snapshot, a concrete server event and the filter.

## Live proof and limits

- VERIFIED, TEST: The focused calendar integration filter passes 5/5 tests.
- VERIFIED, BUILD: The native dev helper build and local release helper build pass; `cargo fmt --all` passes.
- VERIFIED, LIVE: Private UDP `127.0.0.1:5995` automation opened the panel, navigated November 2026 to December and back, closed it, and reopened it from `GameTimeFrame`.
- VERIFIED, TREE: The fresh UI tree has 12 dark frames for December 1–12, no dark frame for November 29–30, 24 `CalendarShadows` textures and no `EventBackground` frames.
- VERIFIED, IMAGE: The fresh 3× header/grid crop shows all day tiles with no world bleed between or around cells: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-header-grid-after-followup-3x.png`.
- VERIFIED, IMAGE: The 4× bottom crop shows November 29–30 at normal brightness and December 1–12 darkened: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-out-of-month-after-followup-4x.png`.
- VERIFIED, IMAGE: The 8× header crop shows the full `2026` label above the month-title art at its XML anchor: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-year-label-after-followup-8x.png`.
- VERIFIED, IMAGE: The full fresh panel capture is `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-open-after-followup.webp`.
- VERIFIED, CLEANUP: The owned client and server stopped, and UDP port 5995 is free. The private server database and admin socket were under the engine target; no build or data write occurred in the game-server checkout.
- UNVERIFIED, LIVE: The capture has no event title, so visible rendering of a non-empty server snapshot is not proven. The model test covers the snapshot path.

## Sources

- [Feature spec](../../specs/calendar-panel.md)
- [Calendar rendering investigation](../investigations/calendar-panel-rendering.md)
- [Calendar model](../../../godot/ui-model/src/calendar_data.rs)
- [Calendar presentation](../../../godot/ui-model/src/ui/screens/calendar_frame_component.rs)
- [Calendar model tests](../../../godot/ui-model/tests/calendar.rs)
- [Native lifecycle](../../../godot/rust/src/calendar.rs)
- [Minimap entry point](../../../godot/rust/src/minimap.rs)
- [Account event dispatch](../../../godot/rust/src/account.rs)
- [Wire tests](../../../godot/network/src/calendar_wire_tests.rs)
- [Account tests](../../../godot/rust/src/account_calendar_tests.rs)
- Retail cached source: `Blizzard_Calendar.xml:187–213`, `Blizzard_CalendarTemplates.xml:94–118`, and `Mainline/Blizzard_Calendar.lua:160–1453`.

## See Also

- [[minimap]] — the Forever date-button entry point
- [[ui-system]] — native frame projection and draw layers
