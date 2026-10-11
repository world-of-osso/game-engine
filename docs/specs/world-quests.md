# World quests

Retail Mainline world quests: authoritative availability, area-entry acceptance, objective progress, automatic completion and authored rewards, presented with the same behavior in Modern and Forever. Source availability is recorded in the [local source investigation](../wiki/investigations/world-quest-availability.md). This contract is entirely unverified; no runtime implementation is claimed.

## What it must do

- [ ] Server publishes authentic eligible world quests, availability windows and rotation from local DB2 `12.1.0.69933`, read-only world.db and TDB1210. When no authentic schedule exists, use only availability actually defined by TDB and record the missing schedule; never fabricate availability or rewards.
- [ ] Server accepts an available world quest on area entry, credits its authored objectives, completes it and grants authored rewards. Behavioral tests cover eligibility, expiry, completion and duplicate-reward prevention.
- [ ] World-map pins and tooltips show available world quests and their rewards/remaining availability; the objective tracker shows accepted task progress. Modern and Forever share Retail behavior; Forever changes art only.
- [ ] Private UDP5861 proof with `fb_*` accounts completes a real world quest in each skin, with inspected PNGs under `/syncthing/AgentShared/2026-10-11/world-quests/`.
- [ ] Changed repositories merge origin/master, pass targeted tests/check/fmt (full protocol suite if changed), and push explicit feature refs. PLAN.md remains read-only and uncommitted; canonical world.db and UDP5000 remain untouched.

## How it works

- [Local availability source boundary](../wiki/investigations/world-quest-availability.md)
- [Existing quest UI](../wiki/systems/quest-ui.md)

## Implementation inventory

No world-quest runtime implementation. Existing ordinary quest/map/tracker paths are prerequisites, not world-quest acceptance evidence.

## Tests asserting this spec

None. Existing `world_quest_flow.gd` and `world_quest_progress.gd` fixtures exercise ordinary NPC-given quests, not world quests.

## Known gaps (current cycle)

- [ ] Authentic active-set/window source: inspected TDB quest pools and event relations contain no world quests; local exports do not establish activation windows.
- [ ] Server lifecycle, protocol availability payload, both-skin map/tracker/reward integration and private completion proof remain missing.

## Out of scope

Invented schedules/rewards, canonical database writes, UDP5000 operations, PLAN edits and full server/engine suites are prohibited by the requested goal. No feature requirements were deferred out of scope.
