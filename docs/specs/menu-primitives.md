# Shared Retail menu and navigation primitives

Dropdowns, context menus and breadcrumb navigation implement Retail Mainline `Blizzard_Menu` and `Blizzard_FrameXML/Mainline/NavigationBar.xml`. Forever changes art through existing atlas resolution, not elements, layout or behavior. [Implementation](../wiki/systems/menu-primitives.md).

## What it must do

- [x] Shared WowStyle1 dropdown uses the XML text-holder, 27px arrow and text anchors while preserving existing callback names and caller dimensions.
- [x] Filter and WowStyle2 variants expose open/disabled state and the 23px reset control; Filter follows measured-text+60 source resizing.
- [x] Context menus use MenuStyle1 backdrop/insets and sourced button, checkbox, radio, title, divider and submenu elements; disabled entries lose responders and selection-art saturation.
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

## Known gaps (current cycle)

No remaining primitive-scope gaps. Source textures used by these templates are present in canonical `data/textures/`; no extraction performed.

## Out of scope

- CASC extraction while install-following work is blocked; no fabricated art substitutions.
- Rebuilding unrelated screen layouts.
