# Calendar panel rendering gaps

Two visible CalendarFrame defects remain after the grid and title-anchor correction: other-month dates lack Retail's dark-day treatment, and the year label is clipped by the month title art. This page records the source behavior, observed pixels, and follow-up requirements.

## Other-month dates

- VERIFIED, CODE: `CalendarDay.in_viewed_month` correctly identifies dates outside the viewed month, but `calendar_frame_component.rs` renders every date label with the normal color and emits no `CalendarShadows` overlay.
- VERIFIED, SOURCE: Retail `CalendarFrame_UpdateDay` shows each other-month button's `CalendarDayButtonDarkFrame`, sets top/bottom UVs from the dark flags, and hides the frame for viewed-month dates. The XML template uses two 91×45/46 `CalendarShadows` regions with alpha 0.9; the texture is FileDataID 235432.
- VERIFIED, IMAGE: The 4× crop shows November 29–30 at normal brightness and December 1–12 at the same brightness. The December cells need the Retail dark-day treatment; November 29–30 must stay unchanged. Evidence: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-out-of-month-before-followup-4x.png`.

## Year label

- VERIFIED, CODE: `CalendarMonthBackground` uses `OVERLAY`, while `CalendarYearBackground` and `CalendarYearName` use `ARTWORK`; their source-positioned rectangles overlap between y=24 and y=37.
- VERIFIED, SOURCE: Retail XML places the month background on `OVERLAY` and the year art/name on `ARTWORK`, with the year anchored below the month art at its authored offset.
- VERIFIED, IMAGE: The 4× header crop shows the upper part of `2026` obscured by the month-title art. Evidence: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-header-before-followup-4x.png`.
- INFERRED: The draw-layer order puts the month backplate above the overlapping year label. The follow-up must keep the XML anchor and show the full label.

## Grid and backdrop checks

- VERIFIED, IMAGE: The fresh 3× source-versus-engine comparison shows no world bleed between or around the day tiles: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-source-vs-engine-3x.png`.
- VERIFIED, SOURCE: The inspected Calendar XML has no root backdrop. The panel must not add an invented full-frame background.

## Sources

- [Calendar panel system](../systems/calendar-panel.md)
- [Calendar panel spec](../../specs/calendar-panel.md)
- [Calendar model](../../../godot/ui-model/src/calendar_data.rs)
- [Calendar presentation](../../../godot/ui-model/src/ui/screens/calendar_frame_component.rs)
- Retail cached source: `Blizzard_Calendar.xml:187–213`, `Blizzard_CalendarTemplates.xml:94–118`, and `Mainline/Blizzard_Calendar.lua:160–1453`.

## See Also

- [[ui-system]] — frame strata and draw-layer projection
