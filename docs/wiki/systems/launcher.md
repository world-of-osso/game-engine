# In-world launcher

Native centred search grid. Contract: [launcher spec](../../specs/launcher.md). Existing micro menu remains mounted unchanged.

## State and dispatch

`godot/ui-model/src/launcher.rs` owns open/query/selected state. Labels match case-insensitive word prefixes; filtering resets selection. Five-column arrow navigation clamps to visible entries. Enter/click returns the same action stored on its entry and closes the launcher.

Micro entries derive labels, actions and normal icons from `MICRO_BUTTONS`. Character renders the same portrait/mask at double size through `unit_portraits.rs`. Additional entries invoke existing backpack, map and Options controllers. Key Bindings enters Options directly on Interface bindings. Help invokes the existing Support action, currently a placeholder; other unconverted micro windows retain their existing error messages.

`godot/rust/src/launcher.rs` observes toggle/navigation before the native search editbox consumes them, drains text before activation, focuses search on opening and releases the canvas on closing. The registry canvas participates in existing UI visitation/scale/hit-testing. Keyboard gameplay stays blocked while launcher is open.

## Art and sources

- Forever: existing `metal_frame_no_portrait` style, composed by the registry host.
- Modern: existing `static_popup` diamond-dialog border (FDID 6795680), existing dark dialog background atlas.
- Search: `common-search-magnifyingglass`, local Retail/Forever DB2 member34111, atlas3172, FDID6725697 (1x canvas selected by the existing atlas resolver). The older canvas0 member9809/atlas1541 is FDID3281887. No downloads.
- Micro icons/actions: `godot/ui-model/src/micro_menu.rs`; host `godot/rust/src/character_frame.rs`.
- Game Menu and Help: local Retail `AddOns/Blizzard_MicroMenu/Mainline/MainMenuBarMicroButtons.xml` declares `MainMenuMicroButton`/`HelpMicroButton` with their corresponding mixins; `.lua:1773-1788` loads `GameMenu` for both. `UI-HUD-MicroMenu-GameMenu-Up` resolves to Retail FDID4708813/member17183 or Forever FDID8200846/member39848. Both crops contain Blizzard's authored red question mark, not the unknown-icon FDID134400. Keep required micro art rather than substitute an unrelated icon.
- Options: `Gear`, member4702, Retail FDID1121272. Key Bindings: `newplayertutorial-keyboard`, member10556, FDID1065418, extracted from local CASC; preserve its 480×169 aspect ratio. World Map: `UI-HUD-Minimap-Tracking-Up`, member14849, FDID4618651 (Retail's folded-map tracking button). Options/Key Bindings previously reused GameMenu; World Map incorrectly reused Questlog. These were wrong assignments, not failed texture resolution.
- Launcher lifecycle/filter/navigation are the user's October 5, 2026 design, not Retail behavior.

## Sources

- [Spec](../../specs/launcher.md)
- `godot/ui-model/src/launcher.rs`, `godot/rust/src/launcher.rs`
- Local `data/UiTextureAtlasMember.csv` / `data/UiTextureAtlas.csv`

## See Also

- [[minimap]] — buff-sized opener beside the cluster
- [[keybindings]] — persisted Toggle Launcher action
