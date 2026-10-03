# Flight Master

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

The Retail `FlightMapFrame` and server-driven flights. The contract is shared-protocol `protocol/taxi_messages.rs` and the replicated `MovementControl`; server rules are in game-server `docs/specs/taxi.md`.

References:
- FM.xml / FM.lua = `Blizzard_FlightMap/Blizzard_FlightMap.xml` / `.lua`
- FPD.lua = `Blizzard_FlightMap/FM_FlightPathDataProvider.lua`
- MapCanvas.xml = `Blizzard_MapCanvas/Blizzard_MapCanvas.xml`
- `TAXIMAP_OPENED` picks `FlightMapFrame` for every map system but `Taxi` (`Blizzard_Game/Mainline/EventImplementation.lua:722-728`)

## What it must do

- [x] `TaxiMap` opens the frame; the end of the flight master interaction (take-off, walking away) closes it; the close button sends `CloseInteraction`.
- [x] `TaxiNodeDiscovered` shows `ERR_NEWTAXIPATH` "New location discovered!"; `TaxiFailed` shows its Retail text (`ERR_TAXINOTENOUGHMONEY`, `ERR_TAXINOTVISITED`, ...).
- [x] Frame: 1004×689 centred (FM.xml:5-8), `PortraitFrameTemplate` border titled `FLIGHT_MAP` "Flight Map", black background (FM.lua `SetupTitle`), `AdventureMap_TopBorder` at 2,22 / -3,2 (FM.xml:17-24).
- [x] Map art: the continent `UiMap` (type Continent, whole-map `UiMapAssignment` on the node's `MapID`: Eastern Kingdoms = 13) → `UiMapXMapArt` → `UiMapArt` → `UiMapArtStyleLayer` layer 0 (3840×2560, 256 tiles) → 150 `UiMapArtTile`s, fitted and centred in the 1002×668 canvas at 1,20 (`ResetZoom`), whole-pixel tiles over the tiled `AdventureMap_TileBg` (MapCanvas.xml:33-37). CSVs in `data/db2/12.1.0.69933/`.
- [x] Node positions follow `C_Map.GetMapPosFromWorldPos` over the assignment `Region`: x = (maxY - y)/(maxY - minY), y = (maxX - x)/(maxX - minX).
- [x] Pins (FPD.lua:287-349): current `Taxi_Frame_Green` 28, reachable `Taxi_Frame_Gray` 20 (hovered `Taxi_Frame_Yellow`), unreachable `UI-Taxi-Icon-Nub` 14 and shown only on the hovered route (FPD.lua:193). Unknown flight points are therefore not drawn, as in Retail.
- [x] Lines: with nothing hovered, current → first hop of every reachable node (`ShowBackgroundRoutesFromCurrent`, FPD.lua:131-166); hovering a reachable node draws its whole route (FPD.lua:66-100).
- [x] Tooltip at the pin's TOPRIGHT (FPD.lua:240-269): name, then `TAXINODEYOUAREHERE` "You are here", the cost in coins, or `TAXI_PATH_UNREACHABLE` "Not Discovered" in red.
- [x] Left-clicking a pin sends `ActivateTaxi` (`TakeTaxiNode`).
- [x] Flight: while `MovementControl.controlled` the local player follows the replicated position and facing, with no input and no local physics; the camera stays free. The mount shows as the Riding Gryphon.
- [x] Every new `MovementControl.epoch` (landing, spirit release, resurrection, admin `set-position`) snaps the local player to the server position.
- [x] The client-only preview flight (`taxi.rs`, world-map taxi pin clicks, `TaxiCameraTarget`) is gone.
- [ ] The rider is not drawn on the mount (a mounted player renders as the mount only).
- [ ] Not built: the portrait icon (`icon_petfamily_flying`, needs a circle mask), zoom and pan, the rotated `_UI-Taxi-Line-horizontal` line texture (route lines are dots), pin reveal animations, special/texture-kit pins, the early-landing button, the gossip taxi icon.
- [ ] 23 of the 150 Eastern Kingdoms tiles are not in the local CASC install, so the tiled background shows through them.
- [ ] Each zone crossing during a flight shows the client loading screen (existing zone-transition behaviour, not Retail).

## Tests asserting this spec

- `src/game/taxi_state.rs` tests: Eastern Kingdoms art is UiMap 13 in 150 tiles, Stormwind lands at its Retail map position, map state.
- `src/ui/screens/flight_map_component_tests.rs`: frame, title, tile offset, pin sizes and clicks, route dots, tooltip cost and "You are here".
- `src/scenes/flight_map/tests.rs`: fitted canvas and pin positions, seamless tiles and background, hidden unreachable nodes and background lines, hovered route and tooltip, clicks.
- `src/game/networking/server_movement_tests.rs`: teleport snap and controlled follow.
- `src/network_runtime/replication.rs` tests: `MovementControl` mirrored.
- Live evidence: `data/diagnostics/npcloot-20260925/` (t08/t12/t15 flight maps, t09 in flight, t11 landed on the Sentinel Hill platform, t14 Stormwind landing).
