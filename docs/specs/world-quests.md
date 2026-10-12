# World quests

Retail Mainline world quests: authoritative availability, area-entry acceptance, objective progress, automatic completion and authored rewards, presented with the same behavior in Modern and Forever. Source availability is recorded in the [local source investigation](../wiki/investigations/world-quest-availability.md). Runtime and live acceptance remain open. Deterministic rotation is authorized by the user's 2026-10-11 decision; it is World of Osso policy, not Retail activation data.

## What it must do

- [ ] Server publishes eligible authentic world-quest definitions from local DB2 `12.1.0.69933`, read-only world.db and TDB1210 using deterministic World of Osso rotation. Every active duration comes from that quest's authentic DB2 definition; missing durations are excluded and reported, never assigned a guessed default. Rewards remain authentic quest data only.
- [ ] Server accepts an available world quest on area entry, credits its authored objectives, completes it and grants authored rewards. Behavioral tests cover eligibility, expiry, completion and duplicate-reward prevention.
- [ ] World-map pins and tooltips show available world quests and their rewards/remaining availability; the objective tracker shows accepted task progress. Modern and Forever share Retail behavior; Forever changes art only.
- [ ] Private UDP5861 proof with `fb_*` accounts completes a real world quest in each skin, with inspected PNGs under `/syncthing/AgentShared/2026-10-11/world-quests/`.
- [ ] Changed repositories merge origin/master, pass targeted tests/check/fmt (full protocol suite if changed), and push explicit feature refs. PLAN.md remains read-only and uncommitted; canonical world.db and UDP5000 remain untouched.

## How it works

- [Local availability source boundary](../wiki/investigations/world-quest-availability.md)
- [Existing quest UI](../wiki/systems/quest-ui.md)

## World of Osso rotation policy

Retail Mainline defines map/task/tracker/completion/reward/expiry behavior, but does not supply our activation calendar. The realm's policy is deterministic from quest IDs and UTC server time, with no random startup state:

- `WORLD_QUEST_EPOCH`: Unix seconds; default `1767225600` (2026-01-01 00:00:00 UTC). No quest is active before it.
- `WORLD_QUEST_ORDER`: `ascending` (default) or `descending` quest-ID ordering, independently in each authored zone.
- `WORLD_QUEST_ACTIVE_PER_ZONE`: positive concurrent capacity, default `1`. Sorted definitions are assigned round-robin to that many independent lanes, capped by the zone's candidate count.
- `WORLD_QUEST_GAP_SECONDS`: nonnegative inter-quest gap, default `86400`; this gap is realm policy, not DB2 duration/down-time data.
- Each lane cycles its ordered quests. A quest is active in `[start, start + authentic_duration)`, then the configured gap elapses. Lane period is the sum of its durations and gaps. Availability and completion history use that exact window; a character cannot earn the same quest twice within it.

Currently only quests 52847, 52848, 54895, 54896 have authenticated duration joins: ManagedWorldState12/113/114/115, each `UpTimeSecs=604800`. The other 4,203 definitions have no proven duration in the inspected sources and cannot yet enter this rotation. Their missing data does not justify changing the four authored durations.

## Implementation inventory

Server rotation calculation, environment configuration, definition loader, private-copy importer and persisted task expiry are implemented. Area-entry acceptance, automatic rewards, availability transport and native client integration remain open. Existing ordinary quest/map/tracker paths are prerequisites, not world-quest acceptance evidence.

## Tests asserting this spec

- Server `world_quests::tests`: deterministic restart/input ordering, half-open boundaries, differing authored durations/capacity and process-isolated environment configuration.
- Server `world_quests::expiry_tests`: exact-end persisted task/watch removal across actual redb close/reopen, ordinary timed-quest isolation and rejection of zero-duration definitions.
- `scripts/test_import_world_quests.py`: positive-only duration admission, authentic objectives/POIs/race masks/raw rewards, idempotence, source-copy immutability and output no-clobber.

These tests establish foundations, not area-entry/completion/reward or live UI acceptance. Existing `world_quest_flow.gd` and `world_quest_progress.gd` fixtures exercise ordinary NPC-given quests, not world quests.

## Known gaps (current cycle)

- [ ] Duration coverage beyond the four managed-state definitions. Missing Retail activation data is no longer a blocker: the explicit realm policy above supplies scheduling.
- [ ] Retail auto-accept/task-area and eligibility joins for the four admitted quests. All four have AreaGroupID=0, no TDB area-trigger involved relation, single-point static DB2 task pins and no TDB boss spawn. QuestSortID is a category/zone identifier, not evidence of an acceptance radius.
- [ ] Automatic authentic currency/reputation/spell reward settlement, protocol availability payload, both-skin map/tracker integration and private completion proof remain missing. Raw TDB reward fields are retained by the importer; they have not been awarded.

## Out of scope

Invented durations/rewards, Retail activation-calendar claims, canonical database writes, UDP5000 operations, PLAN edits and full server/engine suites are prohibited by the requested goal. No feature requirements were deferred out of scope.
