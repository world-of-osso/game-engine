# Flight Master

Native Godot flight map over shared-protocol `TaxiMap`/`ActivateTaxi` and replicated `MovementControl`. Source: `godot/rust/src/flight_map.rs` and `godot/ui-model/src/flight_map*.rs`. Server contract: game-server `docs/specs/taxi.md`. [Implementation](../wiki/systems/native-flight-map.md).

Retail references under cached `retail/AddOns/`:
- `Blizzard_FlightMap/Blizzard_FlightMap.xml:5-24`: 1004×689 portrait frame and map canvas.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:131-166`: first-hop background routes from current node.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:188-193`: hide undiscovered pins.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:233-269`: left-click TakeTaxiNode, name/current/cost tooltip and highlighted route.
- `Blizzard_FlightMap/FM_FlightPathDataProvider.lua:287-349`: current 28px green, reachable 20px gray/yellow pins.

## What it must do

- [ ] Talking to a flight master opens its continent map from the server's `TaxiMap`.
- [ ] Draw current and reachable taxi nodes projected through the shared world-map catalog; mark current node green. Do not draw undiscovered nodes.
- [ ] Draw deduplicated first-hop routes from current node; hovering a reachable destination shows its full route.
- [ ] Hover shows destination name and copper cost expressed as gold/silver/copper, or “You are here” for current node.
- [ ] Clicking a reachable destination emits exactly `ActivateTaxi { npc, destination }` and closes the frame. Current/unknown/unreachable nodes cannot activate.
- [ ] Close button/Escape send `CloseInteraction`; server interaction closure closes matching frame. Leaving InWorld disposes it.
- [ ] Server refusals and discoveries show their UI error text. Flight starts and advances from replicated server movement, not client preview motion.

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
- `godot/tests/flight_map_live.gd`: real NPC pick → native map → hover → native click → closed map/replicated controlled flight and position advance on private server.

## Known gaps (current cycle)

- [ ] Current revision targeted tests and private rendered proof pending.

## Out of scope

Zoom/pan, portrait masking, texture-kit/special pins, reveal animations, early landing and rotated line textures. Routes use dots, as the retired client did; not full Retail visual parity. Missing local-CASC continent tiles remain undrawn and counted. Rider-on-mount rendering and zone-loading transitions remain existing client behavior.

## Historical evidence

Retired Bevy implementation paths (`src/game/taxi_state.rs`, `src/ui/screens/flight_map_component_tests.rs`, `src/scenes/flight_map/tests.rs`) are not native acceptance evidence. Historical captures: `data/diagnostics/npcloot-20260925/` (t08/t12/t15 maps, t09 flight, t11/t14 landings).
