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
- [x] Both skins show online party/raid members except the local player from existing `GroupMemberStates` positions, even outside ordinary replica interest. Missing positions and offline/left members draw nothing; no protocol change.
- [x] Party dots use `playerpartyblip`, raid dots `playerraidblip`, tinted by the existing `RAID_CLASS_COLORS` palette. Out-of-range members use `rotating-minimapgrouparrow` in its authored colours, clamped along the member direction inside the round Modern or square Forever rim, with room for half the 16-unit icon; the Forever north rim clears the opaque 22-unit header. In-range dots remain north-up; edge arrows point toward the member (registry counter-clockwise rotation, negated by native Godot projection).
- [x] The selected replicated target shows `target-tracker` at its north-up position, above other blips; clearing/despawning or moving outside the map mask hides it.
- [x] Both skins' tracking button opens a multi-select menu for supported replicated NPC services: Flight Masters, Innkeepers, Repair, Class Trainers, Profession Trainers, Bankers, Food & Drink, Reagents and Auctioneers. All start unchecked; each checkbox toggles its matching `NpcFlags` filter. A selected, in-range service draws the matching Retail ObjectIconsAtlas art at its replicated position; one icon per NPC with priority in menu order. Unselected, unclassified and out-of-range units draw none. Clicking outside closes the menu. Selection is client-session-local and limited to replica interest, not an invented full-map service feed.

Blip art: local `data/db2/12.1.0.69933/UiTextureAtlasMember.csv`, atlas 647, members 4741/4742/4776/4791; `Interface/Minimap/ObjectIconsAtlas.blp` FDID 1121272 (1024²). Crops are respectively `(525,557,628,660)`, `(525,557,662,694)`, `(627,659,764,796)`, `(695,727,594,626)` in left/right/top/bottom order. Cached Retail `Blizzard_Minimap/Mainline/Minimap.lua:251-253` dispatches `PLAYER_TARGET_CHANGED` to native `UpdateBlips`; FrameXML does not expose the native placement/colour algorithm. Icon sizing and inset remain client choices, not Lua-derived pixel-parity claims.

### Forever skin (FlareUI square minimap)

Sources: local FlareUI 1.3 `data/reference/flareui/`; Forever Blizzard files relative to `~/.cache/wow-ui-sim/blizzard-ui/wowforever/AddOns/`. The user's `user-minimap-zoom-2026-10-03.png` and `user-flareui-hud-2026-10-03.png` references require square bevelled panel chrome, a 22-unit title band and fixed yellow zone text (2026-10-06 minimap gaps correction). Project-drawn chrome is allowed; never copy FlareUI Media.

- [x] Cluster 260×260, map 244×244, centred at (8,8), square mask: FlareUI `Modules/Minimap.lua:32-37,301-304`. Blips and arrow follow the map; zoom behavior and map-centre-relative offsets remain unchanged.
- [x] Header order: tracking, zone, clock, calendar. Header height 17, map-relative top 2, sides 6, gaps 2, clock width 40, calendar drop 1 (`Modules/Minimap.lua:42-47,200-208`). Tracking background `(14,10,17,17)`; its centred icon `(15,11,15,15)` uses height minus 2 (`:154-157`). Existing tracking art and behavior are reused.
- [x] Zone rect `(35,10,150,17)` fills the bar before the clock, left-aligned without wrapping (`Modules/Minimap.lua:162-167`). User-requested fixed GameFontNormal yellow `[1,0.82,0,1]`, not PvP tint: `Blizzard_Fonts_Shared/Shared/FontStyles.xml:50-52`, explicit gold RGB in `Blizzard_Fonts_Shared/Mainline/FontStyles.xml:6-7`. FRIZQT 12 and black shadow offset `(1,-1)` from `Blizzard_Fonts_Shared/Shared/Fonts.xml:275-282`.
- [x] White right-aligned clock `(185,10,40,17)` uses GameFontHighlight (`Modules/Minimap.lua:178-184`): white from `Blizzard_Fonts_Shared/Shared/FontStyles.xml:62-64`, FRIZQT 12 from `Shared/Fonts.xml:275-277`. Calendar `(227,10.5,19,18)` retains day art and behavior; its 19×18 size is `Blizzard_Minimap/Mainline/GameTime.xml:4`, placement derived from FlareUI `Modules/Minimap.lua:200-208`. Clock still uses the existing local-time format, not FlareUI's dynamic-width AM/PM clock.
- [x] Square bevelled panel chrome: project-drawn eight-unit rim around the unchanged 260×260 cluster. FlareUI selects `ButtonFrameTemplateNoPortrait` and shifts its left corners −6 (`Modules/Minimap.lua:35-36,99-126`); flat drawing reproduces its square visible map-edge silhouette, not its texture layout. Warm light/shadow colours come from the user's reference; the Lua supplies only opaque black under the map (`:123-126`), retained while map data is absent. An opaque black title band `(8,8,244,22)` covers the header's map-relative top 22 units (`:41`). Composite display `(8,30,244,222)` uses UV top `22/244`, preserving the original map scale and centre without drawing map pixels behind the header.
- [x] Mail stays below the header, avoiding tracking overlap: `(14,33,20,15)` from map-relative side 6, title-band height 22, gap 3 (`Modules/Minimap.lua:41,44,50,226`); inherited art size 20×15 (`Retail Blizzard_Minimap/Mainline/Minimap.xml:99`).
- [x] Modern serialized trees remain byte-identical to the pre-skin fixture; a skin switch rebuilds the cluster canvas and composite.

Header geometry comes from FlareUI; the flat bevel palette is reference-sampled. Inherited engine arrow/blip sizes remain assumptions listed below. Magnifier, buffs and objective tracker placements stay unchanged; the default Forever tracker at TOPRIGHT `(0,-300)` must clear the minimap. The day/night badge uses locally extracted Retail FDID 136484, 50×50 sprite cells on its 128×64 sheet (`GameTime.lua:76-88`). Its rendered rect is `(2,226,33,32)` in cluster-local units: measured reference ~57×55 pixels / map enlargement `(443−26)/244 ≈ 1.71` gives ~33.35×32.18, rounded to 33×32 **after** FlareUI's 0.9 scale. Do not apply 0.9 again or infer the missing native XML size. FlareUI `Modules/Minimap.lua:48-49,273-276` supplies six-unit left/bottom overhang. At 1920×1080 the badge `(1662,226,33,32)` clears the unchanged magnifier `(1624,230,30,30)` by eight horizontal units; tracker and buffs do not move. Like the clock, the badge follows local time (dawn 05:30, dusk 21:00), not unspecified realm time.

### Not done
- [ ] Indoor (WMO) minimaps, the indoor diameter table, and `rotateMinimap` 1.
- [x] Creature vignettes: every replicated unit with `UnitVignette` whose `Vignette.csv` row lacks `DontShowOnMinimap` shows `VignetteKill`, or `VignetteKillElite` for elite/rare-elite/world-boss classifications, at its offset; the server adds and removes `UnitVignette` with the unit's 100 yd visibility and its death (`godot/rust/src/vignettes.rs`).
- [ ] Vignette blip size (16, as quest blips) and hover tooltip; infinite-AOI (map/zone-wide) vignettes are not sent.
- [ ] Find Herbs/Minerals and Track Humanoids remain unsupported. Audit (2026-10-07): NPC flags and game-object metadata are replicated (`shared-protocol/src/protocol/interaction_messages.rs`); creature type is only returned by tooltip queries (`protocol/tooltip_messages.rs:53-54`, server `creature_tooltip.rs`), not a complete tracking feed. No active spell-tracking masks or herb/mineral source stream exists. Do not invent spell-tracking results from arbitrary NPCs or static scenery. Mailbox tracking and quest POI edge arrows remain separate gaps; game-object mailboxes have metadata but are not included in this bounded NPC-service implementation. No protocol/server change.
- [ ] Mail, crafting-order and instance-difficulty indicators; the expansion landing-page button; `AddonCompartment`.
- [ ] Clock and calendar tooltips and their clicks (TimeManager, Calendar). Realm time: the protocol carries none, so the clock shows local time. Military time.
- [ ] Minimap ping and the zone tooltip. Edit-mode size, the header-underneath option, and moving the cluster.
- [ ] Exact player arrow and blip sizes. These are engine-drawn, so the 32 and 16 unit sizes are assumptions.

## Native proof — 2026-10-07

Production `ded3dee3a`, observer `0ba1098ed`, private server `3f87ae7`, UDP 5316, Weston `mb36`. Canonical `data/diagnostics/minimapblips-20261007/` owns `proof-ledger.txt`, exact argv/PID manifests, `final-inspection.json` and `cleanup.json`. Two real clients used accounts `fb_blips1`/`fb_blips2`, characters Blipsone (57, Paladin) and Blipstwo (58, Mage), with the requested offline redb copy and read-only world snapshot. Four total rendered launches, at most two concurrent; the final client switched Modern → Forever through native Options, without another launch.

- Inspected `final-{modern,forever}-{near,far,target}.png` and their crops: 40 yd east dot, 600 yd east edge arrow, and selected Blipstwo target overlay with server echo. Rounded dot centres are `(1819,143)` / `(1811,130)`; edge centres `(1893,143)` / `(1904,130)`. Native arrow rotation is +π/2. The dot's native tint is Mage `[0.25,0.78,0.92,1]`; arrow art retains authored colours. `inspect-final.py` independently checks positions, tint/rotation, target echo and coloured pixels (cyan 36/35, green 3/3, target 61/61).
- Inspected `final-forever-north.png`: a 600 yd north member clamps at `(1790,38)` with icon top 30, below the opaque header, facing north.
- Inspected both skins' `final-*-tracking-menu` and visible result captures: Class Trainers checked, three live service markers; Forever off → zero/on → three and outside-click close observed. Selection survives the live skin switch. Other supported categories have concrete flag fixtures, not individual live service captures.
- Targeted RED/GREEN: five member/target/arrow cases, three tracking cases, eight existing skin cases and the 17-case native/integration `minimap` filter. The final art fix reran only its affected five-case target. Matching extension/CLI build and package `cargo fmt --check` pass. No full suite or protocol/server change.
- Cleanup verified: recorded exact runtime PIDs terminated with TERM, no KILL survivors; owned slice inactive and UDP 5316 free. Shared :5000 and other jobs untouched, targets/caches retained. This proves termination/isolation, not natural Godot shutdown.

Earlier grey-arrow captures are regression evidence, not final acceptance. Rendered proof uses test-only Dozen Vulkan under private Weston, not WSLg or a reference-pixel Retail comparison. Indoor/WMO maps and ping remain excluded; spell tracking and mailbox tracking retain the gaps above.

## Tests asserting this spec

- `godot/ui-model/tests/minimapblips.rs`: concrete member positions, offsets, texture colours, dot/arrow rotation and round/square edge clamping; target placement and removal; raid art and offline/missing/left-member exclusion. Direction/header regression RED then GREEN: five blip cases and eight existing both-skin minimap cases passed. Final integration/live proof recorded in canonical `data/diagnostics/minimapblips-20261007/proof-ledger.txt`.
- `godot/ui-model/tests/minimap_tracking.rs`: nine supported role flags, local selection toggles and invalid actions, duplicate-role suppression, concrete positions and range clipping, menu check marks and selected service atlas art in both skins. Three cases passed; live native class-filter toggles are recorded below.

- `godot/core/tests/minimap_data.rs`: Northshire tile key and path; the composite's orientation, missing-tile colour and round mask; the texel under the player; blip offsets and edge; zoom clamps; clock text; zone text and PvP colours; race faction.
- `godot/ui-model/tests/forever_minimap.rs`: the Modern trees equal the pre-skin fixture; the Forever cluster, map, header texts, border, blips, arrow, zoom buttons and mail rects; a live skin switch. `godot/core/tests/minimap_data.rs` covers the square mask's corners and corner blips.
- `godot/rust/src/vignettes_tests.rs`: Doomwalker (Vignette 6520, elite) draws one `VignetteKillElite` blip 50 yd north; a `DontShowOnMinimap` vignette, one past the edge and one without `UnitVignette` draw none. `godot/tests/vignettes_live.gd` (live, private server, Doomwalker forced active): screenshots in `data/diagnostics/vignettes-2026-10-02/`.
- `godot/tests/world_minimap_quest.gd` (live, private server): the drawn tile is the player's `map32_48`, the composite centre is that tile's texel, and the zone text is "Northshire Valley". The arrow's rotation matches the facing, and the arrow points along W movement. A McBride blip shows. Hover shows the zoom buttons, the wheel zooms in and ZoomOut zooms out. Screenshots are in `data/diagnostics/minimapquest-2026-09-29/`.

## Implementation inventory

- `godot/core/src/minimap_data.rs`: tile keys and paths, composite, blip offset, zoom, clock, `AreaCatalog` zone text and PvP, `ChrRaces` faction groups.
- `godot/ui-model/src/minimap.rs`: the `MinimapCluster` rsx screen, atlas crops, and postsetup for the arrow rotation and the dynamic composite.
- `godot/rust/src/minimap.rs`: the host. It handles tile loading from local CASC, recompositing, quest-giver queries, hover and wheel, actions, and the `minimap_state()` fixture view.
