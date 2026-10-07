# Native Flight Map

Godot taxi UI consumes server `TaxiMap` without client-side route or flight simulation. Contract: [flight master](../../specs/flight-master.md).

## Content

`game-engine-network` relays taxi messages; `Account` decodes the map, discoveries and refusals. `FlightMapSession` owns the open snapshot and validates reachable clicks. Activation returns the exact NPC/destination request and clears the snapshot; the native host frees its registry UI and sends it on `TaxiChannel`. Matching interaction closure, Escape and leaving InWorld close the frame.

Continent selection follows the current node's best world-system UiMap lineage to its continent. Nodes use the existing UiMapAssignment projection; art uses the existing world-map view model and canvas. First-hop routes are deduplicated, while hovered reachable nodes expose all route hops. Current/known reachable pins use Retail green/gray/yellow atlases; undiscovered pins are hidden. Tooltip displays name and whole-route copper cost in coin units. Missing local-CASC tiles are omitted and counted.

Flight motion and mounts remain existing replicated `MovementControl`/`Mounted` handling in `world.rs`. `flight_map_preview.rs` exposes only read-only real-state evidence; it does not synthesize offline taxi state.

## Sources

- [Flight-master contract and Retail source citations](../../specs/flight-master.md)
- `godot/ui-model/src/flight_map.rs`, `flight_map_component.rs`
- `godot/rust/src/flight_map.rs`, `flight_map_preview.rs`
- `godot/network/src/lib.rs`, `godot/rust/src/account.rs`

## See Also

- [[godot-conversion]] — native client conversion
- [World-map contract](../../specs/world-map.md) — shared catalog and tiled art
- [Private live-run procedure](../../headless-live-run.md) — evidence isolation
