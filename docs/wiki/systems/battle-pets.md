# Battle pets: data and companion journal

Phase 1 connects the existing account journal domain to collection messages and native Pet Journal; battles remain a separate phase. Acceptance lives in the [client contract](../../specs/battle-pets.md) and [server contract](../../../../game-server/docs/specs/battle-pets.md).

## Local data
`export_db2_csv.py` pins each observed WDC5 layout. BattlePetSpecies has inline ID at field2. BattlePetEffectProperties has six inline relative string offsets in one 192-bit field: each offset's base is its own 32-bit element, not the array start. Unreadable Species/Creature records are excluded and counted. Key-flagged sections already decrypted by local CASC are readable and must not be rejected solely by that flag.

`export_pet_catalog.py` joins readable species to local Creature names/display arrays. Unresolved names/displays remain empty/zero; no invented creatures. Unnamed species remain in the catalog but cannot create a blank journal selection. Server reads `data/battle-pets.json`; native icon lookup reads the exported BattlePetSpecies CSV.

## Runtime
Server stores PetJournal under authenticated account ID in `account_pet_journals`, separate from character collections and hunter/warlock stable. Existing CollectionStateUpdate carries optional account journal and active instance; mount-only updates leave the journal untouched. Instance summon requests use u64 identity, not species ID.

ItemEffect learn-spell records for critter summons select the species grant instead of executing a combat spell. Summoning creates an owned replicated NPC with ModelDisplay, UnitSummonedBy and existing FollowOwner movement. Non-combat flags prevent attacks; dismissal/owner disappearance remove it. Player ownership resolves from real PlayerZone/map state, not the NPC-only Zone component. Passive companions are excluded from the combat pet frame.

## Native UI
Pets and Toy Box use `collections_component::collections_shell`: the same centered 703×606 CollectionsJournal, portrait FDID454046 and six-tab bar in both skins. Only Pets/Toy Box are enabled in Phase1; they switch mutually exclusive content hosts. The stray disabled PetJournalPetsTab was a hand-built duplicate, not a Retail frame, and is removed. Selected species display/family come from the same local battle-pets.json as the server. The card requests a detached creature visual through WorldScene and reuses the character sheet's own-world model viewport; changing selection/closing cancels pending work and frees the previous model. Preview camera bounds include nested creature scale/rotation. Summon/Dismiss moves to the right-panel footer. UI decisions live in PetJournalView; account network dispatch feeds authoritative updates. RegistryUi owns rendering/input and existing creature rendering handles the follower. Find Battle and the three battle slots remain disabled.

## Rendered acceptance — 2026-10-10
Private loopback5591, account fb_pets: item4401 learned Mechanical Squirrel; Petson and Petstwo share the persisted journal. Both 703×606 journal skins and an extracted-model companion were captured under `/syncthing/AgentShared/2026-10-10/pets/`. Scripted movement moved the owner about14m and the follower about9m; dismiss removed the replicated mesh. Diagnostics `pets-2026-10-10/follower-motion-proof.json` preserves both positions.

Strict appearance import accepts `--pet-catalog` display roots; additive SQLite merge preserves10792 existing displays and adds2422 companion displays. No raw-model fallback. This proves the Mechanical Squirrel path, not visual coverage of every species. Independent verification remains unavailable because Claude OAuth expired.

## Collections integration correction — 2026-10-10
The v2 screenshots replace the standalone-window visual acceptance: shared shell/portrait, species39/display7937 in the card, footer Dismiss after authoritative summon and clearly framed4-yard follower in both skins. Pets↔Toy Box and shared close are live-proved with one visible CollectionsJournal. The Pets content frame must ignore background pointer events so it cannot intercept the shell's close button; actual pointer RED/ GREEN is preserved in `pets-v2-2026-10-10/modern-close-interception-red.json` and `modern-close-after-toy-green.json`.

Rebased master requires product/build receipts for player equipment and companions. Focused frozen Retail69933 local-archive publication adds the Mechanical Squirrel chain and this private character's starter-plate/shield assets:21 authenticated assets, no legacy relabeling or data commits. Unrelated missing NPC receipts remain outside this acceptance. Capture layout explicitly enables the existing micro menu; both presets hide it by default. [Contract](../../specs/battle-pets.md#acceptance--2026-10-10) owns final proof scope.

## Sources
- [Client spec](../../specs/battle-pets.md)
- [Exporter](../../../scripts/export_db2_csv.py)
- [Catalog join](../../../scripts/export_pet_catalog.py)
- [UI model](../../../godot/ui-model/src/pet_journal.rs)
- Local Mainline AddOns/Blizzard_Collections/{Mainline/Blizzard_Collections.xml,Shared/Blizzard_PetCollection.xml,.lua}

## See Also
- [[godot-conversion]] — native screen hosting
- [[shipped-assets]] — local asset policy
