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

## Effect HUD scopes — verified 2026-10-10

The [effect HUD contract](../../specs/wild-pet-battles.md#effect-hud-state) adds a single weather snapshot and two team-pad aura lists, independent of each pet's aura list. Snapshot projection excludes weather/team objects from pet rows; PvP reverses pad lists along with teams. This fixes both missing pad/weather displays and their old duplication on the original caster. Aura name, icon, remaining turns and beneficial/hostile direction are server-owned; admitted beneficial effects attach to the source team. The HUD groups buffs/debuffs using Mainline's three-icon pet rows and two-icon pad rows, red hostile borders and blank indefinite durations.

`effect_hud.rs` shares all geometry between skins. Weather backgrounds use the local listfile's authored Retail weather textures. Round messages append into a dedicated temporary Pet Battle log rather than replacing the preceding round; ability/aura names replace numeric aura IDs. Retail's max(cooldown, lockdown) governs numbered overlays. Turn-lock/continuation state disables abilities without fabricating a lockdown duration; nonzero lockdown handlers remain gated server-side.

Private UDP5598 native pointers cast real Sunlight404, Cyclone190 and Tranquility254 against a map1 Rabbit. Both configured skins show weather8→6→4, enemy Cyclone4→2 across ally swaps, friendly Tranquility1, damage/miss/heal floats, cooldown5, retained chat and HUD restoration. Six inspected1280×720 PNGs live at `/syncthing/AgentShared/2026-10-10/petbattle-hud3/`; diagnostics `petbattle-hud3-2026-10-10` retain source-state/control snapshots and pointer ledger. The private redb fixture seeded authentic species347/194/118 at level2 for fb_pbhud3; this proves HUD transport, not pet acquisition. Native detached models are not proved in this run: Soul of the Aspects FDID574802 lacks a verified asset receipt. UI/icons remain visible; prior two-mesh acceptance is separate. No world.db writes, UDP5000, public deployment or master merge.

## Sources
- [Effect HUD projection](../../../godot/ui-model/src/wild_pet_battle/effect_hud.rs), local Retail Blizzard_PetBattleUI Shared XML725–913/Lua1593–1705 and Mainline ChatFrameBase's PET_BATTLE_COMBAT_LOG temporary window.
- [PvP contract](../../specs/petbattle-pvp.md), [journal model](../../../godot/ui-model/src/pet_journal.rs), [battle view](../../../godot/ui-model/src/wild_pet_battle.rs).
- [Wild native host](../../../godot/rust/src/wild_pet_battle.rs), [private pointer fixture](../../../godot/tests/pet_battle_live.gd) and [wild spec/acceptance](../../specs/wild-pet-battles.md).
- [Client spec](../../specs/battle-pets.md)
- [Exporter](../../../scripts/export_db2_csv.py)
- [Catalog join](../../../scripts/export_pet_catalog.py)
- [UI model](../../../godot/ui-model/src/pet_journal.rs)
- Local Mainline AddOns/Blizzard_Collections/{Mainline/Blizzard_Collections.xml,Shared/Blizzard_PetCollection.xml,.lua}

## See Also
- [[godot-conversion]] — native screen hosting
- [[shipped-assets]] — local asset policy
