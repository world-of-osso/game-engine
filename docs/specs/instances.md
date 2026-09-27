# Map transfers, WMO-only maps and dungeon difficulties (client)

Leaving one map for another, loading dungeons made of a single WMO, picking the dungeon difficulty and seeing saved instances. Contract: shared-protocol `protocol/transfer_messages.rs` (`NewWorld`, `WorldPortAck`, `TransferAborted`), `protocol/instance_messages.rs` (difficulty, locks). Server rules: game-server `docs/specs/instances.md`.

## What it must do

### Map transfer
- [x] `NewWorld { map_id, map_directory, position, facing }` (Retail `SMSG_NEW_WORLD`) despawns the old map's terrain tiles and global WMO, sets the streamed map to `map_directory`, puts the local player at `position` facing `facing`, and shows the loading screen. The old map's units leave through replication (the server hides them).
- [x] Once the new map has loaded the client sends one `WorldPortAck` (`CMSG_WORLD_PORT_RESPONSE`), still behind the loading screen; never in the frame `NewWorld` is read.
- [x] The loading screen stays until the local player's replicated `WorldArrival` count exceeds the one it had at `NewWorld`: the server raises it in the replication tick that sends the destination's creatures, whose models spawn as they are replicated, so the first in-world frames do not load them.
- [x] `TransferAborted` shows its Retail GlobalStrings text in UIErrorsFrame ("Transfer Aborted: instance is full", "Transfer Aborted: instance not found", "Map cannot be entered at this time.", "Heroic difficulty mode is not available for %s.", "You are already locked to %s."), `%s` the `Map.db2` name.
- [x] Logging in on another map loads that map: `LoadTerrain.map_name` is the map's directory (`kalimdor`, `stormwindjail`).
- [x] An open `CONFIRM_SUMMON` offer shows again after a loading screen with the time left.

### WMO-only maps (WDT MPHD flag 0x1)
- [x] The map's WDT (`world/maps/<dir>/<dir>.wdt`) naming a global WMO in its MODF makes the map that one WMO: no ADT tiles stream; the WMO spawns at its origin-relative placement (`shared::ground::global_wmo_placement_position`; Stockade WDT 791060 → WMO 108631 at 0,0,0).
- [x] Loading completes when the global WMO has spawned; a WMO that fails shows "Terrain failed to load".
- [x] WMO floors are the map's only ground: no terrain is waited for (the character falls when no floor supports it), and the camera is not clamped to a terrain height (the WMO walls bound it).

### Dungeon difficulty and saved instances
- [x] Difficulty names and flags come from `Difficulty.csv`, map names from `Map.csv` (quoted multi-line fields), lock extension flags from `MapDifficulty.csv` (build 69933, `data/db2/12.1.0.69933/`).
- [x] The player frame's right-click menu (`UnitPopupMenuSelf`) has "Dungeon Difficulty" (`UnitPopupDungeonDifficultyButtonMixin`), opening a submenu of Normal, Heroic and Mythic radios (`PLAYER_DIFFICULTY1/2/6`, difficulty 1/2/23; `common-dropdown-tickradial`, the current one `common-dropdown-icon-radialtick-yellow`). Clicking one sends `SetDungeonDifficulty` (`SetDungeonDifficultyID`). Inside an instance, or in a group for a non-leader, the radios are grey and do nothing (`DifficultyUtil.IsDungeonDifficultyEnabled`).
- [x] `DungeonDifficultySet` records the difficulty; a change prints "Dungeon Difficulty set to %s." (`ERR_DUNGEON_DIFFICULTY_CHANGED_S`) as a system line, the login value prints nothing.
- [x] `WorldServerInfo` records the copy's difficulty (`GetInstanceInfo`) and asks for the saved instances (`RequestRaidInfo`); so do `InstanceSaveCreated` ("You are now saved to this instance") and `RaidInstanceMessage` Expired ("Your instance lock for %s has expired."). `InstanceReset` prints "%s has been reset.", `InstanceResetFailed` `INSTANCE_RESET_FAILED*`.
- [x] Inside an instance the minimap shows `MinimapCluster.InstanceDifficulty` (TOPRIGHT, 15 px under the cluster top): `ui-hud-minimap-guildbanner-background-top` and `-border-top` with the `normal`/`heroic`/`mythic-large` texture of the difficulty's flags (displayMythic 0x80; heroic-style 0x01 or displayHeroic 0x40; else Normal) over the number of players in the copy; hidden on continents.
- [x] The FriendsFrame Raid tab has a "Raid Info" button (`RaidFrameRaidInfoButton`, enabled with saved instances; opening the tab sends `RequestRaidInfo`) that toggles `RaidInfoFrame` beside the frame: "Raid Information", "Your saved raid instance status.", a row per lock (gold map name, difficulty name, `SecondsToTime` time left counting down, "Expired" in grey, "Extended"), Extend button ("Extend Raid Lock", "Remove Raid Lock Extension" when extended, "Reactivate Raid Lock" when expired; disabled without a selection or for `DisableLockExtension` difficulties) sending `SetSavedInstanceExtend`, and Close.

## Gaps
- The loading screen is the generic one; `Map.db2` LoadingScreenID art is not used.
- Zone name and minimap show stale or "Unknown" data inside a WMO-only map (no WMO area lookup).
- Party members outside the interest radius, or on another map, cannot be targeted, so a summon cannot be started for them from the client.
- The unit menu and RaidInfoFrame use the engine's flat menu/panel styling, not Retail's Menu and ButtonFrameTemplate art; the Dungeon Difficulty submenu opens on click, not on hover; no Raid Difficulty or Reset All Instances entries; no instance lock popup (`INSTANCE_LOCK`) and no banner tooltip.

## Live evidence (2026-09-26)
Headless clients on an own :5081 server, screenshots in `data/diagnostics/instances-20260926/`: Stockade entrance (`01`) → inside map 34 (`03`) → loading screen on the exit (`04`) → Stormwind (`05`); grouped players visible to each other inside (`06`), a solo player's copy empty (`07`); Kalimdor terrain and NPCs after a transfer (`12`); the summon offer re-shown after the transfer (`12`, `15`).

## Tests asserting this spec
`cargo test --bin game-engine networking_transfer` (NewWorld to map 34, one ack once the map loaded, the loading screen held until the WorldArrival count rises, aborted text), `cargo test --lib world_arrival` (count mirrored), `networking_instance` (difficulty change line, WorldServerInfo, save/reset/expiry lines, lock list), `unit_frames` (Dungeon Difficulty submenu picks Heroic, disabled in a Heroic copy), `friends_frame` (Raid Info rows, extend texts and requests), `instance_banner` (minimap banner textures and count); `cargo test --lib instance_state` (catalog names, banner flags, SecondsToTime, enable rule), `game_state` (WMO-only loading), `collision` (WMO-only ground), `camera_follow` (camera below the world origin on a WMO-only map), `summon_popup` (offer after a loading screen); `cargo test --lib wdt::` (Stockade WDT global WMO).
