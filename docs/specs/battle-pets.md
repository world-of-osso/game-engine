# Battle pets — Phase 1 client

## Contract
- Collections micro button opens the Pets tab of the existing CollectionsJournal shell. Pets and Toy Box share the same centered window, portrait (FDID454046), title and six-tab bar; switching tabs never opens a second journal. Retail Mainline defines geometry and elements; Forever changes art only.
- Journal lists server-owned instances and uncollected catalog species, with names, icons, level, quality and breed. Search filters names; selecting an uncollected species cannot summon it.
- Each populated CompanionLoadOutTemplate slot shows its owned pet's extracted creature display model in the XML ModelScene host, independently of card selection. Moving/removing pets clears old models; closing cancels pending loads and frees previews before the journal UI. Health and ability content appears only from owner-only server combat snapshots. Health remains injured between battles until healed/revived; chosen abilities persist and later PvE/PvP battles consume them. No invented heal/revive timers or costs.
- Pet card/loadout cameras and actor scale/origin come from BattlePetSpecies CardUIModelSceneID/LoadoutUIModelSceneID and matching UIModelSceneCamera/Actor/ActorDisplay data, not mesh-fit or hand-tuned size. XML defines each model viewport.
- CompanionLoadOutTemplate uses 399×106 slots with its authored shadow, pet-type emblem, owned-quality icon border and a dead overlay only for authoritative health ≤0. Ordinary names use the source 28px bottom-aligned area; custom names use 12px plus the species subtitle. PetCard BG remains 405×166. No favorite/unlock state is invented when the protocol does not carry it.
- Selected pet card shows the species' extracted creature display model, name, level/quality/breed and Retail type icon. Summon/Dismiss sits at the bottom of the right panel; Find Battle follows the [PvP queue contract](petbattle-pvp.md). No standalone Pet Journal tab button below the shell.
- Summon/dismiss sends the owned instance GUID through existing CollectionChannel requests. Wait for server authority; show refusals and prevent duplicate pending requests.
- Companion is an ordinary replicated non-combat creature, rendered through the existing CreatureID/display/model path and moved by authoritative owner-follow positions.
- Escape closes the journal; disconnect clears account journal state.

## Authoritative combat details
Owned card/loadout health, maximum health, power, speed, level/XP and next-level XP come from server snapshots. Card lists authored ability options; the loadout uses Retail three ability buttons and a two-choice flyout with level locks. Ability selection sends an owned GUID, one-based slot and authored ability ID; displayed selection changes only after acknowledgement. Refusals preserve authoritative state. Pending loadout/summon requests and queued/in-battle loadouts cannot change abilities; wait for the current owned loadout acknowledgement before opening a selector. Unowned species do not receive fabricated combat values.

## Initial phase scope
Retail CollectionsJournal 703×606, 260px left list, upper-right card, three disabled battle slots and disabled Find Battle. Battles are not implemented in this phase. No Forever-only widgets or behavior.

## Data
Maintained local exporter `scripts/export_db2_csv.py` supports the BattlePet tables and Creature. `scripts/export_pet_catalog.py <csv-dir> <output.json>` joins species to local Creature names/displays for server runtime. Exported assets are uncommitted data, never downloaded from Wago/Wowhead.

`scripts/export_pet_model_scenes.py --db2 <private-local-extraction> --definitions <matching-WoWDBDefs> --build <installed-build> --species data/db2/12.1.0.69933/BattlePetSpecies.csv --output data/battle-pet-model-scenes.json` exports journal-only scene cameras and actor transforms, with byte/schema hashes in a provenance sidecar. Missing scene/actor/display data fails explicitly; no paperdoll-fit fallback. **ModelScene default FOV unsourced**: these tables/XML do not define it. Retain existing engine projection; do not claim projection parity.

## Verification
UI-model owned/unowned/search/pending/action tests in both skins, protocol journal/64-bit request round trips, and private-server native captures of both journals plus follower. Captures/proof status recorded in the Phase 1 handoff; code alone does not establish visual acceptance.

### Acceptance — 2026-10-10
Initial Phase1 evidence remains in `data/diagnostics/pets-2026-10-10/`. The v2 client correction uses rebased private server417ae491 on UDP5591, account fb_petsv2: actual item4401 bag learning, selected species39/display7937 model in the card, Summon→Dismiss authority, Pets↔Toy Box in one shell and shared close in both skins. Four inspected `v2-*.png` at `/syncthing/AgentShared/2026-10-10/pets/` show card and player/follower at1280×720. Squirrel position(-8949,82.6208,128) versus player(-8949,82.6208,132):4-yard follow offset, lateral camera yaw1.57/pitch−0.25/distance4. Pets UI4 and Toy Box9 targeted tests pass; ffmpeg decodes all7 published PNGs. Full v2 commands and runtime revisions live in the handoff and `data/diagnostics/pets-v2-2026-10-10/`. No battle or all-species visual parity claim.
