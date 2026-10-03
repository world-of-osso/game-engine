# Forever chat and meter chrome

Forever-only reference chrome in `godot/ui-model/src/ui/screens/{chat_frame,damage_meter}_component.rs`, sharing `ui/flare_panel.rs`. Modern trees and both HUD frame positions/sizes remain unchanged. References: `data/diagnostics/forever-reference/user-{chat-box,damage-meter,flareui-hud}-2026-10-03.png`. FlareUI 1.3 source below means `data/reference/flareui`; only MIT layout numbers/colours, never its Media art.

## What it must do

- [ ] Both panels retain dark translucent Blizzard dialog backgrounds, bronze tooltip borders, a header band and separator.
- [ ] Meter header shows DPS and a display-only Threat tab, chart and gear icons; no Retail timer, arrow, minimize or dropdown background.
- [ ] Meter rows show square class icons (snapshot has no specialization), rounded bronze outlines, class-coloured horizontal-gradient fills over dark tracks and white shadowed rank/name and damage (DPS) labels.
- [ ] Chat keeps existing tabs/actions as plain text, active bright bronze and inactive dim bronze; four bronze header icons. Message rendering, input and scroll behavior remain unchanged.
- [ ] Modern serialized chat/meter trees remain byte-identical to the pre-change baseline, for all chat tabs and an open meter session menu.

## Number and colour provenance

All rectangles use top-left screen coordinates. Derived coordinates below are arithmetic, not extra reference claims.

| Contract | Provenance |
| --- | --- |
| Panel opacity 0.6, border RGB (0.65,0.49,0.27), alpha 1; edge 16, inset 3 | `Core.lua:8,142,144-145,246`; `Modules/DamageMeter.lua:221-223` |
| Header height 24; meter skin padding 2; chat padding 10 | `Modules/DamageMeter.lua:46`; `Core.lua:143,146,249` |
| Active title RGB (0.80,0.60,0.34), icon RGB (0.61,0.48,0.29), inactive RGB (0.56,0.51,0.46), alpha 1 | `Modules/DamageMeter.lua:60-61,705`; `Core.lua:148,162,194` |
| Header/row font 12; black shadow alpha 1 and offset (1,-1); row text white | `Modules/DamageMeter.lua:548,103-104`; `Core.lua:251`; white retained from Retail `NumberFontNormal` (existing damage-meter spec) |
| Header buttons 22 square | `Modules/Chat.lua:531`; meter hit box measured adaptation around visible icons |
| Meter title x=15, y=7 (90x12); Threat x=64 (60x12); gear x=369.5, chart x=348.5, y=2 | Title x derived from padding -2 + `TITLE_LEFT` 17 (`Modules/DamageMeter.lua:49`); y derived from skin top -2 + centre 15 minus font half 6 (`:48,548`); icon centres derived from `ICON_RIGHT` 15, `ICON_SIZE` 13, `ICON_SPACING` 21 (`:50,58-59`); label boxes/Threat x measured from damage-meter screenshot, adapted to retained 400-wide frame |
| Meter rows x=4, y=32 + index*20, width 392, height 16; square icon 16; gap 4; bar x=20, width 372 | Measured from damage-meter screenshot, adapted to retained 400x140 frame; height 16 and spacing 4 retained from existing Retail layout (damage-meter spec); y derived from skin top -2 + `CONTENT_TOP` 34 (`Modules/DamageMeter.lua:47`) |
| Track RGBA (0.1,0.1,0.1,0.9); fill inset 1; border corner extent 8; gradient black alpha 0.6, transparent left to dark right | **Measured** from damage-meter screenshot, adapted to retained row height. Existing Blizzard cooldown-manager fill art (FDID 6704514); existing Chattynator Fade.png supplies gradient, not FlareUI art |
| Row name x=5,width=214; value x=224,width=140; text height 16 | Name x=5/value width 140 retained from Retail component; right inset 8 retained, width 372 gives value x=224; measured name gap 5 gives width 214 |
| Chat header icons x=352,387,422,457, y=-1, 22 square | **Measured** from chat-box screenshot, adapted to existing panel bounds; 35 spacing, first x derived from panel right 505 minus 48 minus 3*35; y derived from panel top -7 + 6 |
| Chat text tabs x=34, top=-7, height=24; label y=6, text width +14, gaps 4 | x derived from panel left 24 + tab offset 10 (`Modules/Chat.lua:946`); height `:1149`; text padding 7 per side **measured**, gap `:1248`; centered 12-high font in 24-high header |
| Separator 1 high, bronze; inset 3; chat x=27,y=17,width=475; meter x=1,y=22,width=398 | **Measured** thin solid line from screenshots, not FlareUI's default 8-high white-tinted divider. Solid height 1 available in `Modules/Chat.lua:1609`; inset 3 `Modules/DamageMeter.lua:249-250`; coordinates derived from retained panel bounds +24 header |
| Panel bounds chat (24,-7,481,259), meter (-2,-2,404,144); chat message rect (34,27,461,215) | Existing component constants and FlareUI padding/header arithmetic; retained, not newly measured |

Chart column boxes (x=4,9,14; y=12,8,4; width=3; heights=6,10,14 inside a 22-square hit box) are **measured** icon proportions adapted from the damage-meter reference, drawn geometrically rather than copying FlareUI Media. Gear art fills the 22-square box; zero-based column index and unit alpha are structural values, not screenshot claims.

Class colours remain `RAID_CLASS_COLORS` (`damage_meter_data::class_color`); no changes to amounts, ordering or formatting. Atlas substitutions are explicit existing Blizzard art choices, not missing-texture fallbacks: chatballon, settings-shadowless, UI-HUD-MicroMenu-GuildCommunities-Up (Forever set-1 sheet), sound-on and classicon-*; chart is drawn as three bronze columns because FlareUI's chart artwork is not reusable.

## How it works

- [Skin palettes](shared-skin-palettes.md)
- [Chat behavior](chat-frame.md)
- [Meter data and behavior](damage-meter.md)

## Implementation inventory

- `godot/ui-model/src/ui/flare_panel.rs`: shared panel/header/text/icon chrome.
- `godot/ui-model/src/ui/screens/damage_meter_component.rs`: Forever header and rows; Modern unchanged.
- `godot/ui-model/src/damage_meter_data.rs`: class identity retained for icon selection.
- `godot/ui-model/src/ui/screens/chat_frame_component.rs`: Forever text tabs and header icons.
- `godot/depot-test-assets.txt`: existing art included in CPU fixture snapshot.

## Tests asserting this spec

`godot/ui-model/tests/forever_chat_meter.rs`; Modern serialized fixture in `tests/fixtures/modern_chat_meter.rs` captured before production edits.

## Known gaps (current cycle)

- [ ] Threat tab only: the client meter snapshot carries damage sessions, not threat; no threat numbers invented.
- [ ] Chat header channel/menu/social/volume icons are presentation only where the client has no corresponding action; existing copy action remains available on the menu icon, scrolling control preserved.
- [ ] CPU tree proof only; no live/pixel rendering proof requested.

## Out of scope

HUD positions/sizes, message colours/behavior, minimap, unit frames, tooltips, new meter modes/data sources and FlareUI Media reuse.
