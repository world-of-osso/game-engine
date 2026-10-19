# Wild pet battles

Wild PvE encounters use server-authoritative snapshots and actions, rendered through `godot/ui-model/src/wild_pet_battle.rs` and the native host. Retail Mainline `Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.xml`, `.lua` and `Blizzard_PetBattleUIPatchwerks.xml` define the controls and layout; Forever changes art only.

## Anomalus framing diagnostic (fixture only)

- [x] Owned private5971 fixture records all indexed posed triangles' projected vertices/extrema in the same frame as active material/alpha/cull/shader state, camera/projection/viewport and viewport-edge crops.
- [x] Preserve original framing guard, authored scales, fixed pads, camera and product behavior. Card/loadout DB2 scenes are not a sourced battle-camera replacement.

This is diagnostic evidence for `PET_BATTLE_ANOMALUS_NATIVE_FRAMING`, not a camera/scale fix or pixel-visibility certificate. Alpha/culling/shader displacement remain distinguishable from geometric projection. Owner `anomalus-framing` uses only its fresh private5971 storage; never petbattle5961 or another owner's redb. Native run requires explicit lead window after matching build receipts. Forever frame7444 on 2026-10-19 confirms actual left-edge clipping: 879 indexed ally triangles, 190 outside (160 in nonzero-material-alpha batches), and 134 alpha-positive pixels on the leftmost viewport column. Rabbit1124 triangles remain inside. Bottom-overflow geometry belongs to an alpha0 batch in this frame; bottom/top/right border pixels are transparent. This classifies one pre-cast frame, not both skins, every animation, or a corrected camera. [Evidence and staging incident](../wiki/systems/battle-pets.md#anomalus-indexed-framing-diagnostic--2026-10-19).

## What it must do
- [x] Right-click a wild battle-pet creature to request a battle with the journal's first three ordered combat slots.
- [x] Journal slots equip distinct owned instances and send an authoritative loadout request.
- [x] Both pets display health, level, family and auras; ability1–3 display cooldown/availability.
- [x] Swap, pass, confirmed forfeit and eligible trap actions carry server battle/round identity.
- [x] Wild PvE uses the untimed Retail timer mode, not an invented timeout.
- [x] The camera frames both active 3D pet models on their corresponding health-frame sides, facing each other; swapping updates models.
- [x] Both active pets retain the ordinary creature render path's CreatureDisplayInfo.CreatureModelScale × CreatureModelData.ModelScale, multiplied by CreatureDisplayInfo.PetInstanceScale for battle instances. Never normalize size or position from model/animation bounds.
- [ ] Both active pets stay on fixed separate left/right battlefield pads, unclipped by a fixed presentational camera independent of the pair's bounds. Battle presentation repeats the base idle; ordinary world idle variations remain unchanged. Fixed pads/idle are implemented, but Anomalus95171 has confirmed left-edge raster clipping; no correction is accepted yet.
- [x] Each ready pet renders independently; a missing asset on one side reports that display's error without suppressing the other pet or retrying the consumed completion.
- [x] End displays outcome/XP/capture and disables further actions. Journal updates remain authoritative.
- [x] Modern and Forever retain identical geometry and controls.
- [x] Ordinary world HUD/nameplates cannot cover the active battle HUD; closing a battle restores each surviving layer's prior visibility.
- [ ] Disconnect/reset restores surviving layers (implemented, not separately live-proved).

## Model staging acceptance — 2026-10-10
Runtime `a0608a009`: Soul, Tickbird, Mechanical Squirrel and Winter Reindeer versus Rabbit pass all eight Modern/Forever native cases. Tests observe both changing posed meshes, fixed pads, unclipped geometry and authentic CSV-derived scale ratios for eight seconds. Eight individually inspected `v3-*.png` captures live in `/syncthing/AgentShared/2026-10-10/petbattle-models/`; durable logs/fixtures/ledger are in `data/diagnostics/petbattle-models3-2026-10-10/`. Importer tests preserve PetInstanceScale0.7 independently of world scale and rebuild an old schema with unchanged CSV mtimes. These four species have PetInstanceScale1; no non-1 live instance or all-species/combat-animation coverage is claimed. [Implementation and historical rejected sizing](../wiki/systems/battle-pets.md#authentic-battlefield-scales--verified-2026-10-10).

## Effect HUD state
- Weather has a centered Retail frame with authored background, icon, name and remaining turns.
- Each team pad and active pet separates buffs/debuffs into Retail aura rows; hostile icons use red borders. Negative duration hides the label. Team-pad state survives pet swaps without duplicating onto pet rows.
- Ability overlays display the greater of cooldown and lockdown; other authoritative unusable states dim the icon without inventing a duration.
- A dedicated temporary Pet Battle combat log retains successive rounds and terminal messages, with history navigation. Floating damage/heal remains event-based, independent of net HP deltas.
- Both skins use the same frames and geometry. Server-gated ability-lockdown handlers remain out of scope; no client simulation fabricates their turns.

## How it works
- [Battle-pet journal and assets](../wiki/systems/battle-pets.md).
- [Server turn catalog](../../../game-server/docs/wiki/systems/pet-battle-engine.md).

## Implementation inventory
- `godot/ui-model/src/wild_pet_battle.rs`: state, action admission and Retail-derived screen.
- `godot/ui-model/src/pet_journal.rs`: three ordered combat-slot controls.
- `godot/rust/src/wild_pet_battle.rs`: inputs, networking, native HUD and two-model camera scene.
- `godot/rust/src/account.rs`, `godot/network/src/lib.rs`: actual network receipt and requests.
- `godot/rust/src/merchant.rs`, `gameplay.rs`: wild right-click routing and modal movement.

## Tests asserting this spec
- `godot/ui-model/tests/wild_pet_battle.rs`: both-skin frames/timer/actions, round identity, cooldowns, swaps, forfeit/end.
- `godot/ui-model/tests/pet_journal.rs`: owned loadout action and duplicate-slot clearing.
- `scripts/tests/test_petbattle_model_visibility.py`: private native regression for a ready Rabbit opposite a receipt-gated ally; complete-pair checks assert both pets' fixed pad positions, authentic CSV-derived scale ratios, changing posed geometry, full posed-animation framing and actual skin over eight seconds.
- `godot/tests/pet_battle_live.gd`: private real-pointer fixture; durable `data/diagnostics/pbwild-20261010/prove.py` asserts actual skin, two models, isolated HUD, defeated 0-HP win, capture and close restoration.

## Known gaps (current cycle)
- [x] Rebased native extension/CLI build (2026-10-10).
- [x] Both-skin private live full win/capture proof with inspected PNGs.

### Acceptance — 2026-10-10
Server `55b823a`, engine runtime `49244b137`, protocol branch recorded in the handoff: private UDP5592, `fb_pbwild_proof`/Pbwildproof character31. Explicit saved Modern/Forever layouts are confirmed by native `account_state().ui_skin`, not inferred from the default (now Forever). Three real item4401 learns equip instances7/8/9. Each skin defeats Rabbit species378/creature61080 and separately captures it (instances10/11); server rewards persist, and the second client process retains the first capture/XP. Both defeated ends show0/147 HP. Trap-ready30/147 and49/147 are strictly below35%. Normal HUD is absent during battle and restored after close.31 static native battle rectangles match; health fill widths and the newly unlocked ability2 icon are data differences, not skin geometry.

Eight final `modern|forever-wild-{win,trap-ready,capture,journal}.png` at `/syncthing/AgentShared/2026-10-10/petbattle/` were individually ffmpeg-downscaled1280×720→960×540 and visually inspected. `published-inspection.json` records hashes; `isolated-*.json`, `native-skin-geometry-proof.json`, `server-hudproof.log` and the pointer ledger retain process evidence.26 filtered UI integration tests pass; required full server workspace passes1,753 tests,71 ignored, one requested skip. Earlier captures and initial implicit-skin runs are superseded, not acceptance. Independent verifier authentication remains unavailable.

## Out of scope
PvP matchmaking and turn effects not admitted by the inherited server engine. No unsupported-effect direct-damage substitute is allowed.

## Retail HUD v2 — 2026-10-10
Both skins share Retail Mainline PetBattleFrame geometry: active portraits/quality/level/family/HP/aura rows and available reserve pets; icon-only ability actions with hotkeys, cooldowns, locks and ability-family effectiveness badges. Bottom chrome carries swap/trap/forfeit/pass and the active pet XP bar. Wild PvE hides the round timer per PetBattleFrame_UpdatePassButtonAndTimer; timed snapshots show it. Server event amounts float over their target pets for two seconds; Rust Debug output and ability-name labels never appear. Snapshot metadata owns portrait, quality, XP, ability family and aura icons; no combat simulation moves into the client.
Sources: local Retail Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.xml, .lua and Blizzard_PetBattleUIPatchwerks.xml. Acceptance: engine Rust b70e27ad8, protocol1808cdd and server12a7103; targeted UI6/6, wire2/2 and server6/6 pass, native extension/CLI/server builds and extension check pass without compiler warnings. Fresh private UDP5592 database, fb_pbwild_v2/Pbwildtwo: battle in progress, ability damage floating above both pet meshes, strict trap-ready HP and defeated0HP win in each explicitly asserted skin. Eight `v2-modern|forever-wild-{battle,ability,trap-ready,win}.png` at `/syncthing/AgentShared/2026-10-10/petbattle/` are individually ffmpeg-downscaled960×540 and visually inspected. Durable proof: `data/diagnostics/pbwild-v2-20261010/`; no public deployment/merge. Independent verifier authentication expired before it ran.
