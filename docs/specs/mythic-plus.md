# Mythic Keystone UI

Retail Mainline defines both skins' elements and behavior. Forever changes art only. Server contract/source qualification: [server Mythic+](../../../game-server/docs/specs/mythic-plus.md).

- Bags carry the authoritative keystone item GUID; picking it up and clicking the Font slot sends a bag-ownership-validated Slot request.
- `ChallengesKeystoneFrame` uses Mainline398x548 geometry,48x48 slot, dungeon/level/time labels and120x24 Activate button. Activation closes the slot frame after the server starts the run.
- The shared scenario/objective tracker shows countdown, authored remaining time, weighted forces, encounter kills, deaths and time lost. Completed/abandoned state displays resulting key level; absent rating stays absent.
- `ChallengeModeCompleteBanner` displays dungeon/level, within/over time and key result from the server; no synthetic completion.
- Every operation uses the existing reliable InstanceChannel. Wrong copy, wrong dungeon, range, death, stolen/moved keys and repeated start are server decisions.

Sources: local Retail `Blizzard_ChallengesUI/Mainline/Blizzard_ChallengesUI.xml:218-459,578+`, Lua640-746,850-925; `Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua:774-845`. Server DB2/source references remain their SSOT.

Proof: targeted UI model `mythic_challenge` and native Font rendering RED/GREEN; inspected private5800 physical Font/bag/slot/Activate in Modern and Forever. Modern within-time upgrade and Forever clock-advanced overtime depletion use production NPC death/BossAI boundaries. [Outcomes/boundaries](../../../game-server/docs/wiki/systems/mythic-plus.md#proof--2026-10-11). Unsourced countdown/death/season/rating policy stays server-gated disabled/0; UI does not assign it.

Gaps: authored Azure combat mechanics beyond existing BossAI lifecycle; source-scenario event producers; keystone minimum-level floor/next-dungeon policy; full affix art/mechanics, slot drag-removal and completion-banner animation/right-click dismissal. These are not claimed by timer/forces/slot proof.
