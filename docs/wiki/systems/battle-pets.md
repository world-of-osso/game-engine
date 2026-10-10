# Battle pets: data and companion journal

Phase 1 connects the existing account journal domain to collection messages and native Pet Journal; battles remain a separate phase. Acceptance lives in the [client contract](../../specs/battle-pets.md) and [server contract](../../../../game-server/docs/specs/battle-pets.md).

## Local data
`export_db2_csv.py` pins each observed WDC5 layout. BattlePetSpecies has inline ID at field2. BattlePetEffectProperties has six inline relative string offsets in one 192-bit field: each offset's base is its own 32-bit element, not the array start. Species/Creature encrypted sections are excluded and counted.

`export_pet_catalog.py` joins readable species to local Creature names/display arrays. Unresolved names/displays remain empty/zero; no invented creatures. Server reads `data/battle-pets.json`; native icon lookup reads the exported BattlePetSpecies CSV.

## Runtime
Server stores PetJournal under authenticated account ID in `account_pet_journals`, separate from character collections and hunter/warlock stable. Existing CollectionStateUpdate carries optional account journal and active instance; mount-only updates leave the journal untouched. Instance summon requests use u64 identity, not species ID.

ItemEffect learn-spell records for critter summons select the species grant instead of executing a combat spell. Summoning creates an owned replicated NPC with ModelDisplay, UnitSummonedBy and existing FollowOwner movement. Non-combat flags prevent attacks; dismissal/owner disappearance remove it.

## Native UI
Collections micro button opens the same 703×606 Retail layout in both skins. UI decisions live in PetJournalView; account network dispatch feeds authoritative updates. RegistryUi owns rendering/input and existing creature rendering handles the follower. Find Battle and the three battle slots remain disabled.

## Sources
- [Client spec](../../specs/battle-pets.md)
- [Exporter](../../../scripts/export_db2_csv.py)
- [Catalog join](../../../scripts/export_pet_catalog.py)
- [UI model](../../../godot/ui-model/src/pet_journal.rs)
- Local Mainline AddOns/Blizzard_Collections/{Mainline/Blizzard_Collections.xml,Shared/Blizzard_PetCollection.xml,.lua}

## See Also
- [[godot-conversion]] — native screen hosting
- [[shipped-assets]] — local asset policy
