# Glue/menu Retail fidelity

Character-select navigation, deletion/collections, game menu and Settings use Retail templates. Forever swaps art only. Existing selection, category, paging and deletion safety behavior remains. [Implementation/source receipts](../wiki/investigations/glue-menu-fidelity.md).

## What it must do

- [x] CharacterSelectNavBar uses55px root,51px buttons, zero-spacing tray,4px width padding and measured GlueFontNormal label+70px widths; source end-cap/selected/divider art replaces gold rules and colored boxes.
- [x] MainMenuFrame uses260px width,48/34px vertical and28px side padding,zero spacing,20px sections; DialogBorder/DialogHeader and source three-slice button art replace independent framed panels.
- [x] GlueDialog uses512x256 base container,150x32 input and200x30 buttons, source dialog/tooltip art; existing typed DELETE/countdown gate and actions stay.
- [x] GlueCollections uses703x595 parent, source IconsFrame/grid anchors,215x173 cards and224x183 selected art at(-4,+5); paging follows the six entries fitting source geometry and clamps stale pages.
- [x] Campsite hover follows selection: selected cards use `campcollection-frame-selected-hover` with18px atlas slices, unselected cards retain `campcollection-frame-hover` with17px slices. Hover remains224x183 at(-4,+5), disappears on leave, and updates after reselection without changing actions or selected-id behavior (`GlueWarbandSceneCollection.lua:13-19`, atlas slice elements29572/29531); both-skin native proof gates acceptance.
- [x] Settings uses920x724 root,175x20 category rows,30x29 checkbox controls,160x22 source binding buttons and shared dropdown/radio art. Category/value/binding actions remain.
- [x] Options has one popup open at a time, attached outside ScrollBox clipping; selecting forwards `options_toggle:key:value` and closes, Escape closes only popup, outside click closes without intercepting normal routing.
- [x] Fractional source shadow crops must not render unrelated atlas members.
- [x] Behavior and source-geometry tests plus inspected both-skin native captures gate completion.

## How it works

- [Template geometry, atlas margins and native input root cause](../wiki/investigations/glue-menu-fidelity.md).
- [Shared menu art](../wiki/systems/menu-primitives.md).

## Implementation inventory

- `godot/ui-model/src/ui/screens/{char_select_top_nav_component,char_select_delete_confirm_component,campsite_component,game_menu_main,options_menu_component}.rs` — template geometry/art.
- `godot/ui-model/src/ui/screens/options_menu_active_sections*.rs` — source checkbox/dropdown/binding controls.
- `godot/ui-model/src/ui/screens/options_dropdown.rs` — ephemeral popup state, shared radio composition, root portal.
- `godot/rust/src/ui/{options_dropdown,projection}.rs` — native popup dispatch/dismissal and passive-art hit testing.
- `scripts/export_db2_csv.py` — exact pinned atlas slice-data export.

## Tests asserting this spec

- `godot/ui-model/tests/{character_select,game_menu,options_views,options_policy,menu_primitives}.rs`.
- `scripts/tests/test_glue_atlas_slices.py` and its real local DB2 fixture.
- `godot/tests/{glue_menus_capture,options_dropdown_capture}.gd` — native source dimensions and physical click/dismissal boundaries.

## Known gaps (current cycle)

None in these bounded UI template/dispatch gates. Captures do not prove authenticated deletion or server-side campsite persistence.

## Out of scope

Server/account changes, adding unsupported Retail services, changing shared menu art, canonical data/cache mutation, CDN extraction.
