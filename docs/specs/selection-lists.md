# Selection lists

Selectable list rows must not use the default UIPanelButton skin. Idle generic rows show left-aligned text on the panel, authentic Retail hover artwork, and persistent selected artwork plus a check. Disabled rows cannot activate. Icons and subtext remain optional. Real actions (Join Battle, Queue, Accept, Train, Apply) remain buttons.

Retail Mainline defines structure and interaction for both skins. Forever substitutes authored atlas members only; absent Forever art uses the same Retail member. No generated or invented artwork.

Shared row interaction includes pointer selection, visible keyboard focus, keyboard activation, and existing scroll behavior. Scrolling or state refresh must not reset selection.

## Audit

- Battleground specific queue, guild roster/ranks, Adventure Guide bosses/search/filter menus and cooldown-assist selection rows: generic selection rows.
- Mount/pet journals, transmog outfits, trainer/profession recipes, auction results/categories, achievement rows/categories, quest/gossip and stable pets: retain their specialized authentic row art, share selection hit-target behavior where applicable.
- Retail arena/rated and LFG dungeon selectable lists are not implemented in this client revision. Existing placeholder panels must not acquire invented rows.
- Image grids (appearances, item sets, toys, pets/slots), tabs, checkboxes, action toolbars and context-menu actions are not default-button selection lists.

## Source

Local Retail cache: `Blizzard_PVPUI/Mainline/Blizzard_PVPUI.xml`, `PVPInstanceListEntryButtonTemplate` / `PVPSpecificBattlegroundButtonTemplate`; `Blizzard_GroupFinder/Mainline/LFGList.xml` search/group list entries; SharedXML ScrollBox templates. Hover `search-highlight` and selection `groupfinder-highlightbar-blue` resolve from local UiTextureAtlas DB2 exports and local CASC textures.

## Acceptance

Behavioral model and native-renderer tests; native private UDP5933 sessions with fb_sellist_* accounts. Before/after Modern and Forever PNGs at most1280px, individually inspected, under `/syncthing/AgentShared/2026-10-12/selection-list-ui/`. Preserve source queue availability policy, including any map708 visibility gate. No queue transport or objective changes.
