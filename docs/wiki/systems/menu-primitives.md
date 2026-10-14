# Shared Retail menu primitives

Shared dropdown/menu art lives in `godot/ui-model/src/ui/screens/menu_primitives.rs`; breadcrumb navigation lives in `navigation_bar.rs`. [Contract](../../specs/menu-primitives.md).

## API and rendering

`dropdown_button` keeps the existing `DropdownButton` fields and y-up offset. `dropdown_button_with_state` adds style, open/enabled state and optional reset responder without changing old callers. Styles map to Mainline WowStyle1, Filter and WowStyle2. Filter follows source resizeToText: measured label width+60; non-Filter styles preserve explicit dimensions. Filter/Style2 text displaces +2,+1 while pressed. All dynamic menu state remains reducer-owned. `menu_element` exposes button/checkbox/radio/title/divider/submenu art with enabled and selected state; rectangle coordinates are y-down. `context_menu` keeps its existing title/plain-item API. `context_menu_with_rows` composes existing source radio/checkbox elements; `menu_row_rect_with_heading` shares the same8px inset/20px pitch with headerless roots. An empty title omits CreateTitle/CreateDivider and their33px allocation, as required by the follower type menu in LFDFrame.lua; titled Options/menu consumers retain their layout. [Native follower proof](follower-dungeons.md#retail-template-layout-repair--2026-10-13).

Atlas names resolve under active skin; source file regions remain source files in both skins. Forever no longer changes world-map breadcrumb anchors. NavBar receives measured text widths, counts full widths from the end with a20px buffer, overlaps the home/overflow button by15/18px, and places hidden ancestor actions in the overflow menu. WorldMapDisplay owns overflow visibility and clears it on navigation.

RegistryUi synchronizes child normal/pressed regions and dropdown atlas states after screen sync and pointer input. `menu_template_art` sets authored ADD/tile flags unavailable as RSX attributes and applies Retail `C_Texture.GetAtlasInfo` slices: backdrop16/13/16/19, text holder16/0/19/0, Filter8/0/18/0, Style2 12/0/12/0. Whole-member stretching shifts the backdrop metal edge into labels; the native fixture checks source edge pixels. The real world-map consumer and standalone shared-menu capture use the same code paths.

## Source-decimal gotcha

The current RSX parser truncates bare float literals to their integer prefix. Native capture found `selection_list::menu_background` alpha0 instead of source.925, making authentic menu borders invisible. Express fractional attrs as strings/dynamic expressions: `alpha: "0.925"`, reset `button_highlight_alpha: "0.4"`. The native capture fixture checks actual CanvasItem opacity, not source spelling.

## Proof

Targeted integration module `tests/menu_primitives.rs` asserts source geometry, active-skin art, callbacks, selection ticks, disabled items, reset control, overflow retention and pressed art. `godot/tests/menu_primitives_capture.gd` mounts actual RegistryUi in each skin, waits for async art and saves PNGs; native backdrop RED is retained under `data/diagnostics/ui-primitives-2026-10-13/`.

## Sources

- Retail `Blizzard_Menu/Mainline/MenuTemplates.xml:3,66`; shared `MenuTemplates.xml:4` — dropdown geometry/art/reset anchors.
- Retail `Blizzard_Menu/Mainline/MenuTemplates.lua` — MenuStyle1 backdrop alpha/insets.
- Retail `Blizzard_Menu/{MenuVariants.lua,Mainline/MenuVariants.lua}` — menu entry height, ticks, divider/highlight/submenu source art.
- Retail `Blizzard_FrameXML/Mainline/NavigationBar.{xml,lua}` — home, breadcrumb, overflow art/sizing and collapse decisions.
- `UiTextureAtlas*.csv`, local `community-listfile.csv` — authored crops and source FDIDs.
- `/home/osso/Projects/wow/wow-ui-sim/data/atlas.rs:16139,16156-16158` — Retail atlas slice metadata exported from the client.

## See Also

- [World map](../../specs/world-map.md) — shared navigation consumer.
- [Selection lists](../../specs/registry-bevy-ui.md) — registry rendering contract.
