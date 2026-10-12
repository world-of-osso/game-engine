# Native Adventure Guide

Verified: 2026-10-11. The [contract](../../specs/adventure-guide.md) owns Retail Mainline behavior; Forever only changes the shared screen's art.

## Catalog and native host

`scripts/adventure_guide_import.py` decodes the pinned local69933 Journal WDC5 layouts into `data/cache/adventure_guide.sqlite`, preserving hierarchy, difficulty relations and readable/unreadable counts. Pinned Item/ItemSparse, ChrSpecialization, ChrClasses, GlobalStrings, Difficulty and MapDifficulty CSVs supply metadata. Item compatibility follows TrinityCore a352b1fa `ObjectMgr::ItemSpecStats/LoadItemTemplates`: overrides replace general type/stat matching; no authored specialization means unrestricted. Gem relic traits remain unmodeled without GemProperties.

The core Catalog loads immutable rows; the UI View owns selection, expandable sections, filters and global search. MapDifficulty selects genuine legacy difficulties (Molten Core9), rather than assuming modern raid14; JournalEncounterXDifficulty further restricts boss lists and choices (Vanessa is Heroic-only). Loot menus group robes with Chest and weapon types into Retail MainHand/OffHand categories. A collapsed ability parent remains traversable on Overview when that parent has no visible control there. Item Sets build viewport-intersecting compact atlas rows with authored icons, quality colors, specific-spec filters and shared Retail squished item levels. Their hover reuses the item/Shift comparison tooltip and appends pinned ItemSetSpell thresholds, equipped membership and explicit unsupported eligibility notices. Class/spec changes reset the pixel offset. Tutorials uses Retail's authored RPE Catch Up Experience strings/art, not invented journal help; the realm lacks the start service.

The native host opens from the existing J binding, micro menu and active boss-frame right-click. First-open requests stay queued behind the existing asynchronous spell catalog, with explicit loading chrome and no raw-token fallback. Its pointer handler routes real wheel/stepper/thumb input through RegistryUi; registry enumeration alone does not wire scrolling. All four Journal pixel lists use Retail ScrollFrameTemplate's30-pixel pan extent, not the native default one-row/one-pixel step. Opening during an active ENCOUNTER_START selects that encounter without opening a window automatically in combat. Raid-entry's InstanceInfo supplies saved status, completed-mask count and snapshot-relative reset time. A detached creature preview hosts the selected JournalEncounterCreature display. `adventure_guide_state` exposes read-only displayed selection/filter/model readiness for native automation.

## Realm activities and suggestions

The [realm service](../../../../game-server/crates/server/src/adventure_guide.rs) loads copy-imported pinned AdventureJournal/PerksActivity JSON payloads and routes shared AdventureGuideRequest/AdventureGuideSnapshot on InstanceChannel. The bridge registers the snapshot receiver; Account clears cached snapshots on reset/disconnect. Native opening requests both Guide and saved-instance data. Traveler's Log tags/search/tracked-only and Track/Untrack operate on authoritative replies, with unsupported criterion trees shown as unavailable. Source priorities are signed: unsigned decoding rejected genuine -500 rows and disabled the entire catalog.

The [monthly policy](../../specs/adventure-guide.md#world-of-osso-monthly-policy) owns calendar and source-interval limits. Per-character monthly counters and tracking persist in achievement state; the existing account contribution store aggregates only current-period counters. Live achievement events update monthly counters once per unique criterion; login reconstruction does not grant historical monthly credit. Supported suggestion conditions reuse existing ContentTuning-level semantics; other gates are hidden and counted. Eligible cards remain authored/display-only; unsupported activations are absent and counted as a subset of visible cards. No reward or unsourced activity-month binding is inferred.

Evidence in `data/diagnostics/adventure-guide3-2026-10-11/`: server5/5 (wire query, UTC/leap/reset, tracking persistence, once-only criterion death, account-alt projection, eligible display-only suggestion), UI20/20, final UDP bridge1/1, native builds/checks and all-three format checks pass. Independent filesystem artifact verifier passes bounded existence/substance/wiring; it did not execute commands. Both skins render all945 source activity rows, Track3 and relog persistence; Training Grounds852 uses authored text/texture1523630 without unsupported activation. The pinned suggestion census is698 source/697 unsupported gates/1 visible unsupported activation, not full gate coverage.

Owned keys use ChallengeSnapshot.keys independently of an active run, joined via authentic MapChallengeMode to Journal maps/timers. Opening explicitly queries challenges: an in-session grant had left cached keys stale, reproduced in Forever and fixed with native1→2 GREEN. Both skins show owned Azure8/timer37:30 and first-unopened Cobrahn right-click selects Journal475/Dungeons while fetching persisted Molten Core9/mask1 save. Both skins prove ItemSet filter scroll reset, authored Lightbringer icons and set tooltip membership/bonuses. Source Warmonger items270367–270374 have icon0; the shared spell renderer still lacks some stat-name text. Neither gap gets invented replacement data.

## Journal model preparation

Import every positive JournalEncounterCreature.CreatureDisplayInfoID with the existing `import_npc_appearance.py --display-id ... --base-cache ...` pipeline. Use pinned CreatureDisplayInfo/TextureFileData, the local listfile, authenticated creature-display catalog and local Extra/Option/Geoset WDC5 inputs. Never add `requires_appearance=0` by hand or bypass strict coverage in the renderer.

The October11 import covers all1889 positive Journal display IDs and retains every prior profile, choice, geoset and coverage row. This proves authored appearance coverage, **not** all1889 model/texture payloads. Ozruk36475 and Lucifron13031 chains were extracted from frozen Retail69933 local archives, content-key/SHA256 checked and published in additive `cache/model-asset-index-adventure-guide.json` (22 assets,6 aliases). Existing no-clobber publisher rules apply. Reproduce a new boss's closure from its actual creature-display model and replacement textures; never relabel unqualified bytes.

## Proof and boundaries

Evidence lives in canonical `data/diagnostics/adventure-guide-2026-10-11/`. The rendered `godot/tests/adventure_guide_scroll.gd` fixture exercises first J open and model→Classic raids→wheel→Molten Core/Lucifron selection against an explicit private fb_* session. Initial native mouse harness captures were one frame behind; settle after injected motion/down/up and assert the observed journal state before accepting pixels. The file-command driver also retries the observed truncate/write response boundary; this is test-harness synchronization, not product fallback.

- CPU proof: targeted both-skin Registry tests, authored sword/spec exclusion and Molten Core9 RED→GREEN; explicit spell-reference text rendering RED→GREEN.
- Native Modern and Forever fixtures pass at the scroll-pan checkpoint: first-open readiness and Ozruk model→Classic raid wheel→Molten Core/Lucifron9, Y518.67→418.67. Both private capture flows pass real micro/J open-close, section collapse, role selection, difficulty loot6→8, Ret3→Neck1, textured Ozruk/Lucifron, ItemSet scrolling, Tutorials and search.32 inspected PNGs live under `/syncthing/AgentShared/2026-10-11/adventure-guide/`; exact scope/remaining proof belongs to the [contract](../../specs/adventure-guide.md#october-11-native-checkpoint).
- Initial inspection found no activity/Journeys/Suggested backing. The final master merge adds [Mythic+](mythic-plus.md): the native journal copies the server's active ChallengeRun and renders the existing authoritative tracker summary only on its matching instance overview. That checkpoint lacked owned-key and monthly services, added in the continuation above; journal-active-run native proof and RPE start remain unsupported.
- The reused spell-token renderer retains its documented unavailable scaling/token limits; missing values are not manufactured.

## Sources

- [Catalog](../../../godot/core/src/adventure_guide.rs), [shared screen](../../../godot/ui-model/src/adventure_guide.rs), [native host](../../../godot/rust/src/adventure_guide.rs), [importer](../../../scripts/adventure_guide_import.py).
- [NPC appearance import contract](../../specs/npc-appearance-importer.md), [product-isolated asset contract](../../specs/product-isolated-model-assets.md), [raid-entry](raid-entry.md).
- Local `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_EncounterJournal/Mainline/Blizzard_EncounterJournal.{lua,xml}`: section/role flags, loot menus, model tab and RPE TutorialsFrame.
- Evidence `ObjectMgr.cpp`, `DBCEnums.h` at TrinityCore a352b1fa; `model-source-proof.json`, `npc-import-2.log`, `model-assets/chain-receipts.json`, `stage-models.log`.

## See Also

- [[asset-pipeline]] — local CASC acquisition.
- [[raid-entry]] — authoritative saves and difficulty selection.
