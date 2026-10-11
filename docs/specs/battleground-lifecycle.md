# Native battleground lifecycle

## Contract

Retail Mainline defines PvP UI and behavior; Forever changes art only. Implement Warsong Gulch (BattlemasterList1014, map2106): specific queue, invitation with Enter Battle/Leave Queue, source-position entry, flag pickup/capture world-state HUD, retained-roster scoreboard, match results and explicit origin return. Server owns identity, phase, scores, flags, outcome and return coordinates. UI requests echo match identity and remain pending until server response. Errors clear pending without discarding retained authoritative state.

## Source

Local Retail export `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:
- `Blizzard_PVPUI/Mainline/Blizzard_PVPUI.xml`: PVPUIFrame563×428, specific battleground selection and HonorFrame queue button.
- `Blizzard_GroupFinder/Mainline/PVPHelper.xml`: PVPReadyDialog306×210; EnterBattle/LeaveQueue buttons115×22. Lua uses accept/decline rather than starting a local match.
- `Blizzard_PVPMatch/PVPMatchScoreboard.xml`:1024×420, All/Alliance/Horde tabs and roster table.
- `Blizzard_PVPMatch/PVPMatchResults.xml`:1024×620 and end results/leave.

[Server lifecycle contract](../../../game-server/docs/specs/battleground-lifecycle.md) owns source content and policy. Source WSG capture limit3; team flags and score update from typed PvpStateUpdate. The source-backed WSG selection is the only selectable mode here; no unsupported queue modes are fabricated.

## Explicit parity gaps

Preparation, invitation, match, drop/reset and exit timers are unsourced. No fabricated countdown. Honor awards/levels/honorable kills remain absent unless sent by server. Current server combat counters are not instrumented; native table displays em dashes, not false measured zeros. Results omit unknown rewards. Native chrome uses existing skin-aware metal/button art; full PvP-specific texture parity remains separate from functional proof.

## Verification

Behavioral tests exercise both skins, invitation independent of queue-window visibility, identity echo/pending/error recovery, authoritative HUD/roster/outcome and absent fabricated honor. Live multi-account/private UDP proof and inspected PNGs are required before completion; no UDP5000 or canonical DB writes.
