# Retail gameplay-panel fidelity

Retail XML/Lua under `Blizzard_*/retail/AddOns` defines controls, sizes, anchors and art for both skins; Forever swaps art only. Preserve existing reducers and authoritative decisions. Use shared panel tabs, dropdown/menu elements and MinimalScrollBar; never substitute defaultbutton rows or solid popup fills. Atlas-coordinate drift is a named data gap, not permission to hand-correct coordinates.

## Acceptance

- [x] Customer orders: Place Order/My Orders PanelTabButtonTemplate and recipient/duration WowStyle1DropdownTemplate use authored anchors, pending state and existing actions. Personal orders show Order To; payment labels stay inside the right pane and share input centerlines. Back uses UIPanelButtonTemplate sheet art; the portrait shows the interacting NPC. Form.lua overrides recipient/duration widths to136/143px. [Crafting contract](crafting-ui.md).
- [ ] Gameplay menus: damage-meter radio menus, minimap tracking checkboxes, transmog source/completion filters and unit difficulty submenus use Retail menu backdrop/elements, source20px rows and measured title/label extents. [Shared menu contract](menu-primitives.md).
- [ ] Talent loadout dropdown/menu/dialog, actual talent choice entries, cooldown settings and Edit Mode LayoutDropdown use their named Retail templates without replacing supported behavior.
- [ ] Guild rank controls, scoreboard tabs, rated/casual arena rows and pet selection buttons match their Retail controls.
- [ ] Raid Info list uses MinimalScrollBar instead of arrow action buttons.

Each finished group requires behavior tests, XML-geometry assertions, native1920×1080 before/after captures in both skins, and inspection alongside the corresponding XML. Source screening alone is not visual acceptance. Each landable report names its commit group; incomplete groups remain explicit.

## Proof owners

- [Customer-order source/control proof](../wiki/systems/crafting-ui.md#customer-control-fidelity--verified-2026-10-13).
- [Shared menu rendering and source slicing](../wiki/systems/menu-primitives.md).
- `godot/tests/gameplay_panels_capture.gd` and native offline previews mount production screens without server mutation.
- `godot/ui-model/tests/{professions_orders,menu_primitives,minimap_tracking,transmog_wardrobe,forever_chat_meter,talent_footer,battleground}.rs` exercise changed controls and observable actions.
