# Battle pets: data and companion journal

The account journal supports companion collection and bounded wild PvE battles. Phase1 acceptance lives in the [client contract](../../specs/battle-pets.md) and [server contract](../../../../game-server/docs/specs/battle-pets.md); [wild acceptance](../../specs/wild-pet-battles.md#acceptance--2026-10-10) owns battle proof and exclusions.

## Local data
`export_db2_csv.py` pins each observed WDC5 layout. BattlePetSpecies has inline ID at field2. BattlePetEffectProperties has six inline relative string offsets in one 192-bit field: each offset's base is its own 32-bit element, not the array start. Unreadable Species/Creature records are excluded and counted. Key-flagged sections already decrypted by local CASC are readable and must not be rejected solely by that flag.

`export_pet_catalog.py` joins readable species to local Creature names/display arrays. Unresolved names/displays remain empty/zero; no invented creatures. Unnamed species remain in the catalog but cannot create a blank journal selection. Server runtime reads shipped `world.db` battle-pet tables; native icon/input metadata reads exported BattlePetSpecies CSV. [Server contract](../../../../game-server/docs/specs/battle-pets.md) owns the producer boundary.

## Runtime
Server stores PetJournal under authenticated account ID in `account_pet_journals`, separate from character collections and hunter/warlock stable. Existing CollectionStateUpdate carries optional account journal and active instance; mount-only updates leave the journal untouched. Instance summon requests use u64 identity, not species ID.

ItemEffect learn-spell records for critter summons select the species grant instead of executing a combat spell. Summoning creates an owned replicated NPC with ModelDisplay, UnitSummonedBy and existing FollowOwner movement. Non-combat flags prevent attacks; dismissal/owner disappearance remove it. Player ownership resolves from real PlayerZone/map state, not the NPC-only Zone component. Passive companions are excluded from the combat pet frame.

## Native UI
Pets and Toy Box use `collections_component::collections_shell`: the same centered 703×606 CollectionsJournal, portrait FDID454046 and six-tab bar in both skins. Only Pets/Toy Box are enabled in Phase1; they switch mutually exclusive content hosts. The stray disabled PetJournalPetsTab was a hand-built duplicate, not a Retail frame, and is removed. Selected species display/family come from the same local battle-pets.json as the server. The card requests a detached creature visual through WorldScene and reuses the character sheet's own-world model viewport; changing selection/closing cancels pending work and frees the previous model. Preview camera bounds include nested creature scale/rotation. Summon/Dismiss moves to the right-panel footer. UI decisions live in PetJournalView; account network dispatch feeds authoritative updates. RegistryUi owns rendering/input and existing creature rendering handles the follower. Find Battle uses the experimental [PvP queue contract](../../specs/petbattle-pvp.md); wild PvE and PvP share the three ordered combat-loadout slots.

## Rendered acceptance — 2026-10-10
Private loopback5591, account fb_pets: item4401 learned Mechanical Squirrel; Petson and Petstwo share the persisted journal. Both 703×606 journal skins and an extracted-model companion were captured under `/syncthing/AgentShared/2026-10-10/pets/`. Scripted movement moved the owner about14m and the follower about9m; dismiss removed the replicated mesh. Diagnostics `pets-2026-10-10/follower-motion-proof.json` preserves both positions.

Strict appearance import accepts `--pet-catalog` display roots; additive SQLite merge preserves10792 existing displays and adds2422 companion displays. No raw-model fallback. This proves the Mechanical Squirrel path, not visual coverage of every species. Independent verification remains unavailable because Claude OAuth expired.

## Collections integration correction — 2026-10-10
The v2 screenshots replace the standalone-window visual acceptance: shared shell/portrait, species39/display7937 in the card, footer Dismiss after authoritative summon and clearly framed4-yard follower in both skins. Pets↔Toy Box and shared close are live-proved with one visible CollectionsJournal. The Pets content frame must ignore background pointer events so it cannot intercept the shell's close button; actual pointer RED/ GREEN is preserved in `pets-v2-2026-10-10/modern-close-interception-red.json` and `modern-close-after-toy-green.json`.

Rebased master requires product/build receipts for player equipment and companions. Focused frozen Retail69933 local-archive publication adds the Mechanical Squirrel chain and this private character's starter-plate/shield assets:21 authenticated assets, no legacy relabeling or data commits. Unrelated missing NPC receipts remain outside this acceptance. Capture layout explicitly enables the existing micro menu; both presets hide it by default. [Contract](../../specs/battle-pets.md#acceptance--2026-10-10) owns final proof scope.

## Wild PvE integration — verified 2026-10-10
The [wild client contract](../../specs/wild-pet-battles.md) and [server contract](../../../../game-server/docs/specs/wild-pet-battles.md) own capability and acceptance. Native right-click routes catalog wild CreatureIDs before ordinary NPC attack/interaction; the server checks authority and owns decisions/rewards. CollectionChannel carries Start/State/Round/End plus loadout/actions. The terminal state precedes End; otherwise a lethal round left the previous HP on screen.

`WildBattleHud.hidden_layers` stores each ordinary CanvasLayer's InstanceId and prior visibility. A final frame step hides competing world HUD/nameplates while the battle view exists; close restores surviving layers. The battle layer, game menu and debug FPS layer remain separate. The own-world two-model viewport reuses character-preview asset loading/camera; its +X camera means ally+Z renders left, enemy−Z right.

Private pointer proof uses explicit account-scoped Modern/Forever layouts and native `account_state().ui_skin`. New accounts default to Forever on current master: an unset layout is not Modern proof. Both skins now have inspected win/trap/capture/journal images, visible health/actions, two pet meshes and close restoration. The named spec records exact revisions, native assertions and eight960×540 PNG hashes. Unsupported inherited effect semantics, all-species assets/spawns and full Retail visual/combat parity remain outside this bounded proof.

## Retail HUD correction — verified 2026-10-10
The [v2 HUD contract](../../specs/wild-pet-battles.md#retail-hud-v2--2026-10-10) owns acceptance. Active/reserve portraits, quality-tinted Retail frame art, icon actions/locks/cooldowns/ability-family badges and XP chrome replace bare bars and ability-name labels. `BattleCombatFeedback` transports actual event target/amounts, not inferred HP deltas; a pointer-transparent Dialog layer paints floating numbers above the detached model viewport and expires them after two seconds. The original text painted beneath the viewport and partially hid ally damage; both-skin captures now prove legible amounts. Mainline Lua hides timer art for wild PvE and centers its Pass button; timed snapshots alone show the round countdown. Terminal rewards include the new XP threshold so level-ups redraw progress correctly.

## PvP queue and initial selection — bounded proof, 2026-10-10

`PetBattleQueueRequest`/`PetBattleQueueUpdate` carry Join/Leave and proposal-specific Accept/Decline over CollectionChannel. The server matches exact sorted level profiles, requires both acceptances, then collects both decisions before invoking the existing engine. Faint replacements automatically pass the living opponent. Per-owner snapshots reverse teams/active indices and feedback targets; PvP uses `wild_creature=0` and disables trapping. Terminal State precedes each owner's End; forfeit/disconnect releases both participants. `PvpBattle.initial_pets` holds both choices until both arrive, then updates active indices without engine stepping or round cost; `initial_selection_required` automatically opens the existing picker and blocks combat/pass. The current active pet is a valid initial choice. No PvP XP is currently granted. The [contract](../../specs/petbattle-pvp.md) owns unresolved numeric countdown/reward policy and native acceptance.

Mainline's320×200 PetBattleQueueReadyFrame can render without the journal open. Its dialog border must be registered in the Pet Journal's own RegistryUi; otherwise projection stops at `Unconverted native frame decoration: PetBattleQueueBorder`, leaving only a dark background. Registration and explicit Dialog-strata children now have [two-account both-skin live proof](../../specs/petbattle-pvp.md#acceptance--2026-10-10-petbattle-pvp2). An XDG config directory alone cannot isolate auth: the token path is under the runtime project's sibling `data/`. The continuation gives each client a separate runtime project/data/config tree, sharing asset links but not token files; account associations survive restart. The file-driven fixture also requires atomic command publication: truncate/write can expose incomplete JSON and stop its coroutine. Atomic rename fixes the writer instead of hiding parse failures.

## Battlefield model availability — verified 2026-10-10

HUD3 omitted three ally asset receipts: Soul of the Aspects display40019 →574802.m2, Tickbird Hatchling display62217 →1100485.m2, Winter Reindeer display15904 →125583.m2. Rabbit display328 →1112735.m2 already had an authenticated receipt and rendered in the ordinary world. The detached battle scene required both arrivals before attaching either; consuming an ally error stranded the pair, hiding the ready Rabbit too. Native Soul/Rabbit RED had0 meshes; independent publication GREEN had1 Rabbit while Soul remained receipt-gated. Failed completions now report team/display context once and no longer block the other side.

Local `casc-local` recovered22 authenticated chain assets from actual Retail69933/build`dcfc90fffd79ba00406ae46f5f657592`. The no-clobber importer published20 new receipt mappings and5 companion aliases in `cache/model-asset-index-petbattle-models.json`. Existing base-index SHA256/inode and asset bytes stayed unchanged. The [product-isolation contract](../../specs/product-isolated-model-assets.md) owns additive fragment validation/duplicate rejection. No raw-model fallback or metadata relabelling was introduced.

Rest-pose mesh bounds also clipped Soul's upper body after receipt recovery. The preview camera now includes authored bounds for every loop variation of the active animation, transformed through the nested display scale, and fits nearest-face depth. Keeping all same-ID variation bounds avoids zoom changes/clipping when Soul chooses another idle sequence. Display scales remain the authentic catalog values: Soul0.35, Tickbird0.25, Reindeer0.3, Squirrel1.3 and Rabbit1.0; no placement or scale override was needed.

Engine runtime `fbba0e908`, server origin/master `3d78754`, shared source archive`6691ae1`: private UDP5607, account fb_pbmodels, Modelproof30. Eight8-second native tests observe visible nonempty, fully projected animation bounds for Soul/Tickbird/Mechanical Squirrel/Winter Reindeer against Rabbit in both asserted skins. Eight individually inspected1280×720 PNGs live at `/syncthing/AgentShared/2026-10-10/petbattle-models/`. Extracted-only final clients emit no pet-model errors and never initialize CASC. Durable proof, immutable staged bytes and private fixture seed: `data/diagnostics/petbattle-models-2026-10-10/`. Journal seeding is model/HUD proof, not acquisition proof. Full Retail visual/combat parity and all-species availability remain unclaimed.

## Sources
- [PvP contract](../../specs/petbattle-pvp.md), [journal model](../../../godot/ui-model/src/pet_journal.rs), [battle view](../../../godot/ui-model/src/wild_pet_battle.rs).
- [Receipt index](../../../godot/core/src/model_asset_index.rs), [authenticated publisher](../../../scripts/import_model_asset_chains.py), [native visibility regression](../../../scripts/tests/test_petbattle_model_visibility.py), [preview bounds](../../../godot/rust/src/character_frame/preview.rs).
- [Wild native host](../../../godot/rust/src/wild_pet_battle.rs), [private pointer fixture](../../../godot/tests/pet_battle_live.gd) and [wild spec/acceptance](../../specs/wild-pet-battles.md).
- [Client spec](../../specs/battle-pets.md)
- [Exporter](../../../scripts/export_db2_csv.py)
- [Catalog join](../../../scripts/export_pet_catalog.py)
- [UI model](../../../godot/ui-model/src/pet_journal.rs)
- Local Mainline AddOns/Blizzard_Collections/{Mainline/Blizzard_Collections.xml,Shared/Blizzard_PetCollection.xml,.lua}

## See Also
- [[godot-conversion]] — native screen hosting
- [[shipped-assets]] — local asset policy
