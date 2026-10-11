# Built-in Mythic Mentor Keys panel

- Launcher **Mythic Mentor** opens an independent native Keys panel; close button/Escape closes it. No Group Finder service or Lua addon runtime is required.
- Layout follows installed `BossHelper/UI/GroupFinderPanel.lua` and `KeystoneWidget.lua`: 176-wide panel, 22-unit header, 8-unit padding, 160×36 rows with 4-unit gaps, 26-unit icons, dungeon/+level, class-coloured player name and leader crown. Dungeon time limit is displayed beside the player name, from the server's authored catalog, not calculated from key level.
- Server [party summaries](../../../game-server/docs/specs/mythic-mentor.md) replace the entire list. Self first, then party roster, including “No key”; solo self remains visible; raids/empty lists hide it. Bag/roster/leader changes refresh an open panel without stale rows.
- Native title drag clamps to the screen and saves per-character placement in existing `ui_layout.ron`.
- Retail Mainline UI assets only, extracted from local CASC. Forever changes the same flat-frame chrome's art, not elements, layout or behaviour. Azure icon: local listfile `interface/lfgframe/ui-lfg-background-azurevaults.blp` (4880614); leader crown132062; no-key icon134400. This icon is explicitly selected local dungeon art, not a claimed C_ChallengeMode icon mapping.
- Dungeon icon teleport mapping matches the installed `UI/MythicTeleport.lua`. No teleport is assigned to Azure401. Casts use normal server authority; unknown spells are not learned by opening the panel.

Excludes addon messaging, seasonal-key fabrication, addon settings/start-page cards and new Group Finder services. Current server qualification is Azure Vault; this panel does not invent acquisition/catalog data for other dungeons.

Proof: [native runtime and final both-skin private5811 evidence](../wiki/systems/mythic-mentor.md).
