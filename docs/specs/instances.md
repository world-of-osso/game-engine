# Map transfers and WMO-only maps (client)

Leaving one map for another and loading dungeons made of a single WMO. Contract: shared-protocol `protocol/transfer_messages.rs` (`NewWorld`, `WorldPortAck`, `TransferAborted`). Server rules: game-server `docs/specs/instances.md`.

## What it must do

### Map transfer
- [x] `NewWorld { map_id, map_directory, position, facing }` (Retail `SMSG_NEW_WORLD`) despawns the old map's terrain tiles and global WMO, sets the streamed map to `map_directory`, puts the local player at `position` facing `facing`, and shows the loading screen. The old map's units leave through replication (the server hides them).
- [x] When the loading screen ends (entering InWorld) the client sends one `WorldPortAck` (`CMSG_WORLD_PORT_RESPONSE`); never in the frame `NewWorld` is read.
- [x] `TransferAborted` shows its Retail GlobalStrings text in UIErrorsFrame ("Transfer Aborted: instance is full", "Transfer Aborted: instance not found", "Map cannot be entered at this time.").
- [x] Logging in on another map loads that map: `LoadTerrain.map_name` is the map's directory (`kalimdor`, `stormwindjail`).
- [x] An open `CONFIRM_SUMMON` offer shows again after a loading screen with the time left.

### WMO-only maps (WDT MPHD flag 0x1)
- [x] The map's WDT (`world/maps/<dir>/<dir>.wdt`) naming a global WMO in its MODF makes the map that one WMO: no ADT tiles stream; the WMO spawns at its origin-relative placement (`shared::ground::global_wmo_placement_position`; Stockade WDT 791060 → WMO 108631 at 0,0,0).
- [x] Loading completes when the global WMO has spawned; a WMO that fails shows "Terrain failed to load".
- [x] WMO floors are the map's only ground: no terrain is waited for (the character falls when no floor supports it), and the camera is not clamped to a terrain height (the WMO walls bound it).

## Gaps
- The loading screen is the generic one; `Map.db2` LoadingScreenID art is not used.
- Zone name and minimap show stale or "Unknown" data inside a WMO-only map (no WMO area lookup).
- Party members outside the interest radius, or on another map, cannot be targeted, so a summon cannot be started for them from the client.

## Live evidence (2026-09-26)
Headless clients on an own :5081 server, screenshots in `data/diagnostics/instances-20260926/`: Stockade entrance (`01`) → inside map 34 (`03`) → loading screen on the exit (`04`) → Stormwind (`05`); grouped players visible to each other inside (`06`), a solo player's copy empty (`07`); Kalimdor terrain and NPCs after a transfer (`12`); the summon offer re-shown after the transfer (`12`, `15`).

## Tests asserting this spec
`cargo test --bin game-engine networking_transfer` (NewWorld to map 34, one ack after loading, aborted text), `game_state` (WMO-only loading), `collision` (WMO-only ground), `camera_follow` (camera below the world origin on a WMO-only map), `summon_popup` (offer after a loading screen); `cargo test --lib wdt::` (Stockade WDT global WMO).
