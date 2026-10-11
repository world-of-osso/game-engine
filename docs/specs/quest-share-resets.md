# Quest sharing, resets and timed quests

- Quest log Share sends the selected quest to the party. Server-issued offers show Retail QUEST_ACCEPT Yes/No popup; dismiss/decline reports No. Accept sends the offer ID, never an arbitrary quest grant. Result notices reproduce local GlobalStrings ERR_QUEST_PUSH_* English messages.
- Both Modern and Forever use the same actions, popup and tracker. Forever only changes art.
- Daily/weekly availability and completion timestamps are authoritative server state. Local realm schedule currently uses daily 15:00 UTC / Tuesday 15:00 UTC, inherited from server maps/locks.rs; no universal Retail-region assumption.
- Timed snapshots carry absolute deadline and failure. Tracker displays updating m:ss countdown; authoritative failure replaces objectives with failed text. Timers continue offline; shared timed quests inherit remaining deadline. Failed quests cannot reward.
- Deployment gate: shared-protocol quest-share → game-server quest-share → game-engine quest-share, no mixed-version compatibility.

References: Retail Blizzard_StaticPopup_Game/GameDialogDefs.lua QUEST_ACCEPT; Blizzard_ObjectiveTracker/Blizzard_QuestObjectiveTracker.lua:339; local GlobalStrings.csv QUEST_ACCEPT, ERR_QUEST_PUSH_*, TIME_REMAINING, FAILED; server docs/specs/quest-share-resets.md owns persistence/reset/migration requirements.
