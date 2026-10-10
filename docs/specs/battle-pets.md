# Battle pets — Phase 1 client

## Contract
- Collections micro button opens the Pet Journal tab. Retail Mainline defines geometry and elements; Forever changes art only.
- Journal lists server-owned instances and uncollected catalog species, with names, icons, level, quality and breed. Search filters names; selecting an uncollected species cannot summon it.
- Summon/dismiss sends the owned instance GUID through existing CollectionChannel requests. Wait for server authority; show refusals and prevent duplicate pending requests.
- Companion is an ordinary replicated non-combat creature, rendered through the existing CreatureID/display/model path and moved by authoritative owner-follow positions.
- Escape closes the journal; disconnect clears account journal state.

## Scope
Retail CollectionsJournal 703×606, 260px left list, upper-right card, three disabled battle slots and disabled Find Battle. Battles are not implemented in this phase. No Forever-only widgets or behavior.

## Data
Maintained local exporter `scripts/export_db2_csv.py` supports the BattlePet tables and Creature. `scripts/export_pet_catalog.py <csv-dir> <output.json>` joins species to local Creature names/displays for server runtime. Exported assets are uncommitted data, never downloaded from Wago/Wowhead.

## Verification
UI-model owned/unowned/search/pending/action tests in both skins, protocol journal/64-bit request round trips, and private-server native captures of both journals plus follower. Captures/proof status recorded in the Phase 1 handoff; code alone does not establish visual acceptance.
