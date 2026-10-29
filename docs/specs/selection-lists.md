# Selection lists

Selectable list rows must not use the default UIPanelButton skin. Every row uses one shared hit target (`godot/ui-model/src/selection_list.rs`) and draws the art of the Retail template its list copies: that template's normal, hover and persistent selected textures, with the source anchors and blend. Real actions (Join Battle, Queue, Accept, Train, Apply) remain buttons.

Retail Mainline defines structure, sizes and interaction for both skins. Atlas art resolves through the active skin: Forever draws its authored member for the same atlas name and otherwise the Retail member. File art is the Retail file in both skins. No generated or invented artwork.

Hover art lives on the HIGHLIGHT layer and shows while the row is hovered or keyboard-focused, never while disabled. Rows support pointer selection, keyboard focus, Enter/Space activation and scroll-to-reveal; keyboard eligibility comes from being a row, not from which hover art its template draws. As in Retail, a pointer press never leaves keyboard focus on a row, so Enter afterwards reaches the game and opens chat; Enter/Space activate a row only while it holds keyboard focus (`godot/tests/selection_row_pointer_focus.gd`, both skins). Known gap: with no focused control, no key moves focus into a list, so a row gains keyboard focus only when code focuses it. Scrolling or state refresh must not reset selection. Disabled rows cannot activate.

User choice: generic search-style rows keep idle text flat on the panel instead of the template's `_search-rowbg` normal texture.

## Templates

| Template | Source | Hover | Selected |
|---|---|---|---|
| Generic search row | `Blizzard_GroupFinder/Mainline/LFGList.xml:129-138` `LFGListSearchAutoCompleteButtonTemplate` | `search-highlight` (HighlightTexture, ADD) | `search-select` (OVERLAY, BLEND) |
| Specific battlegrounds | `Blizzard_PVPUI/Mainline/Blizzard_PVPUI.xml:193-256,292-297` `PVPInstanceListEntryButtonTemplate` | `Interface\PVPFrame\PvPMegaQueue` UV 0.00195313,0.63867188,0.70703125,0.76757813, ADD, inset 2px | same file UV …,0.76953125,0.83007813, ADD, inset 2px |
| PvP categories | `Blizzard_PVPUI.xml:591-637`, `Blizzard_PVPUI.lua:564-574` | `bluemenu-main` crop, ADD 0.8 | selected `bluemenu-main` crop |
| Dropdown menus | `Blizzard_Menu/Mainline/MenuVariants.lua:9-49` | `UI-QuestTitleHighlight`, ADD | radio/checkbox tick |
| Gossip / raid info | `GossipFrame.xml:90-102`, `RaidFrame.xml:39`, `RaidFrame.lua:173` | `UI-QuestTitleHighlight`, ADD | raid: locked highlight |
| Guild members | `Blizzard_Communities/CommunitiesMemberList.xml:238-241` | `UI-FriendsFrame-HighlightBar`, ADD | — (GuildFrame line backing as normal) |
| Crafting orders / AH rows | `ProfessionsCrafterOrderPage.xml:36`, `ProfessionsCustomerOrdersRecipeList.xml:9` | `auctionhouse-ui-row-highlight`, ADD | — |
| AH categories | `Mainline/Blizzard_AuctionHouseCategoriesList.xml:21` | `auctionhouse-nav-button-highlight` | authored row art |
| Transmog outfits | `Blizzard_TransmogTemplates.xml:121-132` | `transmog-outfit-card`, ADD | authored outfit card |
| Mount / pet journal | `Blizzard_MountCollection.xml:80`, `Blizzard_PetCollection.xml:7` | `PetList-ButtonHighlight` | authored `PetList-ButtonSelect` |
| Achievement categories | `Blizzard_AchievementUI.xml:650-656` | `UI-Achievement-Category-Highlight`, ADD | authored |
| Profession recipes | `Blizzard_ProfessionsRecipeList.xml:206-216`, `.lua:375-376` | `Professions_Recipe_Hover` 0.5, unselected only | authored `Professions_Recipe_Active` |
| House finder neighborhoods | `Blizzard_HousingHouseFinder.xml:4-115` | neighborhood highlight | — (default item backing as normal) |
| Authored rows (trainer services, achievement rows/summary, stable pets, profession categories) | screen content | screen-owned or none | screen-owned |

## Audit

- Generic search rows: guild ranks, Adventure Guide bosses/overview/sections/search, monthly activities, cooldown-assist settings, debug selection screen.
- Dropdown radio/checkbox menus: crafting-order recipient (Public/Guild/Personal) and duration (12/24/48h), Adventure Guide tier/difficulty/loot filters, mount filters, minimap tracking, talent loadouts.
- Quest giver titles and quest log titles keep their source clickable-title behaviour; they were not default-button lists.
- Retail arena/rated and LFG dungeon selectable lists are not implemented in this client revision. Existing placeholder panels must not acquire invented rows.
- Image grids (appearances, item sets, toys, pets/slots), tabs, checkboxes, action toolbars and context-menu actions are not default-button selection lists.
- Authored rows without template hover (achievement rows/summary, stable pets, profession categories) show no keyboard-focus highlight; trainer services lock their own highlight on focus.

## Acceptance

Behavioral model and native-renderer tests; native private UDP5933 sessions with fb_sellist_* accounts. Before/after Modern and Forever PNGs at most1280px, individually inspected, under `/syncthing/AgentShared/2026-10-12/selection-list-ui/`. Preserve source queue availability policy, including any map708 visibility gate. No queue transport or objective changes.
