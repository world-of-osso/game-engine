# Native player housing

Retail Mainline controls/behavior are shared by Modern and Forever; Forever changes art. Build-qualified static data, account ownership and actual item acquisition remain separate. Requirements/gaps: [client contract](../../specs/housing.md), [server contract](../../../../game-server/docs/specs/housing-catalog.md).

## Authority and copies

`HousingHud` retains a qualified catalog and independent `HousingAuthority`. Dashboard opening queries reliable HousingRequest/Reply; browsing never grants or enters property. Unknown storage stays unknown until a matching-build snapshot; confirmed absence means zero. Owned counts include stored plus placed copies across the account's houses. Logout/world reset clears authority.

Server requests bind to authenticated account/character/player links. Neighborhood copies key on authored map/property; interior copies key on owned house ID. Admission reserves identity before the normal world-port handshake, with no fabricated MapDifficulty, dungeon locks or reset schedule. First transfer/login populates authenticated neighborhood objects once. Dedicated housing population survives generic startup's deferred GameObjectSpawns replacement.

Plot purchase requires the exact source cornerstone in the player's copy and existing five-yard model-box reach validation. Payment and ownership are one transaction. Home Interior2783 admission uses NeighborhoodMap7's real entry/25-tile ADT floor, without claiming a granted assembled house. Persisted visited-house identity survives restart/relog and rechecks permission. Leave/selected plot visits return to authored NeighborhoodPlot.TeleportPosition. Plot visits never pay, own or grant decor.

## HouseSettings access

Verified:2026-10-12. `HouseSettings` keeps a separate draft; opening, switching modes and toggling relationships never changes confirmed authority. Source600×550 panel stages Anyone/No One/Limited with Neighbors/Guild/Friends/Party; explicit Save sends only house flags and closes, while close discards. Unknown remains unconfigured. Plot/blueprint/owner policy stays unavailable and untouched. Both skins share the same source controls, with art-only Forever differences.

Server `persistence_housing_access` derives neighbors from durable paid same-neighborhood ownership, friends from the assigned owner's persisted character list and real visitor name, guild from persisted nonzero guild IDs, party from accepted current GroupRegistry membership. Pending invites and the visitor's unilateral friend list do not authorize entry. Battle.net friend authority is unavailable. Admission, layout reads and restored visited-house identity recheck current permissions. Schema2 migrates previous unknown/closed/open settings once without changing gold/plots/storage/layouts; [server contract](../../../../game-server/docs/specs/housing-catalog.md#housesettings-persistencesource-boundary) owns the migration contract.

Twelve private native role/mode/skin runs prove staged/closed drafts, saved/relogged Anyone/No One/Limited-Friends, allowed read-only visits and No One denial remaining outside after real reauthentication. Each account owns an actually paid house; no item/decor/property grant. Evidence `data/diagnostics/housing-core-2026-10-12/core8/`; eight inspected public PNGs show both-skin saved modes and denied visitors. Allowed-visitor relog captures lagged registry projection and are not published as visual proof; their authoritative state/readonly/relog proof is retained. Guild/Neighbors/current-party relationship changes have concrete server tests, not new per-relationship live UI proof.

## Catalog and actual acquisition

Pinned Retail12.1.0.69933 / dcfc90fffd79ba00406ae46f5f657592 catalogs preserve literal DB2 fields and unreadable/missing records. Native categories/subcategories, search, Collected/Uncollected/Dyeable filters, total/stored/placed counts and source tooltips share both skins. Vendor names come from real server creature queries; encounter names/instances/difficulty come from authenticated source joins. Source evidence47 tables/70,552 rows/54 unreadable records: `data/diagnostics/housing-core-2026-10-12/qualified-sources`.

The collector resolves each same-build HouseDecor.ItemID → on-use ItemEffect → exact effect324 decor target, with zero player condition/base points. It consumes an owned Retail bag item; it does not require a vendor listing or manufacture quests/recipes. Normal merchant costs/conditions still govern buying. Proven paid examples: bookcase245259/decor1693/500,000 copper and Tesla Coil246487/decor2330/750,000 copper. Both skins retain exact quantities after reauthentication. No item/decor was granted.

Decor-reward quests preserve TDB objective14 as CriteriaTree. Supported unconditional one-leaf CompleteAll4/AcquireItem36 trees accumulate actual successful vendor delivery; missing/conditioned trees block completion rather than using empty legacy objectives. Furniture Favor92572's root216882/leaf216883/criterion109153/item253168 → reward253173/decor9247 has CPU source/credit proof, not live completion. Its vendor source references unresolved252826; no alias/stock is invented. Recipe1239278/1239321 lacks SkillLineAbility, and GnomishFence's collector has target0. See the server contract for these acquisition boundaries.

## Catalog model preview

`housing_preview` mounts the actual decor actor in the existing isolated transparent model-scene viewport, not in the gameplay world. Search/selection replaces the actor; tab/search-empty/close/logout clear it before registry parent teardown. Tile thumbnails stay tile art and no longer substitute for 3D preview. Missing models/textures produce Preview unavailable and a source-specific diagnostic, never a stale actor. Native drag yaw is presentation-only. Retail's collision-bounds preference excludes emitter/animation envelopes from camera fitting; UiModelScene presets, zoom controls and dyed variants remain separate capabilities.

Live Modern preview exposed a tab-transition root cause: desired Catalog tab was set before its search control mounted, and `frame_text` returned empty for the absent control. Sampling only mounted search controls retains the actual filter instead of resetting selection to an unrelated entry. First current private run passed both skins for bookcase1693/M21096761, Tesla2330/M26699745 and pillar525/WMO6429362, physical rotation, filter retention, cross-type replacement and unchanged authority. Image inspection found WMO525's lower body dark: rendered-cage measurement3,717 of11,501 solid pixels above0.02RGB (32.3%). The lighting hypothesis was falsified: binding interior_ambient0.55 produced identical pixels and both batch materials exposed the applied values. Both use pixel20/MOMT23; the reference WebWowViewerCpp1a8cccbe `commonWMOMaterial.slang:279-280` mixes diffuse toward a black AO TODO by MOC2 alpha. The ineffective light change was reverted. Existing literal reference behavior is retained, not forced-unlit/recolored. Geometry/material rendering is proved; full Retail pixel20 AO appearance remains source-blocked. The former60%brightness threshold was an unsupported native acceptance assumption, not a Retail contract. Updated final both-skin proof remains pending. Fixtures: `godot/tests/housing_preview_scene.gd`, `housing_preview_wmo_scene.gd`, `housing_preview_wmo_pixels.gd` and `housing_preview_live.gd`.

### Preview asset publication boundary

The maintained closure walker had confused MOMT22/23 with pixel19/20: it omitted actual parallax/DF auxiliary textures and misclassified unused words of MOMT19/20 as FDIDs. The renderer and importer already use the correct mapping. Concrete binary fixtures reproduce both omitted and invented dependencies; the walker now uses MOMT22→3extra and23→6extra words. Actual housing pillar6429362 needs auxiliary BLP5984930. The importer and runtime receipt index previously rejected WMO kind despite the renderer loading those files. Both now accept authenticated .wmo/REVM records under the same build/product namespace, without M2 relabeling. Publication uses frozen build-key/content-key receipts and the existing no-clobber importer; canonical cache mutation remains lead-only.

## Retail neighborhood discovery

Finder954×490 draws its698×465 MapCanvas from the standard UiMap tile/projection catalog. Parent-approved dedicated housing interpretation uses NeighborhoodMap.MapID → UiMapAssignment, not names:2735/assignment68589→System2 UiMap2401/art2056;2736/68590→2402/art2057. Missing/ambiguous relations report unavailable; there is no normal-world-map fallback. These projection regions are NOT legal outdoor edit bounds.

Sale28px and friend20px pins follow Retail HouseFinderMapDataProvider; Self/Stranger pins are omitted. Friend status comes only from the requester's persisted friends list and the real owner character name. The synthetic occupied-house visit list is removed. Source plot selection renders one PlotInfo price/index/Visit control; friend visiting still requires explicit owner access. The real-UDP friend fixture uses ordinary AddFriend and then verifies the housing relationship, never an admin friendship grant.

Authored GameObjects UI links with PlayerInteractionType72 open the600×400 bulletin board. It shows durable residents alphabetically, online white/offline disabled, and repeatable Name/Status/Plot sorting. Both-skin physical board/relog proof uses actual object576398.

Charter/create remains explicitly unavailable: pinned TC a352 HousingHandler/Packets handles decline-invites only; TDB1210 recheck found no fee/signature/name-length authority. Generated NeighborhoodNameMaxCharacters0 is not inferred as a policy.

## Basic/expert editing and observers

Only a confirmed owner inside the house edits. Place/store conserves owned quantity; preview/cancel does not debit. Saved visuals resolve the actual decor model/texture receipts. Basic ground placement/move/yaw/remove and both-skin reauthentication are proved on the genuinely purchased1693 bookcase/M2 1096761. Visitors see the same layout, cannot edit, and never receive owner private storage.

Expert Translate/Rotate/Scale uses staged full transforms. Axis selection cycles X→Y→Z; commit persists once, cancel restores confirmed rendering, and WMO scaling is refused. Both skins hide storage in expert mode and share64px source mode/submode controls. Native bindings/rates are input tuning, not certified Retail defaults:1/2/3 submode, arrows/PageUp/PageDown translation, Q/E increments, X next axis, Enter commit, Esc cancel. Native3D handle identity/Shift snap increments remain source gaps; available unsnapped mouse controls are proved below.

Expert mouse continuation uses source TransformManipulator rotate80×20 arrows and scale200×19 art/thumb/percentage display; native selected-decor axis dragging and relative scale mapping are explicit input tuning. Start/Change/Complete stage press-relative transforms; release is not commit. Retail XML names Shift rotation/scale snapping but exposes no increment; HouseDecor has only InitialRotation/InitialScale, and GameParameter has5unnamed records. Snap increments and native3D handle identities remain unavailable, with no replacement art/numeric interpretation. Private5870 proof passes in both skins: actual selected M2 XYZ drags, source rotation-arrow holds on all3axes, relative scale-thumb drag, release outside the widget stops manipulation, preview leaves authority unchanged, cancel restores rendering, Enter commits with stored0/placed1, and real same-process reauthentication retains the complete transform. Expert instructions use the source RIGHT/x−30/y0 anchor; live RED reproduced minimap overlap before both-skin GREEN. Evidence: `data/diagnostics/housing-core-2026-10-12/expert-mouse/{modern,forever}/final/`.

Schema2 retains position/yaw/pitch/roll/uniform scale and explicit house-access selections. One-time schema0 migration keeps saved position/yaw and sources missing axes/scale from that decor's authenticated DB2 InitialRotation/InitialScale. Source degrees/Z-up convert to engine radians/Y-up. Missing source aborts atomically; current schema has no legacy runtime reader. Old-layout → migration → reopen → precision edit → reopen/conservation and repeat-migration preservation are tested.

Actual two-client proof compares unsolicited layouts AND rendered nodes for position, all three angles, scale and real mesh presence after owner move/rotate/store/re-place. Observer sends nothing after admission. Both directions/skins pass; both houses retain one paid placed bookcase afterward.

## Root causes and fixture boundaries

- Relog crash: old transport retained PlayerLink/InWorld after its player retired; mount-journal next-frame dereference panicked. Reproduced on plain master, separately pushed `relog-crash` de908bfd. Central retirement removes old world links before despawn while retaining transport/auth for disconnect delivery. Both-skin real reauthentication now survives.
- Editor keys: returning from `_input` without marking the viewport handled let Esc close all windows and Enter open chat. Native editor now consumes handled events; close clears staged editor state. RED cancellation-lost-controls → both-skin retained-controls GREEN.
- Source cornerstone beam's box centre is empty mesh space. Physical picking projects visible mesh triangles and proves native UnitPicker identity, without relaxing collision/picking.
- Map pin centres can be covered by another source pin. Fixture proves an exposed portion through Godot hover dispatch before physical clicking; it does not resize/substitute pins.
- Same-map different-house transfers cannot use map2783 alone as readiness. Fixture waits the new HUD generation and registry projection before querying the confirmed owner layout; final assertion remains actual house ID/editability.

## Proof and assets

Verified:2026-10-12. Private UDP5870 only, real fb_* accounts/characters, no canonical world.db mutation or UDP5000 operation. Both-skin inspected1280px PNGs are under `/syncthing/AgentShared/2026-10-12/housing/`: catalog, editor/visitor, bulletin, dedicated Finder, retained expert controls and live observers. Receipts/logs stay under `data/diagnostics/housing-core-2026-10-12/` and `/home/osso/.worktrees/housing-core-evidence/`; final revision/gates are recorded in the handoff.

Authenticated local-CASC maps2735/2736/2783 close17,227 assets with zero missing payloads; real WDT roots210/225/25 parse. Private LOS bakes contain21,100,583/28,571,544/1,638,400 triangles. Map bytes do not grant house components. Existing receipt-backed Pawdo appearance/hat and actual decor assets are not substituted. Promotion commands/qualification live in the server contract and [asset ownership](../../specs/product-isolated-model-assets.md).

## Remaining source/parity boundaries

No verified first-house exterior/interior component grant, legal outdoor bounds, charter policy or assembled room/blueprint/progression authority is invented. Razorwind's55 referenced cornerstone objects are absent even though its plots/map art are browsable. Generic known-item collection does not manufacture missing recipe, quest, retroactive reward or market-cart authority. Raw interior terrain is not a furnished assembled house.

## Sources

- [Housing contract](../../specs/housing.md), [server/import contract](../../../../game-server/docs/specs/housing-catalog.md).
- [Native host](../../../godot/rust/src/housing.rs), [editor input](../../../godot/rust/src/housing_editor.rs), [mouse input](../../../godot/rust/src/housing_mouse.rs), [source mouse widgets](../../../godot/ui-model/src/housing_mouse_component.rs), [decor visuals](../../../godot/rust/src/housing_decor.rs).
- [Catalog/authority model](../../../godot/ui-model/src/housing.rs), [HouseSettings draft](../../../godot/ui-model/src/housing_settings.rs), [HouseSettings controls](../../../godot/ui-model/src/housing_settings_component.rs), [Finder component](../../../godot/ui-model/src/housing_finder_component.rs), [shared map projection](../../../godot/ui-model/src/ui_map_data.rs).
- Local Retail Blizzard_HousingDashboard, HouseFinder, BulletinBoard, HouseEditor and generated Housing API/constants; pinned TC a352 CriteriaHandler/DBCEnums and HousingHandler/Packets.
- Local authenticated DB2/DBD receipts, pinned WoWDBDefs07d008f28907c00827cacadf2d7c1a40f578b6a3.

## See Also

- [[instances]] — property-copy isolation and world-port handshakes.
- [[game-objects]] — source display/model resolution and model-box interaction reach.
