# Battle pets — Phase 1 client

## Contract
- Collections micro button opens the Pets tab of the existing CollectionsJournal shell. Pets and Toy Box share the same centered window, portrait (FDID454046), title and six-tab bar; switching tabs never opens a second journal. Retail Mainline defines geometry and elements; Forever changes art only.
- Journal lists server-owned instances and uncollected catalog species, with names, icons, level, quality and breed. Search filters names; selecting an uncollected species cannot summon it.
- Selected pet card shows the species' extracted creature display model, name, level/quality/breed and Retail type icon. Summon/Dismiss sits at the bottom of the right panel; Find Battle remains disabled. No standalone Pet Journal tab button below the shell.
- Summon/dismiss sends the owned instance GUID through existing CollectionChannel requests. Wait for server authority; show refusals and prevent duplicate pending requests.
- Companion is an ordinary replicated non-combat creature, rendered through the existing CreatureID/display/model path and moved by authoritative owner-follow positions.
- Escape closes the journal; disconnect clears account journal state.

## Scope
Retail CollectionsJournal 703×606, 260px left list, upper-right card, three disabled battle slots and disabled Find Battle. Battles are not implemented in this phase. No Forever-only widgets or behavior.

## Data
Maintained local exporter `scripts/export_db2_csv.py` supports the BattlePet tables and Creature. `scripts/export_pet_catalog.py <csv-dir> <output.json>` joins species to local Creature names/displays for server runtime. Exported assets are uncommitted data, never downloaded from Wago/Wowhead.

## Verification
UI-model owned/unowned/search/pending/action tests in both skins, protocol journal/64-bit request round trips, and private-server native captures of both journals plus follower. Captures/proof status recorded in the Phase 1 handoff; code alone does not establish visual acceptance.

### Acceptance — 2026-10-10
Private5591 native capture proves both journal skins, item4401 ownership on two characters, summon/dismiss and a rendered Mechanical Squirrel following authoritative movement. PNGs: `/syncthing/AgentShared/2026-10-10/pets/`; motion coordinates: `data/diagnostics/pets-2026-10-10/follower-motion-proof.json`. Targeted journal UI4, protocol19, server follower/account5 and item-learning1 pass. Independent gate unavailable (expired Claude OAuth); no battle or all-species visual parity claim.
