# Calendar panel shading and year-label visibility

A pre-follow-up capture showed two CalendarFrame defects: other-month dates were not dimmed, and the year label was clipped by the month-title art. This investigation records the causes, fixes and final private evidence.

## Other-month shading

- VERIFIED, PRE-FOLLOW-UP IMAGE: November 29–30 were at normal brightness, but December 1–12 were also bright. Evidence: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-out-of-month-before-followup-4x.png`.
- VERIFIED, SOURCE: Retail `CalendarFrame_UpdateDay` shows `CalendarDayButtonDarkFrame` for dates outside the viewed month and sets separate top/bottom `CalendarShadows` UVs. The XML template uses 91×45/46 overlays with alpha 0.9; the texture is FileDataID 235432.
- VERIFIED, CODE: The calendar model already marks `CalendarDay.in_viewed_month`. The component now emits a raised dark frame for each other-month date, uses base previous/next-month UV patches, and loads FileDataID 235432 when those overlays are present.
- VERIFIED, TEST: The calendar presentation test checks that November 29–30 have no dark frame, December 1–12 do, their overlays draw above date labels, and December's previous-month dates also use a dark frame.
- VERIFIED, IMAGE: The final 4× crop shows November 29–30 at normal brightness and December 1–12 darkened: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-out-of-month-after-followup-4x.png`.
- VERIFIED, LIMIT: The component uses the base previous/next-month UV patches. It does not implement Retail's side- and corner-specific dark-day UV variants.

## Year-label clipping

- VERIFIED, PRE-FOLLOW-UP CODE: `CalendarMonthBackground` used `OVERLAY`, while `CalendarYearBackground` and `CalendarYearName` used `ARTWORK`; their source-positioned rectangles overlap from y=24 to y=37.
- INFERRED: The higher month plaque drew over the overlapping year label and hid its upper pixels.
- VERIFIED, CODE: The year plaque and label now use `OVERLAY`, above the month plaque, while their XML positions remain y=24 and y=25.
- VERIFIED, PRE-FOLLOW-UP IMAGE: The 4× crop shows the top of `2026` obscured by the month-title art: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-header-before-followup-4x.png`.
- VERIFIED, IMAGE: The final 8× crop shows the full `2026` label at the unchanged anchor: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-year-label-after-followup-8x.png`.

## Grid and backdrop checks

- VERIFIED, IMAGE: The fresh 3× header/grid crop shows the complete day grid with no world bleed between or around the tiles: `/syncthing/AgentShared/2026-11-12/calendar-panel/calendar-header-grid-after-followup-3x.png`.
- VERIFIED, SOURCE: The inspected Calendar XML has no root backdrop. The panel adds no invented full-frame background.
- VERIFIED, BUILD: The editor selects `target/debug/libgame_engine_godot.so`; the first fresh client used an older debug extension after only the release build. The native dev helper rebuilt the debug extension, and the final capture used that build.

## Sources

- [Calendar panel system](../systems/calendar-panel.md)
- [Calendar panel spec](../../specs/calendar-panel.md)
- [Calendar model](../../../godot/ui-model/src/calendar_data.rs)
- [Calendar presentation](../../../godot/ui-model/src/ui/screens/calendar_frame_component.rs)
- Retail cached source: `Blizzard_Calendar.xml:187–213`, `Blizzard_CalendarTemplates.xml:94–118`, and `Mainline/Blizzard_Calendar.lua:160–1453`.

## See Also

- [[ui-system]] — frame strata and draw-layer projection
