# Adventure Guide

Retail Mainline Blizzard_EncounterJournal defines structure and behavior. Modern and Forever display the same controls; Forever changes art only.

## Required behavior

- Dungeons/Raids: expansion/tier selector, authored instance buttons/backgrounds, instance overview and boss list.
- Encounter: existing 3D model renderer; Overview and expandable Abilities from JournalEncounterSection, role/difficulty filters. Collapse on one page must not hide sections on another page whose parent is not rendered there. JournalEncounterXDifficulty restricts boss visibility and choices.
- Loot from JournalEncounterItem with difficulty/class/spec and Retail slot categories (robes share Chest; weapon variants map to MainHand/OffHand; other drops have an Other category).
- Item Sets; Tutorials uses the authored Catch Up Experience text/art, not invented journal help. Show its unavailable realm service explicitly until RPE is backed.
- Search across encounters, sections and instances; micro-menu/keybind and boss-frame openers. Opening during ENCOUNTER_START selects that encounter without automatically opening a window during combat.
- Display authoritative per-instance lockouts from raid-entry InstanceInfo, with expiry relative to the snapshot. Journeys/Traveler's Log only with authentic local data and server-backed state; missing backing is a documented gap, never invented progress.

## Sources

Pinned local CASC Retail 12.1.0.69933. `scripts/adventure_guide_import.py` imports layout-validated Journal tables and ItemSet into `data/cache/adventure_guide.sqlite`. WoWDBDefs files supplied with `--definitions` must contain each exact pinned layout. Metadata retains readable/unreadable record counts. Difficulty choices come from the instance's MapDifficulty rows, including legacy raids. Boss displays must pass the existing authored appearance importer and authenticated model-chain preparation; renderer coverage checks stay strict. Unreadable records never create synthetic replacements. Data is not committed in this repository. Item/spec compatibility uses ItemSpecOverride or ItemSpec type/stat matches (TrinityCore a352b1fa ObjectMgr::ItemSpecStats/LoadItemTemplates); presentation strings come from pinned ChrClasses and GlobalStrings. Journal bodies reuse the existing spell-token renderer without manufacturing missing numeric values.

Reference behavior: local `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_EncounterJournal/Mainline/Blizzard_EncounterJournal.lua`, `.xml`, `Blizzard_LootJournal.lua` and `Blizzard_LootJournalItems.lua`.

## Acceptance

Targeted importer and UI/layout tests, extension check and format proof. Inspected both-skin PNGs showing dungeon abilities, loot and a raid on private UDP5890 with fb_* accounts; GPU free memory at least 1.5 GiB. No UDP5000 access or full suites. Gaps and exact proof scope recorded in the task handoff.
