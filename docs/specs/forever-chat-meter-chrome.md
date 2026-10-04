# Forever chat and meter chrome

Forever-only reference chrome in `godot/ui-model/src/ui/screens/{chat_frame,damage_meter}_component.rs`, sharing `ui/flare_panel.rs`. Modern trees and both HUD frame positions/sizes remain unchanged. References: `data/diagnostics/forever-reference/user-{chat-box,damage-meter,flareui-hud}-2026-10-03.png`. FlareUI 1.3 source below means `data/reference/flareui`; only MIT layout numbers/colours, never its Media art.

## What it must do

- [x] Both panels retain dark translucent Blizzard dialog backgrounds, bronze tooltip borders, a header band and separator.
- [x] Meter header shows DPS and a display-only Threat tab, chart and gear icons; no Retail timer, arrow, minimize or dropdown background.
- [x] Meter rows show square class icons (snapshot has no specialization), rounded bronze outlines, class-coloured horizontal-gradient fills over dark tracks and white shadowed rank/name and damage (DPS) labels.
- [x] Chat keeps existing tabs/actions as plain text, active bright bronze and inactive dim bronze; four bronze header icons. Message rendering, input and scroll behavior remain unchanged.
- [x] Modern serialized chat/meter trees remain byte-identical to the pre-change baseline, for all chat tabs and an open meter session menu.
- [ ] Forever chat and meter header buttons share a 22-square hit box centred vertically in each 24-high header, entirely above the separator; their glyphs share a 13.2-square draw box and common centre. Right-side button centres retain uniform spacing.
- [ ] Header glyph crops exclude unequal atlas-cell padding and the gear's dropdown frame; the meter uses the same gear crop as chat.

## Number and colour provenance

All rectangles use top-left screen coordinates. Derived coordinates below are arithmetic, not extra reference claims.

| Contract | Provenance |
| --- | --- |
| Panel opacity 0.6, border RGB (0.65,0.49,0.27), alpha 1; edge 16, inset 3 | `Core.lua:8,142,144-145,246`; `Modules/DamageMeter.lua:221-223` |
| Header height 24; meter skin padding 2; chat padding 10 | `Modules/DamageMeter.lua:46`; `Core.lua:143,146,249` |
| Active title RGB (0.80,0.60,0.34), icon RGB (0.61,0.48,0.29), inactive RGB (0.56,0.51,0.46), alpha 1 | `Modules/DamageMeter.lua:60-61,705`; `Core.lua:148,162,194` |
| Header/row font 12; black shadow alpha 1 and offset (1,-1); row text white | `Modules/DamageMeter.lua:548,103-104`; `Core.lua:251`; white retained from Retail `NumberFontNormal` (existing damage-meter spec) |
| Header buttons 22 square; visible glyphs 13.2 square, inset 4.4 | `Modules/Chat.lua:531`; `Core.lua:161,166,171,176` scales all four chat buttons to 0.6 (22*0.6=13.2). Meter hit box/draw box are the same-size adaptation, not the source's per-icon Media sizing (`Modules/DamageMeter.lua:50-57`). Inset derived (22-13.2)/2=4.4 |
| Meter title x=15, y=7 (90x12); Threat x=64 (60x12); gear x=369.5, chart x=348.5, y=-1 | Title x derived from padding -2 + `TITLE_LEFT` 17 (`Modules/DamageMeter.lua:49`); y derived from skin top -2 + centre 15 minus font half 6 (`:48,548`); icon centres derived from `ICON_RIGHT` 15, `ICON_SIZE` 13, `ICON_SPACING` 21 (`:50,58-59`); label boxes/Threat x measured from damage-meter screenshot, adapted to retained 400-wide frame |
| Meter rows x=4, y=32 + index*20, width 392, height 16; square icon 16; gap 4; bar x=20, width 372 | Measured from damage-meter screenshot, adapted to retained 400x140 frame; height 16 and spacing 4 retained from existing Retail layout (damage-meter spec); y derived from skin top -2 + `CONTENT_TOP` 34 (`Modules/DamageMeter.lua:47`) |
| Track RGBA (0.1,0.1,0.1,0.9); fill inset 1; border corner extent 8; gradient black alpha 0.6, transparent left to dark right | **Measured** from damage-meter screenshot, adapted to retained row height. Existing Blizzard cooldown-manager fill art (FDID 6704514); existing Chattynator Fade.png supplies gradient, not FlareUI art |
| Row name x=5,width=214; value x=224,width=140; text height 16 | Name x=5/value width 140 retained from Retail component; right inset 8 retained, width 372 gives value x=224; measured name gap 5 gives width 214 |
| Chat header buttons x=352,387,422,457, y=-6, 22 square | Horizontal spacing/right inset **measured** from chat-box screenshot, adapted to existing panel bounds; 35 spacing, first x derived from panel right 505 minus 48 minus 3*35. Corrected vertical centre derived from header top -7 + (24-22)/2=-6, centre y=5. Meter y=-2+(24-22)/2=-1, centre y=10. Button bottoms 16/21 leave 1 above separators 17/22; glyph bottoms 11.6/16.6 |
| Chat text tabs x=34, top=-7, height=24; label y=6, text width +14, gaps 4 | x derived from panel left 24 + tab offset 10 (`Modules/Chat.lua:946`); height `:1149`; text padding 7 per side **measured**, gap `:1248`; centered 12-high font in 24-high header |
| Separator 1 high, bronze; inset 3; chat x=27,y=17,width=475; meter x=1,y=22,width=398 | **Measured** thin solid line from screenshots, not FlareUI's default 8-high white-tinted divider. Solid height 1 available in `Modules/Chat.lua:1609`; inset 3 `Modules/DamageMeter.lua:249-250`; coordinates derived from retained panel bounds +24 header |
| Panel bounds chat (24,-7,481,259), meter (-2,-2,404,144); chat message rect (34,27,461,215) | Existing component constants and FlareUI padding/header arithmetic; retained, not newly measured |

Chart columns inside the centred 13.2-square glyph box: x=0,5.5,11; y=7.7,4.4,0; width=2.2; heights=5.5,8.8,13.2. Proportions **measured** from the damage-meter reference and adapted to the shared glyph size, drawn geometrically rather than copying FlareUI Media. Zero-based column index and unit alpha are structural values, not screenshot claims.

Header art uses existing Blizzard sheets directly, with explicit normalized `[left,right,top,bottom]` crops below. These are **measured from decoded Blizzard pixels**, not FlareUI art or screenshot dimensions. Social/volume use the nonzero-alpha glyph bounds; gear uses the gold cog only (removes the surrounding dropdown frame); channel uses the central coin without its diffuse outer shadow. Each crop maps into the same 13.2-square draw box, rather than stretching the differently padded cells.

| Glyph | FDID / sheet size | Original cell pixels (left,right,top,bottom) | Corrected glyph pixels | Normalized tex-coords |
| --- | --- | --- | --- | --- |
| Channel | 1121272 / 1024x1024 | (423,455,764,796), 32x32 | (427,451,768,792) | (427/1024,451/1024,768/1024,792/1024) |
| Menu / meter gear | 7518377 / 128x64 | (59,86,30,57), 27x27; cog is (8,20,5,17) inside cell | (67,79,35,47) | (67/128,79/128,35/64,47/64) |
| Social | 8200846 / 1024x512 | (265,329,85,167), 64x82; visible bounds (11,53,13,72) | (276,318,98,157) | (276/1024,318/1024,98/512,157/512) |
| Volume | 5390329 / 512x256 | (386,403,29,46), 17x17; visible bounds (2,13,2,16) | (388,399,31,45) | (388/512,399/512,31/256,45/256) |

Captured tree (`shot2-2026-10-03/forever-target-tree2.log:790-813,933-953`) had identical 22-square chat rectangles at y=759, bottom781, but separator777; meter buttons y=2, bottom24, separator22. Separator level1 and glyph level2: layering did not cause the overlap. Unequal apparent sizes/baselines came from padded cells and the off-centre cog; geometry separately put both rows across their separators.

Class colours remain `RAID_CLASS_COLORS` (`damage_meter_data::class_color`); no changes to amounts, ordering or formatting. Header substitutions are explicit cropped Blizzard art choices from chatballon, settings-shadowless, UI-HUD-MicroMenu-GuildCommunities-Up (Forever set-1 sheet) and sound-on, not missing-texture fallbacks; classicon-* rows still resolve by atlas. Chart is drawn as three bronze columns because FlareUI's chart artwork is not reusable.

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

`godot/ui-model/tests/forever_chat_meter.rs`; Modern serialized fixture in `tests/fixtures/modern_chat_meter.rs` captured before production edits at `5c1e2db5` (88,806 bytes). Targeted Depot run at `423dbf7b`: 5 passed, 0 failed, exit 0. Behavioral RED at `5c1e2db5`: missing chat/meter headers and Retail row rectangle instead of reference rectangle. No live run.

## Known gaps (current cycle)

- [ ] Threat tab only: the client meter snapshot carries damage sessions, not threat; no threat numbers invented.
- [ ] Chat header channel/menu/social/volume icons are presentation only where the client has no corresponding action; existing copy action remains available on the menu icon, scrolling control preserved.
- [ ] CPU tree proof only; no live/pixel rendering proof requested.

## Out of scope

HUD positions/sizes, message colours/behavior, minimap, unit frames, tooltips, new meter modes/data sources and FlareUI Media reuse.
