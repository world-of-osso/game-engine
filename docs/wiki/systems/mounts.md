# Mounts, steady flight and skyriding (Godot)

Player mounts, Retail "Steady Flight" and Skyriding flight physics on the Godot client. Server rules (mount aura, MountCapability, flight speed, can-fly): game-server `docs/wiki/systems/mounts.md`.

## Mount model

`godot/rust/src/world_mount.rs`. A player's replicated `Mounted::mount_display_id` loads that creature display as a model under the unit's node; the unit's own visual is reparented onto the mount's MountMain point (M2 attachment 0, `NpcModel/Skeleton3D/AttachmentBone0/Attachment0`), so it follows the saddle bone. `seat_rider` runs every frame, so a re-dressed rider remounts; a dismount seats the visual back on the unit node and frees the mount. Creature (NPC) mounts from `creature_addon` are not drawn yet.

Animation: the rider holds Mount (91). The mount plays the rider's locomotion clip on the ground; while flying (`PlayerMotion::FLYING` for other players, `PlayerMovement::flying` for the local one) its flight clips: MountFlightIdle 548, MountFlightRun 556, MountFlightBackwards 550, MountFlightLeft 552, MountFlightRight 554 (resolved through `AnimationData.Fallback`).

## Mount Journal

[Collections source layout](collections-ui.md) owns shared chrome/menu composition and the2026-10-14 both-skin native fidelity proof.

Verified2026-10-12; acceptance and remaining gaps are owned by the [contract](../../specs/mount-journal.md). Native `mount_journal.rs` receives account catalog snapshots, drives the shared Collections shell, and requests detached real creature visuals into the existing character-preview Scene. Drag rotates the independent model; native row right-clicks use the existing frame-click routing. The server owns learning, favourites, summon/random/dismount; the client never invents ownership.

`MountJournalEntry` carries Mount IDs, source spells/text, display/icon IDs, authored visibility flags and journal categories. Ground/flying/aquatic derive from terrain flags and positive swim-speed auras; skyriding from FlightCapability; Ride Along from the existing enter/exit non-control seat join. These are data-derived categories, not a claim that unpublished C_MountJournal classification code was reproduced. Source labels decode color/newline escapes and express money textures as g/s/c; no raw texture tags appear in the native label.

`import_mount_appearance.py` selects all1,915 local MountXDisplay roots and invokes the strict authored importer with a read-only base. Current import adds1,782 coverage rows,23 profiles,441 choices and268 geosets, preserving every15,039 prior coverage row and every prior profile/choice/geoset. Local69933 CASC tables1264997/3692043/1720141 decoded with no skipped sections. Cache backup, row-preservation/hash proof and reproducible arguments are in `data/diagnostics/mount-journal-2026-10-11/`. No world.db writes.

Rebuild into a new output, never directly overwrite a cache:

```text
python3 scripts/import_mount_appearance.py --mount-catalog data/db2/12.1.0.69933/MountXDisplay.csv --db2-dir data/diagnostics/mount-journal-2026-10-11/import-db2 --data-dir data/diagnostics/mount-journal-2026-10-11/import-data --model-cache data/cache/creature_display.sqlite --base-cache data/cache/npc_appearance.sqlite --output data/diagnostics/mount-journal-2026-10-11/npc_appearance-rebuilt.sqlite
```

Feedback continuation2026-10-11: the original SharedContext/Screen test passes repeated active/favourite snapshots, but real UDP vendor buy→learn→favourite stalled: server logged `(collected=true,favourite=true,active=false)` while the client retained favourite=false for30seconds and received no corresponding update. Repeated full1,694-row publications put immutable descriptions/source text into every ordered reliable response. Initial world entry now sends a complete catalog; subsequent responses merge changed authoritative rows into the retained view before normal SharedContext generation/Screen sync. The boundary resets when character selection replaces the player entity. `mount_journal_snapshot_changes_rebuild_shared_screen_without_reopening` asserts retained catalog, selected mount, favourite art and Mount/Dismount output through successive compact updates (RED dropped the other catalog row).

Already-known feedback uses authentic server teaching-item links and account ownership, not character usability; same-ID Forever items are distinct. Final private UDP5901 proof (2026-10-12, merged revision, fresh redb and read-only world.db copy): in both skins paid Katie Hunter item5655→Mount18 learn, vendor Already known, favourite, rotate, summon/dismount and random favourite; alternate-character persistence; and two-account AH Miscellaneous→Mount listing, buyout, auction-won mail, UseItem of unbound item44413→Mount275 and AH Already known. Logs/receipts: `data/diagnostics/mount-journal-2026-10-12/`; PNGs: `/syncthing/AgentShared/2026-10-12/mount-journal/`.

The preview loads the mount's real creature display only through verified [product-isolated receipts](../../specs/product-isolated-model-assets.md). A worktree whose de-linked `data/cache` predates the canonical `npc-appearance-retail` fragment lacks receipts for models such as Chopper234727 and logs `No verified asset receipt`; canonical data has it.

### Mount asset closure staging — 2026-10-12

`scripts/stage_mount_model_assets.py` traverses every pinned Retail69933 MountXDisplay alternative through CreatureDisplayInfo/CreatureModelData, display replacement textures and authored Extra/customization/item resources. Local CASC config, encoding and root content hashes authenticate both prior extracted bytes and newly extracted payloads. No CDN, inferred ownership or zero-filled payload is accepted.

The candidate package at `data/diagnostics/mount-model-assets-2026-10-12/cache-package/` increases model-receipt coverage509→1,971 Mount IDs (120→686 model FDIDs), complete asset-chain coverage322→1,971 and all-alternative coverage320→1,971. Native-format fragment:10,050 new receipts, including566 M2s;2,119 companion aliases. Five PFID physics satellites are authenticated audit-only files because native previews do not consume them and the runtime receipt format rejects `phys`. No missing payloads or unresolved graph edges remain. Canonical creature/profile caches already cover these displays; a fresh strict import preserves exactly the same rows, so publication does not replace SQLite caches.

`apply_cache_package.py` validates by default, checks the frozen canonical receipt-input set and every destination before hash, and publishes only with `--apply`; the lead owns publication.12,170 artifact paths add3,968,862,232 bytes (3.6963GiB). Package staging does not certify a release bundle or mutate canonical model/cache files.

Native census uses `godot/tests/mount_asset_preview.gd`, the landed Mount Journal and a matching privateUDP5910 server, with the slot's product/cache overlay pointing at the staged package and `extracted-only` runtime mode. Both-skin native census rendered1,444 unique recovered mount identities per skin:1,292 whose primary M2 receipt was missing plus152 whose existing M2 receipt lacked dependencies. Every target has visible meshes and at least20 opaque4-pixel-grid samples in a saved native preview. Squeakers1290 and Sapphire Skyblazer1456 needed a native quarter-turn/90-frame settle to pass that pixel gate; their default-angle silhouettes remain small, so framing quality is not claimed fixed.170 additional newly missing primary-model identities have authored MountXDisplay/assets but no readable Mount row/name/spell, so journal selection is unresolved, not fabricated. The local Mount.db2 probe found none of those170 IDs among readable or encrypted-record IDs. There are277 absent Mount rows overall, including97 whose model receipts predate this package.

Fixture boundary: the normal Mount Journal host does not route scroll input to its list controller. The fixture reparents that same live RegistryUi under the existing UiAuditProbe to forward native thumb-drag input and reach virtualized duplicate-name rows; model loading/rendering remains the real journal path. Remove that fixture routing when the normal host wires `scroll_list_input`. This is not normal-host scrolling acceptance. Rapid reconnect initially reproduced the server's stale-player panic; remaining dependency/rotation batches used rebuilt server master e68ef97f after the lead's relog fix, with no further panic observed. No server code was changed by this slice. Nine inspected ≤1280px PNGs are shared at `/syncthing/AgentShared/2026-10-12/mount-model-assets/`.

Sources: package `_proof/{source-identity,census-before,census-after,asset-graph,unresolvable-journal-mounts,native-preview-summary}.json`, `manifest.json`, and the staging/publication scripts. [Journal requirements](../../specs/mount-journal.md) and [no-install closure](offline-asset-closure.md) retain the broader acceptance boundary.

## Passenger mounts

Contract and acceptance: [passenger mounts](../../specs/passenger-mounts.md). `Mounted` carries VehicleID and eight indexed occupancy slots; `VehiclePassenger` carries the driver's server entity bits, seat index and VehicleSeat row. The server validates group membership, living/range and `InMap { map_id, instance_id }` conditions and enter/exit non-control seat flags, controls passenger movement and ejects invalid ownership. Passengers follow driver-relative M2 bind-pose positions; the server does not animate mount bones. Production players have `InMap`, not the NPC `Zone` component; boarding fixtures use `auth_character::spawn_player_entity` so they exercise that boundary.

VehicleSeat.AttachmentID is a seat enum, not an M2 lookup ID: 13 maps to39, 14 to40 (SolariTyClient native A2D3F0, `crates/systems/src/vehicle/vehicle_passenger_c.rs`). `godot/core/src/vehicle_seat.rs` maps it; `world_mount.rs` places the passenger visual at the animated attachment with authored offset/rotation and RideAnimLoop. The visual remains owned by its own unit node, so freeing a driver's mount cannot destroy another player's visual. The normal mount-summoning rider still uses MountMain0; Vehicle seat index0 can be a passenger seat.

Model points must include `WorldModels::build_creature_visual`'s NpcVisualRoot -90degree alignment and display scale before driver facing/translation. The real-M2 server test exposed the missing alignment: seat2764 at driver(10,2,20) was(8.721561,5.1512804,19.206053), not the rendered seat(10.793948,5.1512804,18.721561). Server offsets now include that model frame and the authored offset together.

The party-frame/target-frame menu exposes Ride for a replicated group member's vehicle mount. `MainMenuBarVehicleLeaveButton` is the same32px control in both skins, using local-CASC Retail exit art237700/237699 and sending ExitVehicle through `spells/assignment.rs::spell_cursor_press`. The action bar enables cursor inputs, so its clicks bypass `poll_action_bar_clicks`; consuming `vehicle_exit` as a spell/item slot previously swallowed exit. Sources: cached Retail `Blizzard_ActionBar/Shared/VehicleLeaveButton.{xml,lua}`, `Blizzard_OverrideActionBar/OverrideActionBar.lua`; current cached UnitPopup has no Ride entry, so menu placement is the explicit user requirement, not claimed upstream Lua parity.

Readable local data links14 mount/display rows to Vehicles312/313/315 (2 passengers),318/774 (1 passenger). Traveler's Tundra Mammoths280/284 use312/313; Chopper275 uses318. Yak62809 and Drake50269 are absent from the local creature-template link source; their VehicleIDs remain unknown rather than guessed. Proof status and live gaps are owned by the linked contract.

## Flight controls

`godot/rust/src/gameplay.rs`, after Retail Bindings_Standard.xml:53-65: `JUMP` = `JumpOrAscendStart`, `SITORSTAND` = `SitStandOrDescendStart`.

- The client may fly while its replicated `PlayerMotion` has `CAN_FLY` (`set_can_fly`); losing it ends a flight and the player falls.
- Take off: Space jumps; with Space still held in the air the player starts flying.
- Flying: no gravity; held Space ascends and X descends at `FLIGHT_SPEED` × the server's aura modifier; with the right mouse button (MOVEANDSTEER) forward movement follows the camera pitch, as swimming does; total velocity is capped at the flight speed (the server grants the 3D distance).
- Landing: touching the ground (not ascending) clears `flying`; reaching swimming depth turns into a swim. X does not sit while flying.
- `PlayerInput.flying` reports the flight; a vertical-only climb or descent reports like a swimmer's.
- Using the mount spell while riding it sends `CancelMountAura` (dismount). `use_spell(id)` (`#[func]`) casts through that same path for fixtures.

## Annotated tree contact

The flying movement path now consumes manually annotated tree contact with a fixed prototype sphere, tangential deflection and per-placement damped branches. The bounded tree prototype has separate native contact and mounted-input proof. See [[elastic-trees]] for authoring and limits.

## See Also

- [[elastic-trees]] — manual annotations, native flight contact and bounded proof.
- [[collision-system]] — terrain, WMO and camera collision boundaries.

## Skyriding (part 1: physics)

Model and constants: shared-protocol `src/skyriding.rs` (`Glider`, FlightCapability 11 from Retail DB2 12.1.0.69933). Server bound and the test-character switch (`game-server-admin flight-style <name> skyriding`): game-server mounts page.

- The client may skyride while its replicated `PlayerMotion` has `CAN_ADV_FLY` (`set_can_adv_fly`, from the Skyriding aura 406095); losing it ends the skyride and the player falls.
- Take off as for steady flight (Space jumps, Space held in the air), which is Retail's Skyward Ascent 372610 (JUMP, JUMP: Blizzard_Tutorials_RPE.lua:391-395): the client sends that cast once (`PlayerMovement::take_takeoff_request`) and keeps falling; the server spends a Skyriding Charge and its `SpellGo` (`skyriding_spell`) launches the mount straight up at 31.5 yd/s on top of its run and rises without lift (about 24 yards), then glides once falling with 7.5 yards of air under it.
- Gliding: no thrust; lift turns the velocity toward the facing and the pitch, gravity trades height for speed (dive faster, climb slower), air friction 1.5 yd/s², gravity stops at 65 yd/s. With the right mouse button the pitch follows the camera pitch (at most 180°/s); without it the pitch stays.
- Landing: coming down onto terrain or a WMO floor clears `flying`; swimming depth ends it into a swim. Every frame of a skyride reports (`PlayerInput.flying`); the server sets `FLYING | ADV_FLYING`.
- Flight style: Switch Flight Style 436854 (5 s cast) swaps Steady and Skyriding on the server, saved per character; the next mount flies the new style (game-server `docs/wiki/systems/mounts.md`).
- Not yet: banking, surface friction, the old-world 85% speed.

## Skyriding (part 2: vigor and abilities)

Vigor is the Skyriding Charges the server keeps (game-server mounts page: ChargeCategory 2391, 6 charges, 10.35 s each while on the Skyriding aura). The client casts the abilities through the normal spell path (`use_spell`; part 3 puts them on the override bar) and flaps only on the server's `SpellGo` for the local player (`combat_visuals.rs` → `PlayerMovement::skyriding_spell` → shared `Glider::cast`), so a refused cast (no charge, cooldown) never moves it:
- Surge Forward 372608 and Whirling Surge 361584: +31.5 yd/s along the facing and pitch; Skyward Ascent 372610: +31.5 yd/s up. The speed an impulse reaches is capped at FlightCapability 11 `AddImpulseMaxSpeed` 100; above `MaxVel` 65 the mount loses `OverMaxDeceleration` 7 yd/s² more. The impulse sizes are assumptions in one block of shared-protocol `src/skyriding.rs` (the effects are DUMMY server scripts), listed in game-server `docs/specs/skyriding.md`.
- Aerial Halt 403092: air friction × 100 (its MOD_ADV_FLYING_AIR_FRICTION 10000%) for 0.5 s, which stops a mount at up to 75 yd/s; its reduced gravity (4 s, amount unpublished) is not modelled.
- Not yet: Whirling Surge's spiral visual.

## Skyriding (part 3: bar and vigor)

- Skyriding bar: while an ADV_FLYING aura is up (Skyriding 406095) `GetBonusBarOffset` is 5, so the main bar shows bonus bar 5, action slots 120..131 (`player_spells::bonus_bar_offset`; Retail's Skyriding tutorial finds Surge Forward past `(NUM_ACTIONBAR_PAGES + GetBonusBarOffset() - 1) * 12`, Blizzard_Tutorials_RPE.lua:446). The server fills those slots when the rider gains ADV_FLYING (game-server mounts page). Dismounting shows page 1 again. Action slots are 0..179 (`MAX_ACTION_BUTTONS`).
- Vigor: `SpellChargesUpdate` carries its `ChargeCategory`; the client keeps charges per category and recovers them locally between updates. `vigor.rs` shows the Retail `FillUpFrames` widget (`dragonriding_vigor`, atlas FDID 4730866: 42×45 frames, first and last padded -20, decor wings 8 lower) for Skyriding Charges 2391 above the main bar while the Skyriding bar is up; unreported charges count as full.
- Assumed: the `dragonriding_vigor` texture kit (Retail also has `dragonriding_sgvigor` themes; the widget's kit is server data we do not have) and the bar order Surge Forward, Skyward Ascent, Aerial Halt, Whirling Surge, Second Wind. Not modelled: the widget's flash/flipbook animations and the spark mask.

## Proof

- Live `godot/tests/skyriding_bar_live.gd` (2026-10-03, private server UDP 5196, game-server skyride3 `62cbfca`, character Fbskybar with 34090, 32235, 403092, 361584, 425782 and `flight-style skyriding`): page 1 → mounted bar `[372608, 372610, 403092, 361584, 425782]` with vigor 6/6, key 1 casts Surge Forward → 5/6 with the sixth filling, dismount → page 1, no vigor. `data/diagnostics/skyride3-2026-10-03/` (run 1 failed: the transport did not receive `SpellChargesUpdate`).

- Depot unit tests (`gameplay::tests`): takeoff/climb at flight speed, hover without gravity, X descent and landing input, mouse-steered pitch, fall after losing `CAN_FLY`, Space without `CAN_FLY` only jumps.
- Live `godot/tests/flying_mount_live.gd` (2026-10-02, private server UDP 5179, game-server `4ac0e91`, character Fbflymount with Expert Riding 34090 and Golden Gryphon 32235 via `game-server-admin learn-spell`): mounts (display 17697, rider on attachment 0), climbs 30 yd in 1.66 s, flies forward, hovers 2 s with no height change, server height within one frame of the client, lands at the terrain height (server speed back to mounted run 14), dismounts. Screenshots and traces: `data/diagnostics/flymount-2026-10-02/`. The client needs ≥ 4 fps: the server applies at most 0.25 s of movement per input; an earlier run at 2 fps (loaded host) left the server half a flight behind.
- Skyriding Depot unit tests (`gameplay::tests`): launch apex 23.9 yd with no lift, glide, mouse-steered dive past 25 yd/s and landing; losing `CAN_ADV_FLY` falls; RED with the launch disabled.
- Live `godot/tests/skyriding_live.gd` (2026-10-03, private server UDP 5185, game-server skyride1 `006b750`, character Fbskyride set on the peak at WoW (-8793, 213, 382) with 34090, 32235 and `flight-style skyriding`): launch 24.2 yd, dive 405 → 239 yd at up to ~64 yd/s, pull-up 63.7 → 46.8 yd/s while climbing 25 yd, level glide, landing on the terrain with the server at the same spot. Server off the client's path at most 0.66 yd over 89 samples at ≥ 10 fps (2.09 yd during a 1-2 fps host stall, where the server's catch-up runs straight to the newest report); replication lag about 0.2 s. `data/diagnostics/skyride1-2026-10-02/` (run 6 final; earlier runs: low launch, WMO landing the old test took for a hang, a 1 fps stall that left the server behind and killed the character by falling after the client landed).
- Skyriding abilities Depot unit test (`gameplay::tests::skyriding_abilities_flap_the_glider`, RED without `skyriding_spell`): Surge Forward adds 31.5 yd/s along the facing, the next frame moves 25+ yd/s faster, Aerial Halt stops the horizontal motion within 0.4 s, nothing flaps on the ground.
- Live `godot/tests/skyriding_abilities_live.gd` (2026-10-03, private server UDP 5194, game-server skyride2 `7e9c2b6`, shared-protocol `8065440`, character Fbskyridetwo on the Northshire peak with 34090, 32235, 403092, 425782 and `flight-style skyriding`): launch, two Surge Forwards through the spell pipeline (mean speed 43.4 → 70.9 and 64.9 → 93.2 yd/s, server on the client's path within 0.00 yd at about 0.1 s lag), landing mounted at (-8793, 145, 515), six more ground casts (two charges had recovered in flight), "No charges remain.", a charge back 7.0 s later (10.35 s after the previous one). `data/diagnostics/skyride2-2026-10-03/` (run 3 final; run 1 never landed within 40 s, run 2 landed in the lake and swam off the mount).
- Live `godot/tests/skyriding_style_live.gd` (2026-10-03, private server UDP 5196, game-server skyride4 `0673f3a`, character Fbskyfour with 34090, 32235, 376777, 436854, entering on Steady): the gryphon flew steady (no Skyriding bar, vigor hidden); Switch Flight Style 436854 then a remount paged the Skyriding bar with 6/6 charges; Space, Space took off through Skyward Ascent, 6 → 5 charges, rising 24.7 yd. Evidence `data/diagnostics/skyride4-2026-10-03/`.


## MountSpecial continuation — 2026-10-15

[Contract](../../specs/mount-special.md): stationary grounded mounted jump and `/mountspecial` share one emote action; moving jump remains ordinary movement. `Mounted.mount_id` supplies authored identity independently of shared display IDs. The server authenticates the actor, requires world entry/mounting, and sends the same event to the owner and its actual NetworkVisibility observers, without social text or an invented achievement ID. Corrected HostClient proof captures real messages in First: Lightyear clears receivers then delivers local messages in Last, so post-update sampling missed delivery. Login achievement state must settle before asserting no additional emote credit. Wire2, slash1, core6 and server3 proofs pass.

`mount_special_kit.rs` joins Mount, every relevant AnimKitSegment, Config/BoneSet and numeric Priority, plus visual-kit AnimKit references. `animation/mount_special.rs` layers scheduled samples over the seated rider using key-bone subtrees and preserves the mount sequence against stationary locomotion updates. Spelltome dependencies/reverse and concurrent DND masks have deterministic timeline tests; condition meanings remain explicitly inferred, not unavailable-client parity. Generic spell first-segment animation resolution is not used for mount rider kits. Supported model/sound effects use the existing visual pipeline.

**Open native timing boundary:** installed Grrloc AnimStartTime2000 exceeds HumanMale FishingCast133's actual1000ms duration. Treating it as a clipped clip offset suppresses the rider completely: native pose test observed zero upper-body delta despite162 keyed tracks. Lead03:52 approved wrapping only held/self-loop clocks; oversized one-shot offsets are explicit errors/named gaps, not silently successful playback. Thus Grrloc19345/19705 remain unaccepted on this model. WebWowViewerCpp animationManager.cpp:924-926 supports modulo progression, not native AnimKit initial-offset semantics. Native mount override and actual late Spelltome rider poses pass. Three-mount/both-skin actor/observer fixture uses Spelltome1532 as the complete four-segment rider case; prepared, not executed. Procedural10/34/17, Rope4 and other unsupported visual discriminants remain named gaps.

Sources: `godot/core/src/mount_special{,_kit}.rs`, `godot/rust/src/animation/mount_special{,_tests}.rs`, `world_mount.rs`, `world.rs`, `godot/tests/mount_special_live.gd`; server `messaging_mount_special_tests.rs`; source references in the contract.
