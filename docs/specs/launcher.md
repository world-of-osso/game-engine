# In-world launcher

Centred search-and-icon launcher for in-world windows. Source: `godot/ui-model/src/launcher.rs` and `godot/rust/src/launcher.rs`. [Implementation](../wiki/systems/launcher.md).

## What it must do

### Opening and dismissal
- [ ] Ctrl+Space toggles the launcher by default; Options > Key Bindings lists the rebindable action as **Toggle Launcher**.
- [ ] A buff-sized magnifying-glass button next to the minimap opens it. No top-edge hover trigger.
- [ ] Search receives keyboard focus on opening. Escape or the toggle binding closes it without opening Game Menu.
- [ ] Enter activates the selected entry; clicking activates that entry. Activation closes the launcher.

### Search and navigation
- [ ] Typing filters labels case-insensitively by any word prefix: `spe` shows Talents & Spellbook, not unrelated entries.
- [ ] Arrow keys move selection through the visible grid. Filtering resets selection; empty results never dispatch an action.

### Inventory and appearance
- [ ] Include every existing micro entry: Character, Professions, Talents & Spellbook, Achievements, Quest Log, Housing Dashboard, Guild & Communities, Group Finder, Collections, Adventure Guide, Shop and Game Menu; also Help, Bags (backpack), World Map, Options and Key Bindings.
- [ ] Micro entries reuse existing micro-menu icons and host actions, including unavailable-window messages. Existing micro menu is unchanged.
- [ ] Centre panel containing search above large icons with labels below. Use existing Forever metal frame or Modern dialog artwork; no new artwork.

## How it works

- [Launcher](../wiki/systems/launcher.md)
- [Minimap](../wiki/systems/minimap.md)

## Implementation inventory

- `godot/ui-model/src/launcher.rs` — inventory, search, selection and screen.
- `godot/ui-model/src/micro_menu.rs` — shared micro icon metadata.
- `godot/ui-model/src/minimap.rs` — search opener.
- `godot/core/src/input_bindings_data.rs` — persisted Toggle Launcher binding.
- `godot/rust/src/launcher.rs` — keyboard ownership, focus, lifecycle and shared dispatch.
- `godot/rust/src/ui/mod.rs` — panel styles and projection.
- `godot/rust/src/unit_portraits.rs` — Character entry portrait.

## Tests asserting this spec

- `godot/ui-model/tests/launcher.rs` — toggle, filtering, selection, activation, Escape, minimap click and inventory.
- `godot/core/tests/input_bindings_data.rs` — binding label, section and Ctrl+Space matching.
- `godot/ui-model/tests/micro_menu.rs` — unchanged micro-menu behavior.

## Known gaps (current cycle)

- [ ] Targeted tests and live Forever captures pending.
- [ ] Help uses existing Support placeholder; no native Help window exists.

## Out of scope

- Replacing or hiding the micro menu: user reviews launcher first.
- Implementing missing destination windows: retain micro-menu unavailable behavior.
- Top-edge hover opening, new artwork and external launcher integration.
