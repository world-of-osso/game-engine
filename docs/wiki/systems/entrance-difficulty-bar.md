# Entrance Difficulty Bar

The Godot client's dungeon-entrance difficulty bar, a port of the Plumber addon's "Instance Difficulty Selector" (`Modules/RaidCheck`). Near a dungeon entrance the instance name and its difficulties appear at the top of the screen; clicking one sets the dungeon difficulty. Requirements: [instances spec](../../specs/instances.md), "Entrance difficulty bar".

## Data

- `JournalInstanceEntrance.db2` (FDID 5228481, layout 874E7CC2): `Position[3]` is one 96-bit inline field of three floats, `MapID`, `AreaTableID`, `Faction<8>` (-1 both; only Battle of Dazar'alor uses 0/1) and the `JournalInstanceID` relation. 203 rows: 141 dungeon, 62 raid entrances.
- `JournalInstance.db2` (FDID 1237438, layout 6C5ED7F2) is locale-split. `casc-local` picks a root entry the local install lacks ("Archive location not found"); the enUS copy comes from game-server `scripts/casc_locale.py` `read_enus`. `Name_lang` is the Encounter Journal name ("The Stockade"), not `Map.MapName_lang` ("Stormwind Stockade").
- Both are exported by `scripts/export_db2_csv.py` to the shared `data/db2/12.1.0.69933/`.
- Difficulties are Plumber's `[1, 2, 23]` ∩ `MapDifficulty` rows, an approximation of `EJ_IsValidInstanceDifficulty`; boss totals are `DungeonEncounter` rows of difficulty 0 or that difficulty, standing in for `EJ_GetEncounterInfoByIndex` (Deadmines Heroic lists 6: Vanessa VanCleef is Heroic-only).

## Code

- `src/dungeon_entrance_data.rs` (in `godot/ui-model`): `EntranceCatalog` loads the tables; `nearest_entrance` is Plumber's 2D proximity; `selector` builds title, choices and selection.
- `src/ui/screens/entrance_difficulty_component.rs`: the rsx screen and `EntranceBarLayout` (Plumber `Def` geometry). Two layers: the dim layer (shadow, bar, glow) at the bar alpha, and the opaque layer (title, box, every button). Buttons never change parent: moving the selected one between layers, as Plumber does by re-parenting, made the diff re-create its frame and Godot node, losing hover and invalidating fixture references. Unselected buttons carry the bar alpha instead. The box is created after the buttons, so buttons take frame level 5 to draw above it.
- `godot/rust/src/entrance_bar.rs`: host. Measures labels with FRIZQT at 14 px, tracks the pointer against the motion rect, runs the fades, box ease and spinner, sends `SetDungeonDifficulty` and `RequestRaidInfo`. Fixture hooks: `entrance_bar_state()`, `set_dungeon_difficulty(id)`.
- Godot UI projection additions for it: font-string `OUTLINE` (outline size 2, black) and TGA textures (`Image.load_tga_from_buffer`).

## Protocol

No new messages. The server persists the dungeon difficulty per character and sends `DungeonDifficultySet` at login and on change (game-server `maps/difficulty.rs`); the Godot network bridge now relays it and `InstanceInfo`.

## Live proof

`godot/tests/entrance_difficulty_bar.gd` on :5000 as Fbstockade. Put the character at the entrance while offline with `game-server-admin teleport Fbstockade 0 -8766.11 845.5 88`; `set-position` keeps the saved map, and a character last saved inside the Stockade stays on map 34. Evidence and Plumber reference screenshots: `data/diagnostics/entrancebar-20260928/`.

## Sources

- [entrancepicker handoff](../../../data/diagnostics/handoff/entrancepicker.md): Plumber behaviour, constants and texture rects

## See Also

- [[stockade-entrance]]: the entrance stairwell the fixture walks
- [[ui-system]]: rsx screens and the registry projection
