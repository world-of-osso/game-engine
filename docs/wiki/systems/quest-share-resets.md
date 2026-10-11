# Quest sharing, resets and timed quests

Native quest sharing, reset re-availability and countdown/failure use the same UI in both skins. Verified 2026-10-11 on the unmerged `quest-share` branches. [Contract](../../specs/quest-share-resets.md).

## Wiring

QuestChannel registers PushQuestToParty, QuestShareOffer, QuestPushResponse and QuestPushResult. Native BridgeConfig must also receive the two server messages: registration alone left the first live push invisible. Account dispatch feeds the popup and exact English sender/recipient GlobalStrings notices; only the server-issued offer ID is returned. The server revalidates membership/eligibility and consumes each offer once.

Timed entries carry absolute `expires_at`, `timer_sampled_at` and `failed`. Account receipt calibrates the model's server clock offset; tracker counts down using local time plus that offset. Sampling matters after admin clock-advance, not only on synchronized real clocks. Failure is authoritative, persists offline and replaces objective lines. The real escort quest938 supplies its authored540-second timer; this work does not implement its escort script.

Daily/weekly persistence and local calendar policy live in the [server runtime page](../../../../game-server/docs/wiki/systems/quest-share-resets.md), not the client.

## Proof

Canonical `data/diagnostics/quest-share-2026-10-11/` holds command logs, fixture driver and proof ledger. Two private fb_questsharea/b accounts (Sharea/Shareb), UDP5517, fresh private redb; no world.db writes or shared realm use. Both skins show quest62 share/accept, quest26153 six authentic Stormwind Pumpkins rewarded then locked/reavailable after clock-advance86400, and quest938 countdown/failure after541 seconds. Native input driver dispatches actual viewport events, never UI callbacks. Fourteen requested PNGs under `/syncthing/AgentShared/2026-10-11/quest-share/` were opened and inspected. Initial failed relay evidence stays in private diagnostics.

Some pre-existing NPC/portrait receipt gaps remain; the quest UI evidence does not certify world asset completeness. Shared realm deployment/master merge were explicitly excluded.

## Sources

- [Native contract](../../specs/quest-share-resets.md) and listed source/tests.
- Local Retail `Blizzard_StaticPopup_Game/GameDialogDefs.lua` QUEST_ACCEPT, `Blizzard_ObjectiveTracker/Blizzard_QuestObjectiveTracker.lua:339`, `data/GlobalStrings.csv`.
- TrinityCore a352b1fa `QuestHandler.cpp::HandlePushQuestToParty`, captured under `data/diagnostics/questmerchlive3-2026-10-09/`.

## See Also

- [[quest-map-objectives]] — existing tracked quest and map selection.
- [[forever-preset]] — account/realm skin selection, not alternate quest mechanics.
