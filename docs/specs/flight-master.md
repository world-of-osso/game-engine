# Flight Master

Native Godot flight map over shared-protocol `TaxiMap`/`ActivateTaxi` and replicated `MovementControl`. Source: `godot/rust/src/flight_map.rs` and `godot/ui-model/src/flight_map*.rs`. Server contract: game-server `docs/specs/taxi.md`. [Implementation](../wiki/systems/native-flight-map.md).

Retail references under cached `retail/AddOns/`:
- `Blizzard_FlightMap/Blizzard_FlightMap.xml:5-24`: 1004×689 portrait frame and map canvas.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:131-166`: first-hop background routes from current node.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:188-193`: hide undiscovered pins.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:233-269`: left-click TakeTaxiNode, name/current/cost tooltip and highlighted route.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:287-349`: current 28px green, reachable 20px gray/yellow pins.
- `Blizzard_UIPanels_Game/Shared/TaxiFrame.lua:128-162`: close on TAXIMAP_CLOSED, destination name and money tooltip.

## What it must do

- [x] Talking to a flight master opens its continent map from the server's `TaxiMap` (after selecting its ride gossip option where offered).
- [x] Fill the portrait ring with Retail's authored `Interface/Icons/icon_petfamily_flying` (FDID 618976), rounded by the existing window portrait mask (`Blizzard_FlightMap.lua:12-18`), not the interacting NPC's face.
- [x] Draw current and reachable taxi nodes projected through the shared world-map catalog; mark current node green. Do not draw undiscovered nodes.
- [x] Draw deduplicated first-hop routes from current node; hovering a reachable destination shows its full route.
- [x] Hover shows destination name and the existing SmallMoneyFrame amount/coin pairs (white amounts, gold/silver/copper icons; omit zero denominations except 0 copper for a free route), or “You are here” for current node. Never spell costs as `0g 0s 5c`.
- [x] Clicking a reachable destination emits exactly `ActivateTaxi { npc, destination }` and closes the frame. Current/unknown/unreachable nodes cannot activate.
- [ ] Close button/Escape send `CloseInteraction`; server interaction closure closes matching frame. Leaving InWorld disposes it.
- [x] Server refusals show their UI error text.
- [ ] Discoveries show their UI error text (implemented; not exercised by this run).
- [x] Flight starts and advances from replicated server movement, not client preview motion.

## How it works

- [Native flight map](../wiki/systems/native-flight-map.md)
- [World map](world-map.md)
- [Private live-run procedure](../headless-live-run.md)

## Implementation inventory

- `godot/network/src/lib.rs`: taxi server-message relay.
- `godot/rust/src/account.rs`: decode taxi messages and send activation through TaxiChannel.
- `godot/rust/src/flight_map.rs`: lifecycle, input, native registry and local-CASC textures.
- `godot/rust/src/flight_map_preview.rs`: read-only real-session/replicated-flight observation.
- `godot/ui-model/src/flight_map.rs`: session, continent selection, node/route projection and tooltips.
- `godot/ui-model/src/flight_map_component.rs`: Retail frame, taxi pins and route overlay.
- `godot/ui-model/src/ui/screens/world_map_frame_component/canvas.rs`: shared tiled canvas.
- `godot/rust/src/world.rs`: existing replicated MovementControl follow, unchanged.

## Tests asserting this spec

- `godot/ui-model/src/flight_map.rs`: concrete Stormwind/Sentinel Hill projection, multi-hop routes, tooltip cost, exact click request and close-on-activation, rejected destinations.
- `godot/ui-model/tests/capturepolish.rs`: 5 copper, 1 gold/23 silver/45 copper and free routes project native coin textures and amounts; current node has no money row.
- `godot/ui-model/tests/npcportraits.rs` and native `ui::icon_masks` regression: authored flying icon in both skins, transparent corners and nonblack retained art.
- `godot/tests/flight_map_live.gd`: real NPC pick → native map → hover → native click → closed map/replicated controlled flight and position advance on private server.

## Known gaps (current cycle)

Historical pre-polish bounded native proof at `5e5ec97b`: four targeted Rust tests pass (projection/request/closure/refusal); unchanged pure-test scope from `adcf3e55`, final native extension/CLI build and format check pass. Private Stormwind → Sentinel Hill live fixture exits 0 with visible native tooltip “Sentinel Hill, Westfall / 0g 0s 5c”, closed map and controlled server position advance. All three captures inspected under canonical `data/diagnostics/flightmap-20261007/shots-accepted/`; commands, failures, revisions and cleanup in that run's `proof-ledger.txt`. Four rendered launches total.

Full Retail visual parity and broad lifecycle/other-continent proof are not claimed. Unchanged mount renderer rejects taxi mount display 6852 as outside imported appearance coverage; movement is proven, mount visuals are not. Editor import finished but exited 134 at shutdown; rendered client exits 0 with RID/ObjectDB/paged-allocator resource errors. These are retained boundaries, not clean-resource/general-shutdown acceptance.

## Capture-polish proof (2026-10-07)

Portrait art `f5d63dea` plus quest-window mask wiring `8949b789`; money rendering `eb75d925` / `5c7fc842`. Targeted source/mask/money regressions pass. Offline `capturepolish_screens.gd` verifies transparent portrait corners, nonblack authored art, amount `5` and visible copper-coin pixels. Inspected `data/diagnostics/capturepolish-20261007/shots/flight_map_preview.png`; this is chrome/tooltip proof, not tiled-map or live-flight acceptance. Commands and failures remain in that run's `proof-ledger.txt`.

## Out of scope

Zoom/pan, texture-kit/special pins, reveal animations, early landing and rotated line textures. Routes use dots, as the retired client did; not full Retail visual parity. Missing local-CASC continent tiles remain undrawn and counted. Rider-on-mount rendering and zone-loading transitions remain existing client behavior.

## Historical evidence

Retired Bevy implementation paths (`src/game/taxi_state.rs`, `src/ui/screens/flight_map_component_tests.rs`, `src/scenes/flight_map/tests.rs`) are not native acceptance evidence. Historical captures: `data/diagnostics/npcloot-20260925/` (t08/t12/t15 maps, t09 flight, t11/t14 landings).
