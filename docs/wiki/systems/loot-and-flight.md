# Loot and Flight Masters

How corpses, the LootFrame, the flight map and server-driven flights fit together on the client. The server rules are game-server `docs/specs/loot.md` and `docs/specs/taxi.md`; the client contracts are `docs/specs/loot-frame.md` and `docs/specs/flight-master.md`.

## Loot

- The server keeps a dead NPC replicated as a corpse until its decay; `Health.current == 0` is the client's dead state. `rendering/model/animation/death.rs` marks the NPC's animated models `DeathPose` (Death once, held); `switch_animation` and emotes skip them.
- `CorpseLootable` (per looter, not replicated) inserts or removes `loot_state::Lootable` on the mirrored entity (`networking/loot.rs`). `quest_sparkle.rs` adds a gold sparkle child for `Lootable`; `wow_cursor.rs` shows `LootAll`.
- Right-click (`target.rs::interact_or_loot`) and IPC `quest interact` both use `loot_state::npc_right_click`: loot a `Lootable` corpse (`LootRequest::Open`, auto = Auto Loot option XOR Shift), only target another corpse, interact with a living NPC.
- `LootState` holds the open corpse's remaining slots; `scenes/loot_frame` places the frame under the cursor once per corpse (`LootFrameAnchor`) and turns card clicks into `LootRequest::Take`.
- Chrome: `quest_art::flat_panel_chrome` = `DefaultPanelFlatTemplate` on the `metal_frame_no_portrait` panel style (the portrait layout with `ui-frame-metal-cornertopleft-2x` in the top-left cell).

## Flight

- `TaxiMap` fills `TaxiMapState`; `scenes/flight_map` loads the continent art once per `MapID` from the UiMap CSVs (`FlightMapArt::load`) and lays out tiles, pins, route dots and the tooltip; hovering updates `TaxiMapState.hovered`.
- `MovementControl` (replicated on every `Player`, `#[require]`d) drives `networking/server_movement.rs`, which runs between `player_movement` and `camera_follow`: a new epoch snaps the local transform to the replicated `Position`; while `controlled`, the transform follows it like a remote unit and `player_movement` stops input and local physics.
- The server moves the player along the `TaxiPathNode` points; the mount is the replicated `Mounted` display, so a flying player renders as the gryphon without a rider.

## Gotchas

- UI textures load only from `data/textures/<fdid>.blp`; they are not extracted on demand. Extract with `asset-resolver`'s `casc-local <fdid>... -o data/textures`. 23 Eastern Kingdoms map tiles are not in the local CASC install.
- Zone crossings during a flight enter the Loading state; `OnExit(InWorld)` resets loot and taxi state.
- The gossip option frame is `GossipOption<id>Text` (the clickable font string), not `GossipOption<id>`.

## Sources

- [loot-frame spec](../../specs/loot-frame.md), [flight-master spec](../../specs/flight-master.md)
- Live run: `data/diagnostics/npcloot-20260925/`

## See Also

- [[merchant-frame]] — the same Retail-window scene pattern and bag deltas
- [[networking]] — replication mirror that now carries `MovementControl`
- [[animation]] — the `DeathPose` exception to movement animation
