# Shared Retail menu and navigation primitives

Dropdowns, context menus and breadcrumb navigation implement Retail Mainline `Blizzard_Menu` and `Blizzard_FrameXML/Mainline/NavigationBar.xml`. Forever changes art through existing atlas resolution, not elements, layout or behavior. [Implementation](../wiki/systems/menu-primitives.md).

## What it must do

- [x] Shared WowStyle1 dropdown uses the XML text-holder, 27px arrow and horizontal Text anchors while preserving callback names and caller dimensions. Its native caption rectangle stays vertically centered despite Godot's font minimum exceeding the source 10px FontString height; AH retains its Number12Font/RIGHT caller override. Filter and WowStyle2 keep their own source Text anchors. All dropdown captions retain the templates' `wordwrap=false`: long selected captions remain one line after native centering, with existing single-line truncation rather than wrapping.
- [x] Filter and WowStyle2 variants expose open/disabled state and the 23px reset control; Filter follows measured-text+60 source resizing.
- [x] Context menus use MenuStyle1 backdrop/insets and sourced button, checkbox, radio, title, divider and submenu elements; disabled entries lose responders and selection-art saturation. An empty title omits title/divider rows. Uniform radio root descriptions, including the follower dungeon type selector, retain their selected radio marker rather than using a push-button row.
- [x] Menu proxies use `FULLSCREEN_DIALOG`, matching Mainline `Menu.lua:2080–2094` and `UIDropDownMenu.xml:3`. Shared backdrop, row and label order stays above dialog owners regardless of their frame levels; pointer selection remains unchanged. Godot projects distinct `(strata, level, raise, draw-layer)` layers densely below the existing highlight plane; excess distinct layers fail explicitly and report once, never clamp. Frames sharing a layer do not consume extra z capacity.
- [x] Dungeon/raid difficulty companion submenus retain their existing higher frame level and use the shared parent's `FULLSCREEN_DIALOG` stratum. Only ordering changes; rectangles, entries, enabled state and actions remain unchanged.
- [x] Shared NavBar exposes home, measured breadcrumbs and overflow; world map consumes it.
- [x] Both skins retain identical geometry/actions; native captures match source XML.

## How it works

- [Shared primitives](../wiki/systems/menu-primitives.md)

## Implementation inventory

- `godot/ui-model/src/ui/screens/menu_primitives.rs` — dropdowns and context menus.
- `godot/ui-model/src/navigation_bar.rs` — shared breadcrumb navigation.
- `godot/ui-model/src/ui/screens/world_map_frame_component/chrome.rs` — world-map consumer.

## Tests asserting this spec

- `godot/ui-model/tests/menu_primitives.rs` — registry geometry, source art, responders, disabled selection art, measured resizing and press/overflow state.
- `godot/tests/menu_primitives_capture.gd` — both-skin native backdrop opacity and source-pixel slice-edge regression; inspected PNGs under `/syncthing/AgentShared/2026-10-13/ui-primitives/`.
- `godot/tests/guild_menu_order.gd` — actual projected rank-menu order above Last Online/public/officer notes and GuildControl navigation control, both skins.
- `godot/tests/projection_order.gd` — actual native tooltip/dialog, conflicting-strata/level, raise and sibling-ID render/pointer controls, including reused sibling reordering.
- `godot/tests/unit_menu_order.gd` — dungeon and raid companion submenu-over-parent native order in both skins.
- `ui/projection_order.rs` tests — dense ordering, repeated-frame capacity and explicit distinct-layer exhaustion.

## Known gaps (current cycle)

No remaining primitive-scope gaps. Source textures used by these templates are present in canonical `data/textures/`; no extraction performed.

## Out of scope

- CASC extraction while install-following work is blocked; no fabricated art substitutions.
- Rebuilding unrelated screen layouts.
