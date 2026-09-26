# Unit Tooltip

The Retail `GameTooltip` for units (NPCs and players) hovered in the world, on a nameplate or on a unit frame, plus a drops and vendor section with appearance-collection marks on NPC tooltips. Server data comes over shared-protocol `protocol/tooltip_messages.rs` (`TooltipChannel`); server rules are in game-server `docs/specs/loot.md` (drop chances, junk to gold) and `docs/specs/appearance-collection.md`.

References:
- GT.lua / GT.xml = `Blizzard_GameTooltip/Mainline/GameTooltip.lua` / `.xml`; STT = `Blizzard_SharedXML/SharedTooltipTemplates.lua`; TDR = `Blizzard_SharedXMLGame/Tooltip/TooltipDataRules.lua`; SCC = `Blizzard_SharedXML/SharedColorConstants.lua`, all under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`
- GlobalStrings: the Retail dump in `/syncthing/Sync/Projects/wow/reference-addons.new/AllTheThings/.contrib/GlobalStrings.lua`

## What it must do

- [x] Hovered unit (`rendering/ui/unit_hover.rs`, `HoveredUnit`): a unit frame cluster under the cursor (PlayerFrame, TargetFrame, TargetOfTargetFrame, FocusFrame) gives the unit it shows; with no UI frame under the cursor, the nearest nameplate under it, else the first NPC or player the camera ray through the cursor hits (a wall in front hides the unit behind it).
- [x] Every tooltip, unit or not, uses `GameTooltip_SetDefaultAnchor` (STT:87-114): the tooltip's BOTTOMRIGHT on `GameTooltipDefaultContainer`, which sits at UIParent BOTTOMRIGHT x -9, y 85 (GT.xml:242-247). It no longer follows the cursor.
- [x] NPC lines: the name coloured by `GameTooltip_UnitColor` (GT.lua:120-182) = `FACTION_BAR_COLORS[reaction]` (SCC:3-13) with `shared::faction_reaction` (hostile red, neutral yellow, friendly green); the subname; `UNIT_TYPE_LEVEL_TEMPLATE` "Level %d %s" with the Retail `CreatureType` name, or `UNIT_LEVEL_TEMPLATE` "Level %d" when the type is 10 "Not specified" or not known yet; the unit's faction name when it is a reputation faction (`Faction.ReputationIndex` >= 0: "Stormwind", not "Creature" or "Beast - Wolf").
- [x] Player lines: the name (red when hostile, else white, per `GameTooltip_UnitColor`), "<Guild>" when in a guild, "Level %d %s %s (Player)" with the race and class names. No drops section.
- [x] Subname and creature type come from the server (`CreatureTooltipQuery { entry }` answered by `CreatureTooltip`), asked once per creature entry on first hover and cached until leaving the world. Until the answer arrives the NPC tooltip shows name, "Level %d" and faction.
- [x] **Deviation from Retail (user request): drops and vendor sections.** Below the NPC lines, "Drops" lists the loot table's items with their chance and "Sells" the vendor's items without one. Each item is its name in its quality colour (`merchant_data::quality_color`) with a mark before it: a green check for an item whose appearance the account has collected, a red cross for one it has not, nothing for items without an appearance (reagents, consumables, recipes, quest items). The marks are Retail `READY_CHECK_READY_TEXTURE` `UI-LFG-ReadyMark` and `READY_CHECK_NOT_READY_TEXTURE` `UI-LFG-DeclineMark` (ReadyCheck.lua:2-4), members of `interface/lfgframe/uilfgprompts.blp` (5171843), drawn 12×12.
- [x] Section order: uncollected items first, then the rest; drops by highest chance within each, vendor items in slot order. Up to 6 items are all listed; more show the first 5 and "+N more", with " (k collected)" when the hidden items include any with an appearance (k counts the collected hidden ones). The wording follows Retail `QUEST_HUB_TOOLTIP_MORE_QUESTS_REMAINING` "+%d more". Chances read "60%", "20.1%", "0.04%", "<0.01%".
- [x] **Deviation from Retail (user request): record ID line.** A tooltip that describes a record ends with one grey (`GRAY_FONT_COLOR`, body size) line, idTip-style, always shown: NPC tooltips "Creature ID: <entry>", spell tooltips (action bar, spellbook, chat links, talents, player and target auras) "Spell ID: <id>", item tooltips (bags, merchant and buyback cells) "Item ID: <id>". Player tooltips, the XP bar and mail have none. `place_tooltip` appends it for every kind from `TooltipFrameState::record`. The item rows inside the drops and vendor sections carry no IDs (one line per item).
- [x] A creature without a loot table drops nothing, not even coins (game-server `loot.md`); its tooltip has no Drops section, no header and no money line.
- [x] The "Level N" line reads the unit's level through one accessor (`unit_sources::displayed_level`, today the replicated `UnitLevel` as the target frame shows it), the single place to switch to the viewer-scaled level of the levelscaling work.
- [x] Grey (poor) items never appear in the Drops section: loot turns them into coins (game-server `loot.md`).
- [x] Collected state is the account collection the server sends (`AppearanceCollectionUpdate`, `AccountAppearances`); a newly learned appearance updates an open tooltip.
- [x] IPC `hover --npc NAME` / `hover --x X --y Y` puts the cursor on the nearest on-screen NPC of that name or on a window point, for headless proof.
- [ ] Not built: health bar under the name (TDR:130-141), PvP and classification lines ("Elite", "Rare", "Level ??" for bosses and skull levels), level colour by difficulty, class-coloured class name, tooltip fade-out, the `Cursor` and `Nameplate` world-cursor anchor types, Retail owner anchors for bag, merchant and action buttons (all tooltips use the default anchor, as requested), gameobject tooltips.

## Tests asserting this spec

- `src/scenes/tooltip_frame/unit_tooltip_tests.rs`: Defias Thug lists the uncollected Pitted Defias Shortsword first, then by chance, 5 lines and "+4 more"; a collected appearance shows the check and loses its priority; 14 items show 5 lines and "+9 more (9 collected)"; 6 items are all listed; Corina Steele's "Sells" with subname, faction and "+3 more (1 collected)"; basic lines before the server answers; the player lines; chance text.
- `src/scenes/tooltip_frame/unit_sources.rs`: only reputation factions are named (Faction.csv).
- `src/scenes/tooltip_frame/mod.rs`: the default anchor; the grey ID line of each kind (Frostbolt "Spell ID: 116", Linen Cloth "Item ID: 2589" in bags and merchant, a talent, an aura), none for mail.
- `src/scenes/tooltip_frame/unit_tooltip_tests.rs`: Defias Thug's record is "Creature ID: 38", players have none.
- `src/game/networking/unit_tooltip_tests.rs`: an entry is asked for once and its answer cached; a collection update replaces the account appearances.
- `src/bin/game-engine-cli/tests/camera.rs`: `hover` arguments.
- Live evidence: `data/diagnostics/npctooltip-20260926/` (Dermot Johns vendor, Defias Thug drops, gloves collected, Brother Danil before/after learning Scout's Arrow live, player frame).
