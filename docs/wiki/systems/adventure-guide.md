# Native Adventure Guide

Verified: 2026-10-11. The [contract](../../specs/adventure-guide.md) owns Retail Mainline behavior; Forever only changes the shared screen's art.

## Catalog and native host

`scripts/adventure_guide_import.py` decodes the pinned local69933 Journal WDC5 layouts into `data/cache/adventure_guide.sqlite`, preserving hierarchy, difficulty relations and readable/unreadable counts. Pinned Item/ItemSparse, ChrSpecialization, ChrClasses, GlobalStrings, Difficulty and MapDifficulty CSVs supply metadata. Item compatibility follows TrinityCore a352b1fa `ObjectMgr::ItemSpecStats/LoadItemTemplates`: overrides replace general type/stat matching; no authored specialization means unrestricted. Gem relic traits remain unmodeled without GemProperties.

The core Catalog loads immutable rows; the UI View owns selection, expandable sections, filters and global search. MapDifficulty selects genuine legacy difficulties (Molten Core9), rather than assuming modern raid14; JournalEncounterXDifficulty further restricts boss lists and choices (Vanessa is Heroic-only). Loot menus group robes with Chest and weapon types into Retail MainHand/OffHand categories. A collapsed ability parent remains traversable on Overview when that parent has no visible control there. Item Sets build only viewport-intersecting text rows. Tutorials uses Retail's authored RPE Catch Up Experience strings/art, not invented journal help; the realm lacks the start service.

The native host opens from the existing J binding, micro menu and active boss-frame right-click. First-open requests stay queued behind the existing asynchronous spell catalog, with explicit loading chrome and no raw-token fallback. Its pointer handler routes real wheel/stepper/thumb input through RegistryUi; registry enumeration alone does not wire scrolling. All four Journal pixel lists use Retail ScrollFrameTemplate's30-pixel pan extent, not the native default one-row/one-pixel step. Opening during an active ENCOUNTER_START selects that encounter without opening a window automatically in combat. Raid-entry's InstanceInfo supplies saved status, completed-mask count and snapshot-relative reset time. A detached creature preview hosts the selected JournalEncounterCreature display. `adventure_guide_state` exposes read-only displayed selection/filter/model readiness for native automation.

## Journal model preparation

Import every positive JournalEncounterCreature.CreatureDisplayInfoID with the existing `import_npc_appearance.py --display-id ... --base-cache ...` pipeline. Use pinned CreatureDisplayInfo/TextureFileData, the local listfile, authenticated creature-display catalog and local Extra/Option/Geoset WDC5 inputs. Never add `requires_appearance=0` by hand or bypass strict coverage in the renderer.

The October11 import covers all1889 positive Journal display IDs and retains every prior profile, choice, geoset and coverage row. This proves authored appearance coverage, **not** all1889 model/texture payloads. Ozruk36475 and Lucifron13031 chains were extracted from frozen Retail69933 local archives, content-key/SHA256 checked and published in additive `cache/model-asset-index-adventure-guide.json` (22 assets,6 aliases). Existing no-clobber publisher rules apply. Reproduce a new boss's closure from its actual creature-display model and replacement textures; never relabel unqualified bytes.

## Proof and boundaries

Evidence lives in canonical `data/diagnostics/adventure-guide-2026-10-11/`. The rendered `godot/tests/adventure_guide_scroll.gd` fixture exercises first J open and model→Classic raids→wheel→Molten Core/Lucifron selection against an explicit private fb_* session. Initial native mouse harness captures were one frame behind; settle after injected motion/down/up and assert the observed journal state before accepting pixels. The file-command driver also retries the observed truncate/write response boundary; this is test-harness synchronization, not product fallback.

- CPU proof: targeted both-skin Registry tests, authored sword/spec exclusion and Molten Core9 RED→GREEN; explicit spell-reference text rendering RED→GREEN.
- Native proof is recorded separately in the task's durable proof ledger; passing Registry tests alone does not certify a model, assets or interaction.
- No world.db activity/Journeys/Suggested/keystone tables or client Mythic+ host were present at inspection. No monthly progress, RPE start or keystone state is invented.
- The reused spell-token renderer retains its documented unavailable scaling/token limits; missing values are not manufactured.

## Sources

- [Catalog](../../../godot/core/src/adventure_guide.rs), [shared screen](../../../godot/ui-model/src/adventure_guide.rs), [native host](../../../godot/rust/src/adventure_guide.rs), [importer](../../../scripts/adventure_guide_import.py).
- [NPC appearance import contract](../../specs/npc-appearance-importer.md), [product-isolated asset contract](../../specs/product-isolated-model-assets.md), [raid-entry](raid-entry.md).
- Local `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_EncounterJournal/Mainline/Blizzard_EncounterJournal.{lua,xml}`: section/role flags, loot menus, model tab and RPE TutorialsFrame.
- Evidence `ObjectMgr.cpp`, `DBCEnums.h` at TrinityCore a352b1fa; `model-source-proof.json`, `npc-import-2.log`, `model-assets/chain-receipts.json`, `stage-models.log`.

## See Also

- [[asset-pipeline]] — local CASC acquisition.
- [[raid-entry]] — authoritative saves and difficulty selection.
