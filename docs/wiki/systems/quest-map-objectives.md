# Quest map objectives

Native quest map integration. Contract: [quest map objectives](../../specs/quest-map-objectives.md). Verified: 2026-10-10. Source census, 41 targeted tests and private real-quest both-skin rendering proof recorded below; no full-catalog or full-Retail-parity claim.

## Source ownership

Local Retail CASC build `dcfc90fffd79ba00406ae46f5f657592` contains QuestPOIBlob FDID1251882, layoutFDC814CF, and QuestPOIPoint FDID1251883, layout5CBBEFE7. `scripts/export_db2_csv.py` exports 72,381 blobs and 170,335 points, zero encrypted records dropped. Point relationship column is blob ID; coordinates are signed 16-bit world XY. Blob objective index is signed32, `-1` turn-in and `32` navigation. WoWDBDefs definitions confirm the layout; no web quest geometry is used.

Catalog covers 21,765 quest IDs. Of 12,343 `content_quest_template` IDs in server world.db, 4,599 have local DB2 POIs, 11,700 have authored server POIs, and 641 have neither. Server `quest_data::load_pois` already reads `content_quest_poi` and ordered `content_quest_poi_points`; existing QuestEntrySnapshot/QuestLogUpdate replication is sufficient for non-local quest IDs. No protocol expansion.

Locally covered quests use only their local rows, including an empty list if every row is conditional. Other quest IDs retain server-authored snapshot geometry. 6,679 conditional local blobs are excluded until condition evaluation exists. Source blob519073 (quest85461 navigation marker) declares one point but relates ten: explicitly reported and excluded. Two conditional quest38576 blobs also have point-count mismatches. No synthetic replacement.

The 146 Skyborne quest IDs in `zephras_quest_meta` have no POIs in the current server tables or Retail client DB2. Also checked LOCAL Forever product `wow_classic_beta`, buildf6e309c700cea095978aeb5d85210df4/1.60.1.70338: supported layouts,54 blobs/99points,0 encrypted records,0 Skyborne quest overlap. Those quests cannot gain invented objective positions or areas. Raw Forever exports live beside the Retail census in `forever-70338/`.

Quest28766, Beating Them Back!, has blob57135 with seven points, beginning `(-8894,-138)`, and turn-in blob57134 at `(-8913,-137)`. Its server POI coordinates match these client rows. Quest7, Kobold Camp Cleanup, is absent from the Retail DB2 but has an authentic server polygon.

## Rendering and lifecycle

`QuestPoiCatalog::apply` overlays local geometry onto each new authoritative objective snapshot. Server progress/completion remains authoritative. Map quest order is watched IDs followed by remaining log entries, keeping tracker and map numbers equal. Objective completion excludes its POI before the whole quest is complete; whole-quest completion selects only `-1` turn-in points.

`quest_area_data` samples extracted blue fill342529 and white world-map rim342531. Minimap uses the same blue fill with authored minimap rim533895 and selected rim1083696 (Retail minimap fill533894 is transparent; the user explicitly requested filled blue areas on both maps). Normal and selected fill/border alpha128/192 follows QuestBlobDataProvider OnLoad; super-tracking does not change opacity. The minimap alone has authored `UI-QuestBlobMinimap-OutsideSelected` art1083696, used only for its selected border, with the same border192. Cached Retail Lua/XML and the local listfile contain no world-map blob highlight texture/atlas or highlight alpha. Quest-log/tracker title/POI hover now adds that quest's authored world-map polygons at unchanged128/192, independently of super-tracking; leaving clears it. Matching world-map POI buttons show `UI-QuestPoi-InnerGlow` (UiTextureAtlasMember23600, atlas2549 / FDID5320914, rectangle1..33/67..99), additive as POIButton.xml:25. No invented native blob shading is applied; focused selection remains unimplemented.

Retail `QuestBlobDataProvider.lua:126-145` clears the world-map pin then draws the super-tracked quest unless focused, followed by explicit highlighted/focused/POI-hover quests. The native host draws the super-tracked and hovered quest polygons once each; quest pins retain their existing numbering/visibility. `Blizzard_Minimap/Mainline/Minimap.lua:37-44` makes QuestPOIs an always-on tracking filter, but exposes no native polygon selection policy. `minimapShowQuestBlobs` is a visibility CVar, not evidence that all watched polygons or only the selected one render. Minimap retains watched polygons pending native evidence; full minimap selection parity is not claimed. Geometry rasterization uses even-odd containment and nearest-edge distance; exact Blizzard native tessellation/filtering is not claimed.

`MinimapView.rotation` rotates pixel sampling, objective polygons, blip offsets and inverse click projection together. Native `set_minimap_rotation(bool)` exposes the setting; no new skin-specific controls. Off-screen selected quests use SuperTrackerArrow FDID407337 and the same inset mask-boundary calculation as member arrows. Available `!` pins use authoritative queried giver statuses and replicated positions only, not an invented global giver catalog.

## Proof (2026-10-10)

The v2 correction552a3f9ec supersedes the original selected-alpha assertions and opacity captures below. Targeted RED reproduces180 vs128; three corrected raster tests pass, asserting128 interior and192 exterior border for both selection states, and unchanged world-map selected pixels. Updated real-quest map test passes for no selection, absent selection, super-tracked geometry and completion. Native extension/CLI build, helper cargo check and changed-Rust format check pass without warnings; manual changed-line readability found no new violations. Existing live fixture passes on new private character32/Questpoivtwo with authentic28766 acceptance and authoritative completion. Four1920×1080 Modern/Forever objective world-map/minimap originals were FFmpeg-decoded and visually inspected, then published with `v2-` prefix. V2 receipts/captures: `data/diagnostics/questpoi-2026-10-10/v2/`, including `proof-ledger.md` and publication hashes. Owned processes/slice stopped; inherited spell80676 attachment22 and spell10848 attachment19 diagnostics remain, unrelated to this opacity fix.

- Production integration `b6496a79c`, tracker/fixture `c7c20e6f3`, minimap number fix `5508dc0f9`; mount-ready fixture `69f7b22c1`, server-only fixture `4ec52368f`, selected-alpha assertion `1284f9069` do not change production behavior. `065ab44f6` extracts the existing available-giver projection into a tested pure helper without changing its outputs.
- 41 distinct targeted tests: core blob2, core minimap12, local POI3, world map11, quest flow12, numbered minimap1. RED receipts precede selection, completion, color, rotation, catalog and visible-number fixes. Build and native cargo check pass without warnings. Changed Rust files format cleanly; workspace format check fails in unchanged `network/src/replica/codec.rs` and `ui-model/src/game_tooltip/merchant.rs`.
- Private UDP5518, fresh private redb, disposable `fb_questpoi_number`/Questpoitwo character31: real quest28766 acceptance; tracker click selects it; numbered tracker/world/minimap icons; clipped blue area; rotation; server admin objective completion emits live quest updates, removing areas and showing turn-in `?`. No client state injection.
- Final fixture `quest_map_objectives.gd` PASS. Twelve1920×1080 original PNGs decoded through FFmpeg, all scaled frames inspected, plus both-skin number crops; PNG-only publication `/syncthing/AgentShared/2026-10-10/quest-poi/`. Manifest/hashes and final logs in persistent diagnostics. Off-screen arrow has model/asset-orientation proof, not a live edge capture. Available-giver projection has real Marshal McBride position/status eligibility proof; no separate live offer-pin capture is claimed.
- Live setup corrections: server ground root is engine `data/`, not server `data/ground`; admin set-position accepts world XYZ, not engine X/height/Z. The fixture waits for mounted EnterWorld controls, not just the account screen enum. Visual inspection caught the unregistered `font_string` tag; `fontstring` plus a real displayed-text RED/GREEN regression fixed it before final publication.

Owned clients/server and agents-questpoi.slice stopped. Independent verifier could not authenticate (expired Claude OAuth); main-observed evidence only. Inherited live spell80676/attachment22 diagnostics and compositor/libdecor warnings are not quest-render regressions and were not suppressed.

## V3 source boundary (2026-10-10)

Retail source references under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:
- `Blizzard_UIPanels_Game/Mainline/QuestMapFrame.lua:2092-2117`: quest-title OnEnter calls SetHighlightedQuestID. `Blizzard_SharedMapDataProviders/QuestBlobDataProvider.lua:126-145,152-175` draws the highlighted quest alongside super-tracking.
- `Blizzard_POIButton/POIButton.lua:91-93,565-575`: managed button highlights use UI-QuestPoi-InnerGlow. `POIButton.xml:25-31` defines additive/default opacity,32×32.
- `Blizzard_Minimap/Mainline/Minimap.lua:37-44`: QuestPOIs is always-on. `Blizzard_APIDocumentationGenerated/MinimapFrameAPIDocumentation.lua:151-217` only exposes texture/alpha/ring controls; `MinimapDocumentation.lua:151-164` queries player-inside-blob, not drawable quest selection. None establishes which quests native minimap draws. Local solarityclient/WebWowViewerCpp searches found no quest-minimap selector. World-map selection is **not** evidence of minimap native selection; retained watched polygons are explicitly unverified, not claimed Retail parity.

## V3 proof (2026-10-10)

Production `cc434253f`, live fixture `04c498269`: title/POI hover draws an authored quest area and map-button InnerGlow; leave removes both without changing super-tracking. RED receipts reproduce absent hover polygons and absent button glow.29 distinct scoped tests pass: UI hover2, world-map12 (including real28766 hover), quest-flow12, core raster3. Native extension/CLI build, helper cargo check and changed-Rust format pass; changed-line manual readability found no new violations. No independent verifier claim.

`quest_poi_hover.gd` passes with disposable `fb_questpoi_v3b`/Poivthreeb character30 on private UDP5528, fresh private redb and current server aa36f28/protocol0354eca. Both skins prove tracker/log hover, rendered map-button glow, hover-leave removal, no super-tracked quest and the existing watched minimap set (one28766 area). This proves this client's current minimap behavior, **not** Retail's native selection parity.

Eight1920×1080 originals (both skins: minimap, unhovered map, tracker-hover map, log-hover map) were FFmpeg-decoded/downscaled and individually inspected before publication as `v3-*.png` to `/syncthing/AgentShared/2026-10-10/quest-poi/`. Persistent receipts, manifest/hashes and readable smaller frames: canonical `data/diagnostics/questpoi-2026-10-10/v3/`. Stale handoff server had a protocol mismatch; current server additionally required a current private SQLite backup (old snapshot lacked battle_pet_species). Final proof uses lead-supplied current binaries read-only and an owned SQLite backup, never protected UDP5000. Owned server/client stopped. Existing missing verified unit-asset receipts and compositor warnings were recorded, not suppressed.

## Sources

- Cached Retail `Blizzard_SharedMapDataProviders/{QuestDataProvider,QuestBlobDataProvider}.lua`, `Blizzard_ObjectiveTracker/Blizzard_QuestObjectiveTracker.lua`.
- wowdev/WoWDBDefs `definitions/QuestPOIBlob.dbd` and `QuestPOIPoint.dbd`, layoutsFDC814CF/5CBBEFE7.
- `godot/{core,ui-model,rust}/src` files listed in the [contract](../../specs/quest-map-objectives.md).
- Persistent census, raw local DB2 files, art and RED receipts: canonical `data/diagnostics/questpoi-2026-10-10/`.

## See Also

- [[minimap]] — map mask, tile sampling and giver status lifecycle.
- [[world-map]] — world XY to UiMap transform.
- [[quest-ui]] — authoritative quest snapshots and tracker.
