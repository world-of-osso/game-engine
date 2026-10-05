# Forever chat and meter chrome

Forever-only reference chrome in `godot/ui-model/src/ui/screens/{chat_frame,damage_meter}_component.rs`, sharing `ui/flare_panel.rs`. Modern trees and HUD frame positions/sizes remain unchanged. Forever panel positions/sizes follow [HUD presets](hud-edit-mode.md#godot-client-presets). References: `data/diagnostics/forever-reference/user-{chat-box,damage-meter,flareui-hud}-2026-10-03.png`. FlareUI 1.3 source below means `data/reference/flareui`; only MIT layout numbers/colours, never its Media art.

## What it must do

- [x] Both panels retain dark translucent Blizzard dialog backgrounds, bronze tooltip borders, a header band and separator.
- [x] Meter header shows DPS and HPS tabs (Damage Done and Healing Done; [types](damage-meter.md)), chart and gear icons; no Retail timer, arrow, minimize or dropdown background.
- [x] Meter rows show square class icons (snapshot has no specialization), rounded bronze outlines, class-coloured horizontal-gradient fills over dark tracks and white shadowed rank/name and damage (DPS) labels.
- [x] Chat keeps existing tabs/actions as plain text, active bright bronze and inactive dim bronze; four bronze header icons, left to right channel (page of text lines), menu (speech bubble), social (figure), volume (speaker). Message rendering, input and scroll behavior remain unchanged.
- [x] Modern serialized chat/meter trees remain byte-identical to the pre-change baseline, for all chat tabs and an open meter session menu.
- [x] Forever chat and meter header buttons share a 22-square hit box centred vertically in each 24-high header, entirely above the separator; their glyphs share a 13.2-square draw box and common centre. Right-side button centres retain uniform spacing.
- [x] Header glyph crops exclude atlas-cell padding and button or dropdown frames.

## Number and colour provenance

Rectangles use top-left parent-local coordinates. Chat skin (24,-7) is relative to its canvas: at 1366×768, canvas (1,426) gives screen skin origin (25,419). Meter chrome is relative to its 450×214 root. Derived coordinates below are arithmetic, not extra reference claims.

| Contract | Provenance |
| --- | --- |
| Panel opacity 0.6, border RGB (0.65,0.49,0.27), alpha 1; edge 16, inset 3 | `Core.lua:8,142,144-145,246`; `Modules/DamageMeter.lua:221-223` |
| Header height 24; meter skin padding 2; chat padding 10 | `Modules/DamageMeter.lua:46`; `Core.lua:143,146,249` |
| Active title RGB (0.80,0.60,0.34), icon RGB (0.61,0.48,0.29), inactive RGB (0.56,0.51,0.46), alpha 1 | `Modules/DamageMeter.lua:60-61,705`; `Core.lua:148,162,194` |
| Header font 12; row font 16; black shadow alpha 1 and offset (1,-1); row text white | Header `Modules/DamageMeter.lua:548,103-104`. FlareUI's bar font is 12 (`Core.lua:251`) times Retail's Edit Mode text scale (`DamageMeterEntry.lua:401-403`, `DAMAGE_METER_TEXT_SIZE_TO_SCALE_MULTIPLIER` `DamageMeterConstants.lua:16`); row 16 **measured**: reference digits 18 px at 1.824 px/unit = 9.9 units vs 7.15 units for 12 pt Friz in our capture (x1.38, about text size 140%, rounded down to 16). White retained from Retail `NumberFontNormal` |
| Header buttons 22 square; visible glyphs 13.2 square, inset 4.4 | `Modules/Chat.lua:531`; `Core.lua:161,166,171,176` scales all four chat buttons to 0.6 (22*0.6=13.2). Meter hit box/draw box are the same-size adaptation, not the source's per-icon Media sizing (`Modules/DamageMeter.lua:50-57`). Inset derived (22-13.2)/2=4.4 |
| Meter title x=15, y=7 (90x12); HPS x=64 (60x12); tab hit boxes 49x22 from 4 before each label; third type label x=113 (160x12); gear x=419.5, chart x=398.5, y=-1 | Title x derived from padding -2 + `TITLE_LEFT` 17 (`Modules/DamageMeter.lua:49`); y derived from skin top -2 + centre 15 minus font half 6 (`:48,548`); icon centres derived from `ICON_RIGHT` 15, `ICON_SIZE` 13, `ICON_SPACING` 21 (`:50,58-59`); button x derives from root width 450 minus 30.5 (gear) or 51.5 (chart); label boxes/second tab x measured from damage-meter screenshot, adapted to the 450-wide root; hit boxes and third label derived from the 49 between the tab labels |
| Meter rows x=4, y=32 + index*34, width 442, height 30; gap 4; square icon 24 centred (y=3); bar x=28, width 414, height 30. Rows keep this size at any window size; the window shows as many as fit between y=32 and 6 above its bottom (`floor((h - 6 - 32 + 4) / 34)`): 5 at the preset 450×214, 8 at height 320, 2 at 120 | FlareUI sets no row size of its own: it draws Retail's `DamageMeterEntryTemplate` at the window's `GetBarHeight()`/`GetBarSpacing()` (`Modules/DamageMeter.lua:815-822,852-853`) and fits `floor((frameHeight + spacing) / (height + spacing))` rows (`:854`), so rows do not scale with the window. Retail: icon frame 24×24 anchored LEFT (`DamageMeterEntry.xml:6-9`), bar 5 right of it in Bordered style (`DamageMeterEntry.lua:204-211`), defaults bar height 25 / spacing 4 (`DamageMeterConstants.lua:5,11`), Edit Mode bar height 15-40 (`EditModeSettingDisplayInfo.lua:1306-1310`). Row height 30, gap 4, icon 24, icon-to-bar 4 **measured** from the damage-meter screenshot (window outer 821 px for 450 units = 1.824 px/unit; bar outline 55 px = 30.2, pitch 61.25 px = 33.6, icon 44 px = 24.1, icon-to-bar 7 px = 3.8; five rows in the window). y derived from skin top -2 + `CONTENT_TOP` 34 (`Modules/DamageMeter.lua:47`) |
| Track RGBA (0.1,0.1,0.1,0.9); fill inset 1 (fill height 28); border corner extent 8; gradient black alpha 0.6, transparent left to dark right | **Measured** from damage-meter screenshot (fill 49 px = 27 units). Existing Blizzard cooldown-manager fill art (FDID 6704514); existing Chattynator Fade.png supplies gradient, not FlareUI art |
| Row name x=5,width=256; value x=266,width=140; text box height 30, text drawn after the fill | Name x=5/value width 140 retained from Retail component; right inset 8 retained, width 414 gives value x=266; measured name gap 5 gives width 256 |
| Chat header buttons x=378.4,399.4,420.4,441.4, y=-6, 22 square (glyphs 21 apart, last glyph ending 15 inside the skin's right edge) | FlareUI scales each 22-unit button to 0.6 (`Core.lua:161,166,171,176`), so `SetPoint` offsets `HEADER_FIRST_X` -25 and `HEADER_STEP` -35 (`Modules/Chat.lua:472,481-487`) are in the button's scaled space: 15 and 21 units on the skin (the earlier 35 pitch ignored the scale; the reference shows ~20). Vertical centre derived from header top -7 + (24-22)/2=-6, centre y=5. Meter y=-2+(24-22)/2=-1, centre y=10. Button bottoms 16/21 leave 1 above separators 17/22; glyph bottoms 11.6/16.6 |
| Chat text tabs x=34, top=-7, height=24; label y=6, text width +14, gaps 4 | x derived from panel left 24 + tab offset 10 (`Modules/Chat.lua:946`); height `:1149`; text padding 7 per side **measured**, gap `:1248`; centered 12-high font in 24-high header |
| Separator 1 high, bronze; inset 3; chat x=27,y=17,width=444; meter x=1,y=22,width=448 | **Measured** thin solid line from screenshots, not FlareUI's default 8-high white-tinted divider. Solid height 1 available in `Modules/Chat.lua:1609`; inset 3 `Modules/DamageMeter.lua:249-250`; coordinates derived from resized panel bounds +24 header |
| Panel bounds chat (24,-7,450,214), meter (-2,-2,454,218); chat message rect (34,27,430,170) | Forever messages 430×170, chat padding 10/header 24; meter root matches chat skin, meter skin adds 2 per edge (`Modules/Chat.lua:1515-1520`, `Modules/DamageMeter.lua:202-212,1137-1159`); see [HUD presets](hud-edit-mode.md#godot-client-presets) |

Chart columns inside the centred 13.2-square glyph box: x=0,5.5,11; y=7.7,4.4,0; width=2.2; heights=5.5,8.8,13.2. Proportions **measured** from the damage-meter reference and adapted to the shared glyph size, drawn geometrically rather than copying FlareUI Media. Zero-based column index and unit alpha are structural values, not screenshot claims.

Header order is FlareUI's `HEADER_ORDER` volume, social, menu, channel packed from the right edge (`Modules/Chat.lua:471-487`), so left to right channel, menu, social, volume. FlareUI draws its own Media art on Blizzard's buttons (`Modules/Chat.lua:54-61,609-611,1396-1426`); that art is not reusable, so each glyph is cut from the Blizzard art of the button it restyles, with explicit normalized `[left,right,top,bottom]` crops **measured from decoded Blizzard pixels**. Each crop maps into the same 13.2-square draw box. The host copies the four sheets from local CASC before the chat frame first draws (`FOREVER_CHAT_HEADER_FDIDS`).

The four chat glyphs are one look: flat bronze shapes on a clear ground. FlareUI tints white icons with `SetVertexColor` 0.61/0.48/0.29 (`Modules/Chat.lua:547`, `Core.lua:162-177`), as Retail tints mask textures (`FriendsFrame.xml:1140-1146`). The Blizzard crops are coloured and shaded (the bubble and speaker over dark drop shadows of alpha 0x71-0xdd, the figure on an opaque dark face), so the host draws each crop as a white mask (`flare_glyph_mask`): alpha is source alpha times coverage, 0 at HSV value <= 0.30 (shadows and backdrop measured <= 0.31) rising to 1 at >= 0.55 (gold faces measured >= 0.55). The vertex colour alone then colours each glyph.

| Glyph | FlareUI button | Blizzard art | FDID / size | Glyph pixels (left,right,top,bottom) |
| --- | --- | --- | --- | --- |
| Channel | `ChatFrameChannelButton` (`Chat.lua:59,610`) | Retail's friend-note glyph `Interface/FriendsFrame/UI-FriendsFrame-Note`, drawn tinted (`<Color>`) as a mask (`Blizzard_FriendsFrame/Mainline/FriendsFrame.xml:1140-1146`) | 131129 / 8x8 | (0,6,0,8) |
| Menu | `ChatFrameMenuButton` (`Chat.lua:611`) | NormalTexture `Interface\ChatFrame\UI-ChatIcon-Chat-Up` (`FloatingChatFrame.xml:670`): the speech bubble inside the button frame | 130949 / 32x32 | (8,22,9,23) |
| Social | `QuickJoinToastButton` (`Chat.lua:609`) | `FriendsButton` atlas `quickjoin-button-friendslist-up` (`QuickJoinToast.xml:42`), cell (338,370,1,33): the figure inside the button frame. Its crop is opaque, so its dark face drops out by brightness in the mask; `groupfinder-icon-friend` was tried and is a blue Battle.net glyph, not a fit | 1537274 / 512x64 | (346,360,7,21) |
| Volume | FlareUI's own button (`Chat.lua:1396-1426`) | `common-dropdown-icon-sound-on`, cell (386,403,29,46), without padding | 5390329 / 512x256 | (388,399,31,45) |
| Meter gear | settings (`DamageMeter.lua:50-57`) | `common-dropdown-a-button-settings-shadowless`, cell (59,86,30,57): the cog without its dropdown frame | 7518377 / 128x64 | (67,79,35,47) |

Channel (`ChatFrameChannelButton`, `Chat.lua:59,610`) is a page of text lines in FlareUI. Blizzard's icon for that button is the voice chat speaker `chatframe-button-icon-voicechat` (`ChannelFrameButtonMixin.lua:24`), which would repeat the volume glyph, so it draws Retail's friend-note page instead, tinted like the other three glyphs.

Captured tree (`shot2-2026-10-03/forever-target-tree2.log:790-813,933-953`) had identical 22-square chat rectangles at y=759, bottom781, but separator777; meter buttons y=2, bottom24, separator22. Separator level1 and glyph level2: layering did not cause the overlap. Unequal apparent sizes/baselines came from padded cells and the off-centre cog; geometry separately put both rows across their separators.

Class colours remain `RAID_CLASS_COLORS` (`damage_meter_data::class_color`); no changes to amounts, ordering or formatting. Header glyphs are explicit cropped Blizzard art, not missing-texture fallbacks; classicon-* rows still resolve by atlas. The chart is drawn from solid frames because FlareUI's artwork is not reusable.

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

`godot/ui-model/tests/forever_chat_meter.rs`; Modern serialized fixture in `tests/fixtures/modern_chat_meter.rs` captured before production edits at `5c1e2db5` (88,806 bytes). Targeted Depot run at `423dbf7b`: 5 passed, 0 failed, exit 0. Behavioral RED at `5c1e2db5`: missing chat/meter headers and Retail row rectangle instead of reference rectangle.

Icon alignment/crop regression at `20c1a505`: 3 passed, 4 failed (old chat/meter tops and untrimmed art). GREEN at `46d8af19`: 7 passed, 0 failed, exit 0, including the unchanged Modern byte fixture, exact button/glyph rectangles, common vertical centres, separator clearance and uniform spacing. Which sheet and crop a glyph uses is not asserted (2026-10-04); live capture `data/diagnostics/hudfixes-2026-10-04/` shows them. Same targeted Depot command: `python3 scripts/depot-build.py --root <worktree> --test -p game-engine-ui-model --test forever_chat_meter`. No live run.

## Known gaps (current cycle)

- [ ] Chat header channel/menu/social/volume icons are presentation only where the client has no corresponding action; existing copy action remains available on the menu icon, scrolling control preserved.
- [ ] CPU tree proof only; no live/pixel rendering proof requested.

## Out of scope

HUD preset changes beyond the linked 450×214 panel resize, message colours/behavior, minimap, unit frames, tooltips, new meter modes/data sources and FlareUI Media reuse.
