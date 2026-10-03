# Minimap

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

The in-world Retail `MinimapCluster` at the top right of the screen: the round, north-up map of the local-CASC minimap tiles around the player, the player arrow, quest-giver blips, zone text, clock, calendar day and zoom. This spec covers the Godot client. The Bevy client's square minimap (`src/rendering/ui/minimap*.rs`) is reference only. How it works: [minimap system](../wiki/systems/minimap.md).

Retail sources are in `~/.cache/wow-ui-sim/blizzard-ui`: `Blizzard_Minimap/Mainline/Minimap.xml` and `Minimap.lua`, `GameTime.xml` and `GameTime.lua`, `Blizzard_TimeManager/Mainline/Blizzard_TimeManager.xml`, and `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`.

## What it must do

### Frame
- [x] `MinimapCluster` is 256×256, anchored TOPRIGHT of UIParent at (0, 0) (`EditModePresetLayouts.lua:371-384`, `Minimap.xml:3-9`). The buff area at TOPRIGHT −255 stays free.
- [x] Chrome comes from the Retail atlases on `interface/hud/uiminimap.blp`:
  - `ui-hud-minimap-frame`, 215×226, centred on the 198×198 `Minimap` (`Minimap.xml:172-235`);
  - the `BorderTop` 175×16 `UniqueCornersLayout` bar of `ui-hud-minimap-button` (`Minimap.xml:18-27`, `NineSliceLayouts.lua:399-410`);
  - the tracking button (`Minimap.xml:51-82`).
- [x] Zone text: `GetMinimapZoneText`, the AreaTable name of the area under the player (Northshire Valley). GameFontNormal 12 on the `ZoneTextButton` (`Minimap.xml:28-50`). Its colour follows `Minimap_Update` (`Minimap.lua:183-199`): sanctuary blue, friendly green, hostile red, otherwise gold, from the nearest controlling `FactionGroupMask` in the area's lineage. Clicking the text toggles the world map (`Minimap.lua:129-131`).
- [x] Clock: the 12-hour `TIMEMANAGER_TICKER_12HOUR` "%d:%02d" ticker, FRIZQT 10 white, on `TimeManagerClockButton` (`Blizzard_TimeManager.xml:178-192`, `GameTime.lua:36-42`).
- [x] Calendar: `GameTimeFrame` shows `ui-hud-calendar-<day>-up` for the day of the month (`GameTime.xml:3-58`, `GameTime.lua:245-253`).

### Map
- [x] The map is north-up with a rotating arrow, as the Modern preset's `RotateMinimap` 0 (`EditModePresetLayouts.lua:374`). Screen up is north and screen right is east.
- [x] Tiles are `world/minimaps/<map>/mapFF_SS.blp` from local CASC. The key pair is the ADT's (`azeroth_32_48.adt` pairs with `map32_48.blp`). Each tile covers 533⅓ yards. The composite is bilinear within each tile and clipped to the round map with a soft edge. A tile missing from the install draws dark grey (20, 20, 20).
- [x] Zoom has six levels. Outdoor view diameters are 466⅔, 400, 333⅓, 266⅔, 200 and 133⅓ yards (the addon-measured values; the engine does not expose them). The default is zoom 0.
- [x] Hovering the map shows `ZoomIn` 17×17 at (+88, −68) and `ZoomOut` 17×9 at (+72, −84) (`Minimap.xml:190-219`). Leaving the map and `ZoomHitArea` hides them (`Minimap.lua:267-282`). The mouse wheel over the map zooms (`Minimap.lua:160-166`), and the buttons clamp at the ends (`Minimap.lua:298-334`).
- [x] Num Pad + and Num Pad − zoom in and out: the Retail `MINIMAPZOOMIN` and `MINIMAPZOOMOUT` defaults (`Bindings_Standard.xml:1378-1383`). They are rebindable under Options › Keybindings › Interface. See [key bindings](key-bindings.md).
- [x] The player arrow (`Interface\Minimap\MinimapArrow`) sits at the centre and turns with the facing. It points where forward movement goes.
- [x] Every mirrored NPC with `NPCFlags::QUESTGIVER` is queried once with `QuestGiverStatusQuery`. `Available` shows the `QuestNormal` blip and `Reward` the `QuestTurnin` blip from `ObjectIconsAtlas`, placed at the NPC's offset from the player. Blips outside the circle are hidden, and the other statuses draw none.
- [x] The cluster exists only in world while `hud.show_minimap` is on.

### Forever skin (FlareUI square minimap)

Sources: local FlareUI 1.3 `data/reference/flareui/`; Forever Blizzard files relative to `~/.cache/wow-ui-sim/blizzard-ui/wowforever/AddOns/`. The user's `user-minimap-zoom-2026-10-03.png` and `user-flareui-hud-2026-10-03.png` references require a **bronze tooltip border rather than FlareUI's metal minimap frame**, and fixed yellow zone text. No FlareUI Media or new art.

- [x] Cluster 260×260, map 244×244, centred at (8,8), square mask: FlareUI `Modules/Minimap.lua:32-37,301-304`. Blips and arrow follow the map; zoom behavior and map-centre-relative offsets remain unchanged.
- [x] Header order: tracking, zone, clock, calendar. Header height 17, map-relative top 2, sides 6, gaps 2, clock width 40, calendar drop 1 (`Modules/Minimap.lua:42-47,200-208`). Tracking background `(14,10,17,17)`; its centred icon `(15,11,15,15)` uses height minus 2 (`:154-157`). Existing tracking art and behavior are reused.
- [x] Zone rect `(35,10,150,17)` fills the bar before the clock, left-aligned without wrapping (`Modules/Minimap.lua:162-167`). User-requested fixed GameFontNormal yellow `[1,0.82,0,1]`, not PvP tint: `Blizzard_Fonts_Shared/Shared/FontStyles.xml:50-52`, explicit gold RGB in `Blizzard_Fonts_Shared/Mainline/FontStyles.xml:6-7`. FRIZQT 12 and black shadow offset `(1,-1)` from `Blizzard_Fonts_Shared/Shared/Fonts.xml:275-282`.
- [x] White right-aligned clock `(185,10,40,17)` uses GameFontHighlight (`Modules/Minimap.lua:178-184`): white from `Blizzard_Fonts_Shared/Shared/FontStyles.xml:62-64`, FRIZQT 12 from `Shared/Fonts.xml:275-277`. Calendar `(227,10.5,19,18)` retains day art and behavior; its 19×18 size is `Blizzard_Minimap/Mainline/GameTime.xml:4`, placement derived from FlareUI `Modules/Minimap.lua:200-208`. Clock still uses the existing local-time format, not FlareUI's dynamic-width AM/PM clock.
- [x] Bronze border only, no opaque centre: existing `UI-Tooltip-Border` mechanism, 16-pixel cells (`Modules/UnitFrames.lua:35,41`; `Core.lua:144`). Fixed tint `[0.65,0.49,0.27,1]`, labelled #A67D45 (`Core.lua:8`), not tooltip reaction tint #CC9957 `[0.80,0.60,0.34]` (`Modules/Tooltips.lua:48`). Border frame `(0,0,260,260)` derives from cluster size; corner/edge rects derive from its 16-pixel cells. No Retail metal NineSlice, compass ring or Retail header bar. Cache includes Blizzard border FDID 137057 and calendar art.
- [x] Mail stays below the header, avoiding tracking overlap: `(14,33,20,15)` from map-relative side 6, title-band height 22, gap 3 (`Modules/Minimap.lua:41,44,50,226`); inherited art size 20×15 (`Retail Blizzard_Minimap/Mainline/Minimap.xml:99`).
- [x] Modern serialized trees remain byte-identical to the pre-skin fixture; a skin switch rebuilds the cluster canvas and composite.

No screenshot-measured header/border values remain. Inherited engine arrow/blip sizes are still assumptions listed below, not screenshot measurements. The old assumed map origin (8,16) and measured header placements were replaced by source-derived values. Round day/night sun button, objective tracker and zoom behavior are outside this change; the sun button remains pending a user decision.

### Not done
- [ ] Indoor (WMO) minimaps, the indoor diameter table, and `rotateMinimap` 1.
- [x] Creature vignettes: every replicated unit with `UnitVignette` whose `Vignette.csv` row lacks `DontShowOnMinimap` shows `VignetteKill`, or `VignetteKillElite` for elite/rare-elite/world-boss classifications, at its offset; the server adds and removes `UnitVignette` with the unit's 100 yd visibility and its death (`godot/rust/src/vignettes.rs`).
- [ ] Vignette blip size (16, as quest blips) and hover tooltip; infinite-AOI (map/zone-wide) vignettes are not sent.
- [ ] Tracking menu and filters (`Minimap.lua:21-52`): mailboxes, flight masters, innkeepers, trainers. Party members, the target, and quest POI arrows at the edge.
- [ ] Mail, crafting-order and instance-difficulty indicators; the expansion landing-page button; `AddonCompartment`.
- [ ] Clock and calendar tooltips and their clicks (TimeManager, Calendar). Realm time: the protocol carries none, so the clock shows local time. Military time.
- [ ] Minimap ping and the zone tooltip. Edit-mode size, the header-underneath option, and moving the cluster.
- [ ] Exact player arrow and blip sizes. These are engine-drawn, so the 32 and 16 unit sizes are assumptions.

## Tests asserting this spec

- `godot/core/tests/minimap_data.rs`: Northshire tile key and path; the composite's orientation, missing-tile colour and round mask; the texel under the player; blip offsets and edge; zoom clamps; clock text; zone text and PvP colours; race faction.
- `godot/ui-model/tests/forever_minimap.rs`: the Modern trees equal the pre-skin fixture; the Forever cluster, map, header texts, border, blips, arrow, zoom buttons and mail rects; a live skin switch. `godot/core/tests/minimap_data.rs` covers the square mask's corners and corner blips.
- `godot/rust/src/vignettes_tests.rs`: Doomwalker (Vignette 6520, elite) draws one `VignetteKillElite` blip 50 yd north; a `DontShowOnMinimap` vignette, one past the edge and one without `UnitVignette` draw none. `godot/tests/vignettes_live.gd` (live, private server, Doomwalker forced active): screenshots in `data/diagnostics/vignettes-2026-10-02/`.
- `godot/tests/world_minimap_quest.gd` (live, private server): the drawn tile is the player's `map32_48`, the composite centre is that tile's texel, and the zone text is "Northshire Valley". The arrow's rotation matches the facing, and the arrow points along W movement. A McBride blip shows. Hover shows the zoom buttons, the wheel zooms in and ZoomOut zooms out. Screenshots are in `data/diagnostics/minimapquest-2026-09-29/`.

## Implementation inventory

- `godot/core/src/minimap_data.rs`: tile keys and paths, composite, blip offset, zoom, clock, `AreaCatalog` zone text and PvP, `ChrRaces` faction groups.
- `godot/ui-model/src/minimap.rs`: the `MinimapCluster` rsx screen, atlas crops, and postsetup for the arrow rotation and the dynamic composite.
- `godot/rust/src/minimap.rs`: the host. It handles tile loading from local CASC, recompositing, quest-giver queries, hover and wheel, actions, and the `minimap_state()` fixture view.
