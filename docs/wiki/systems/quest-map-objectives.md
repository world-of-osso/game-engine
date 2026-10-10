# Quest map objectives

Native quest map integration. Contract: [quest map objectives](../../specs/quest-map-objectives.md). Verified source census: 2026-10-10; rendering acceptance remains pending.

## Source ownership

Local Retail CASC build `dcfc90fffd79ba00406ae46f5f657592` contains QuestPOIBlob FDID1251882, layoutFDC814CF, and QuestPOIPoint FDID1251883, layout5CBBEFE7. `scripts/export_db2_csv.py` exports 72,381 blobs and 170,335 points, zero encrypted records dropped. Point relationship column is blob ID; coordinates are signed 16-bit world XY. Blob objective index is signed32, `-1` turn-in and `32` navigation. WoWDBDefs definitions confirm the layout; no web quest geometry is used.

Catalog covers 21,765 quest IDs. Of 12,343 `content_quest_template` IDs in server world.db, 4,599 have local DB2 POIs, 11,700 have authored server POIs, and 641 have neither. Server `quest_data::load_pois` already reads `content_quest_poi` and ordered `content_quest_poi_points`; existing QuestEntrySnapshot/QuestLogUpdate replication is sufficient for non-local quest IDs. No protocol expansion.

Locally covered quests use only their local rows, including an empty list if every row is conditional. Other quest IDs retain server-authored snapshot geometry. 6,679 conditional local blobs are excluded until condition evaluation exists. Source blob519073 (quest85461 navigation marker) declares one point but relates ten: explicitly reported and excluded. Two conditional quest38576 blobs also have point-count mismatches. No synthetic replacement.

Quest28766, Beating Them Back!, has blob57135 with seven points, beginning `(-8894,-138)`, and turn-in blob57134 at `(-8913,-137)`. Its server POI coordinates match these client rows. Quest7, Kobold Camp Cleanup, is absent from the Retail DB2 but has an authentic server polygon.

## Rendering and lifecycle

`QuestPoiCatalog::apply` overlays local geometry onto each new authoritative objective snapshot. Server progress/completion remains authoritative. Map quest order is watched IDs followed by remaining log entries, keeping tracker and map numbers equal. Objective completion excludes its POI before the whole quest is complete; whole-quest completion selects only `-1` turn-in points.

`quest_area_data` samples extracted blue fill342529 and white world-map rim342531. Minimap uses the same blue fill with authored minimap rim533895 and selected rim1083696 (Retail minimap fill533894 is transparent; the user explicitly requested filled blue areas on both maps). Normal fill/border alpha128/192 follows QuestBlobDataProvider OnLoad; selected areas use stronger fill and selected rim. Geometry rasterization uses even-odd containment and nearest-edge distance; exact Blizzard native tessellation/filtering is not claimed.

`MinimapView.rotation` rotates pixel sampling, objective polygons, blip offsets and inverse click projection together. Native `set_minimap_rotation(bool)` exposes the setting; no new skin-specific controls. Off-screen selected quests use SuperTrackerArrow FDID407337 and the same inset mask-boundary calculation as member arrows. Available `!` pins use authoritative queried giver statuses and replicated positions only, not an invented global giver catalog.

## Sources

- Cached Retail `Blizzard_SharedMapDataProviders/{QuestDataProvider,QuestBlobDataProvider}.lua`, `Blizzard_ObjectiveTracker/Blizzard_QuestObjectiveTracker.lua`.
- wowdev/WoWDBDefs `definitions/QuestPOIBlob.dbd` and `QuestPOIPoint.dbd`, layoutsFDC814CF/5CBBEFE7.
- `godot/{core,ui-model,rust}/src` files listed in the [contract](../../specs/quest-map-objectives.md).
- Persistent census, raw local DB2 files, art and RED receipts: canonical `data/diagnostics/questpoi-2026-10-10/`.

## See Also

- [[minimap]] — map mask, tile sampling and giver status lifecycle.
- [[world-map]] — world XY to UiMap transform.
- [[quest-ui]] — authoritative quest snapshots and tracker.
