# Adventure Guide

Retail Mainline Blizzard_EncounterJournal defines structure and behavior. Modern and Forever display the same controls; Forever changes art only.

## Required behavior

- Dungeons/Raids: expansion/tier selector, authored instance buttons/backgrounds, instance overview and boss list.
- Encounter: existing 3D model renderer; Overview and expandable Abilities from JournalEncounterSection, role/difficulty filters. Collapse on one page must not hide sections on another page whose parent is not rendered there. JournalEncounterXDifficulty restricts boss visibility and choices.
- Loot from JournalEncounterItem with difficulty/class/spec and Retail slot categories (robes share Chest; weapon variants map to MainHand/OffHand; other drops have an Other category).
- Item Sets: Retail compact named rows, item-level ordering (then case-insensitive name and descending set ID), authored item icons, class/spec filtering with a specific spec selected for a class. Pinned ItemSet membership and the shared Retail item-level scaling tables are authoritative; rows and item tooltips use the same squished level. Mixed-level sets display their highest effective member level. Entering Item Sets normalizes an unspecified spec, using the active player spec for that class or its first authored spec. Search filters sets directly. Tutorials uses the authored Catch Up Experience text/art, not invented journal help. Show its unavailable realm service explicitly until RPE is backed.
- Search across encounters, sections and instances; micro-menu/keybind and boss-frame openers. First-open input remains queued while its real spell-text dependency loads, with loading feedback rather than an error/raw-text fallback. Native wheel, stepper and thumb events must scroll instance, ability, loot, set and search lists. Opening during ENCOUNTER_START selects that encounter without automatically opening a window during combat.
- Dungeons owned-keystone browsing consumes ChallengeSnapshot.keys, independent of active runs. Join challenge IDs through authentic world.db MapChallengeMode to Journal maps; selecting the dungeon shows its owned level and authored timer. Unknown challenge IDs remain a data gap, never name-guessed. Importer requires `--world` and opens that database read-only.
- Display authoritative per-instance lockouts from raid-entry InstanceInfo, with expiry relative to the snapshot. Journeys/Traveler's Log only with authentic local data and server-backed state; missing backing is a documented gap, never invented progress.

## Sources

Pinned local CASC Retail 12.1.0.69933. `scripts/adventure_guide_import.py` imports layout-validated Journal tables and ItemSet into `data/cache/adventure_guide.sqlite`. WoWDBDefs files supplied with `--definitions` must contain each exact pinned layout. Metadata retains readable/unreadable record counts. Difficulty choices come from the instance's MapDifficulty rows, including legacy raids. Boss displays must pass the existing authored appearance importer and authenticated model-chain preparation; renderer coverage checks stay strict. Unreadable records never create synthetic replacements. Data is not committed in this repository. Item/spec compatibility uses ItemSpecOverride or ItemSpec type/stat matches (TrinityCore a352b1fa ObjectMgr::ItemSpecStats/LoadItemTemplates); presentation strings come from pinned ChrClasses and GlobalStrings. Journal bodies reuse the existing spell-token renderer without manufacturing missing numeric values.

Reference behavior: local `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_EncounterJournal/Mainline/Blizzard_EncounterJournal.lua`, `.xml`, `Blizzard_LootJournal.lua` and `Blizzard_LootJournalItems.lua`.

## Acceptance

Targeted importer and UI/layout tests, extension check and format proof. Inspected both-skin PNGs showing dungeon abilities, loot and a raid on private UDP5890 with fb_* accounts; GPU free memory at least 1.5 GiB. No UDP5000 access or full suites. Gaps and exact proof scope recorded in the task handoff.

## October 11 native checkpoint

- Both skins, privateUDP5890/fb_adventure: real micro-menu and J open/close; Ozruk overview/abilities/expand-collapse/role selection; difficulty2 six drops→Timewalking24 eight; Paladin Ret70 three→Neck one; textured Ozruk36475 and Lucifron13031 models; tier68 Molten Core741/Lucifron1519 at authored difficulty9; Item Sets wheel changes visible rows; authored Tutorials and Lucifron search result selection.
- `godot/tests/adventure_guide_scroll.gd` passes in Modern and Forever: first-open readiness plus model→Classic raid list→five wheel notches moves Molten CoreY518.67→418.67 and selects Lucifron9. The pending-close path is asserted when catalog loading is observed.
- 32 inspected1280×720 PNGs under `/syncthing/AgentShared/2026-10-11/adventure-guide/`; logs/state/CPU ledger in canonical `data/diagnostics/adventure-guide-2026-10-11/`. Cleanup verifies owned processes terminated and privateUDP cleared.
- Not certified: nonempty authoritative lockouts or boss-frame/ENCOUNTER_START live links, all1889 boss payloads, complete Retail Item Sets presentation. These have source wiring/bounded coverage, not blanket live proof. Monthly/Journeys/Suggested Content and RPE start lack realm backing. The final master merge adds Mythic+ snapshots: active-run dungeon overview now renders the shared authoritative tracker summary (level/time/forces/deaths/bosses, absent rating stays absent), with targeted source-data tests; this journal integration has no new native active-run proof. Owned-keystone inventory browsing remains missing; gem relic compatibility lacks GemProperties. Shared spell renderer numeric/token limits remain. No invented service state or asset substitution.
