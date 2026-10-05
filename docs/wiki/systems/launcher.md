# In-world launcher

Native centred search grid. Contract: [launcher spec](../../specs/launcher.md). Existing micro menu remains mounted unchanged.

## State and dispatch

`godot/ui-model/src/launcher.rs` owns open/query/selected state. Labels match case-insensitive word prefixes; filtering resets selection. Five-column arrow navigation clamps to visible entries. Enter/click returns the same action stored on its entry and closes the launcher.

Micro entries derive labels, actions and normal icons from `MICRO_BUTTONS`. Character renders the same portrait/mask at double size through `unit_portraits.rs`. Additional entries invoke existing backpack, map and Options controllers. Key Bindings enters Options directly on Interface bindings. Help invokes the existing Support action, currently a placeholder; other unconverted micro windows retain their existing error messages.

`godot/rust/src/launcher.rs` observes toggle/navigation before the native search editbox consumes them, drains text before activation, focuses search on opening and releases the canvas on closing. The registry canvas participates in existing UI visitation/scale/hit-testing. Keyboard gameplay stays blocked while launcher is open.

## Art and sources

- Forever: existing `metal_frame_no_portrait` style, composed by the registry host.
- Modern: existing `static_popup` diamond-dialog border (FDID 6795680), existing dark dialog background atlas.
- Search: `common-search-magnifyingglass`, Retail `UiTextureAtlasMember.csv` member 9809, atlas1541, FDID3281887. Existing local DB2 records; no downloads.
- Micro icons/actions: `godot/ui-model/src/micro_menu.rs`; host `godot/rust/src/character_frame.rs`.
- Help icon: Retail `AddOns/Blizzard_MicroMenu/Mainline/MainMenuBarMicroButtons.lua:1773-1774`, `LoadMicroButtonTextures(self, "GameMenu")`.
- Launcher lifecycle/filter/navigation are the user's October 5, 2026 design, not Retail behavior.

## Sources

- [Spec](../../specs/launcher.md)
- `godot/ui-model/src/launcher.rs`, `godot/rust/src/launcher.rs`
- Local `data/UiTextureAtlasMember.csv` / `data/UiTextureAtlas.csv`

## See Also

- [[minimap]] — buff-sized opener beside the cluster
- [[keybindings]] — persisted Toggle Launcher action
