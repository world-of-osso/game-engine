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

## Shared UIPanel button art — 2026-10-14 assignment

`quest_art::panel_button` now emits the actual UIPanel sheet files through toolkit RSX `button_fdid_up/pressed/highlight/disabled`, reusing the existing `ButtonData` texture slots. FDIDs130828/130825/130826/130824 are Up/Down/Highlight/Disabled. `ui::parts::project_panel_button` crops the80×22 member from the128×32 sheet, keeping12px caps even when a caller changes height; highlight is additive. Generic default-button rendering and other template families are unchanged. Five caller-specific repairs (Settings, Talents, customer orders, guild ranks and tabard registration) became redundant and were removed. [Contract](../../specs/gameplay-panel-fidelity.md#shared-uipanel-buttons).

Proof: toolkit attribute/action rebuild RED→GREEN1, real-screen/postsetup-family UI40, native projection24 and emitted Quest/Mail golden controls3 pass; engine/toolkit checks and format pass. Quest Accept, Mail Cancel, disabled Raid Info Extend and Bank Deposit retain their authored/Lua-sized rectangles in both skins. All ten full-HD after views and four-window before/after pixel comparisons were inspected. Settings is an additional control. Captures: `AgentShared/2026-10-14/ui-questlog-helpers/{shared-helper-before-scale1,shared-helper-panel-after}/`; exact source/binary/image hashes and commands live in `data/diagnostics/ui-questlog-helper-fidelity-2026-10-14/helper-proof-ledger.json`.

Before views use the archived7cb2dfee2 renderer in the current native host with the corrected scale1 fixture, not a complete historical client. Initial Quest/Mail captures inherited scale1.40625 and are diagnostic only; their replacements assert exact source geometry. Supplied offline samples prove presentation, not server authority or every helper caller. Window-background tiling has its own implementation and proof boundary below.

Sources: `Blizzard_SharedXML/SecureUIPanelTemplates.xml:39–78`, `.lua:40–80`; Mainline `SharedUIPanelTemplates.xml:3–5,315–320`. Both skins use the same source file button geometry; existing surrounding skin art remains unchanged.

## Shared window texture tiling — 2026-10-14 assignment

`quest_art::window_background` emits one rock region at `(2,21)` with dimensions `(width−4,height−23)`, repeating both axes. The former split at Y51 restarted the texture origin. `_UI-Frame-TopTileStreaks` starts at `(6,21)`, has height43 and width `width−8`, and repeats horizontally only. Retail `Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.xml:76–78,598–609,663–675` supplies these bounds and flags. Border and portrait geometry are unchanged.

Toolkit RSX `horiz_tile`/`vert_tile` write the existing `TextureData` flags independently; native projection already supports repeating a cropped atlas member rather than its entire sheet. Toolkit and engine consumers must land together. [Contract](../../specs/gameplay-panel-fidelity.md).

Current proof: toolkit axis/rebuild RED→GREEN1; real Quest/Mail/Bank/Settings emitted regions and AH source coverage6 pass in both skins. Only twelve expected background/streak golden blocks changed across Quest, small-window and character fixtures; the unused historical bank fixture was not refreshed. Native pixel acceptance and final merged checks remain pending. Evidence lives in `data/diagnostics/ui-questlog-window-tiling-2026-10-14/`; earlier panel-button captures remain the unchanged-tiling before control, not fresh tiling acceptance.

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
