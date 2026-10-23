# Game Menu and Settings navigation

Native menus implement Retail `Blizzard_GameMenu/Shared/GameMenuFrame.lua:139-216` and `Blizzard_Settings_Shared/Blizzard_SettingsPanel.xml`. Forever changes art, not controls. [Implementation](../wiki/systems/godot-conversion.md).

## What it must do

- [x] Game Menu preserves Retail order and section spacing for Options, AddOns, Edit Mode, Support, Macros, Log Out, Exit Game, Return to Game.
- [ ] Edit Mode opens the existing HUD editor in a character session. AddOns, Support and Macros open existing settings pages, not invented screens.
- [x] Settings exposes Game/AddOns category sets through shared Retail tabs; search filters category and setting text case-insensitively, groups matching rows by category, and shows an honest empty result.
- [x] Header geometry follows SettingsPanel.xml: GameTab TOPLEFT(32,27), height37; AddOnsTab follows GameTab +5; SearchBox350x22 at Container TOPRIGHT +(4,20). Both skins retain these anchors.

- [x] Footer uses Retail Apply/Close96x22 controls: Close BOTTOMRIGHT(-16,16), Apply RIGHT→Close LEFT-2. Apply is enabled/visible only for unapplied draft setting values; applying commits them without closing Options.
- [x] Defaults retains the existing category-reset action at SettingsList Header TOPRIGHT(-36,-16),96x22, hidden during search; it is not a footer control.
- [ ] Existing immediate setting updates and menu-close behavior remain unchanged; this follow-up does not introduce deferred settings or Retail's discard-confirmation workflow.

## How it works

- [Native conversion](../wiki/systems/godot-conversion.md)

## Implementation inventory

- `godot/ui-model/src/ui/screens/game_menu_main.rs`: ordered menu and source sections.
- `godot/ui-model/src/ui/screens/options_menu_{component,header,scroll}.rs`: category sets, header and search row projection.
- `godot/ui-model/src/ui/screens/merchant_frame_component.rs`: shared PanelTab/MinimalTab templates and measured widths.
- `godot/ui-model/src/ui/options_menu_data.rs`: navigation state.
- `godot/rust/src/game_menu.rs`: native actions and search input sampling.

## Tests asserting this spec

- `godot/ui-model/tests/game_menu.rs`
- `godot/ui-model/tests/options_views.rs`
- `godot/ui-model/tests/options_policy.rs`
- `godot/tests/ui_menus_followup.gd`: both-skin native1920x1080 sizes, source-relative anchors and physical clicks.
- `godot/rust/src/ui/scroll_lists_tests.rs`: actual wheel/end-clamp/partial-row geometry, including first/last binding-button visibility.

## Known gaps (current cycle)

- [ ] Shop: neither StorePublic nor CatalogShop capability exists; Retail hides when both are disabled (`GameMenuFrame.lua:150-157`). No fake shop.
- [ ] What's New: no SplashScreen/CanViewSplashScreen provider. Retail `GameRulesUtil.ShouldShowSplashScreen` returns false without that capability (`GameRulesUtil.lua:70-76`); hidden.
- [ ] Ratings: no ratings frame. Retail shows only when localization enables `ratingsButtonShown` (koKR in `Blizzard_GameMenu/Shared/Localization.lua:9-13`); hidden for this enUS client.
- [ ] Macros page reports planned editor/storage; this change only exposes that existing page. Support remains the existing About/Support page, not Blizzard support services.
- [ ] Search uses supported rendered settings labels/descriptions, not Blizzard-only initializer search-tag metadata or relevance scoring.

[Roster qualification and filed gaps](../handoffs/ui-generics-game-menu-roster-gap.md) distinguish the offline six-button Escape fixture from full Retail conditions. In particular, Retail always adds Log Out (only disabling it), while the native offline preview hides it; Edit Mode/AddOns/Macros eligibility also lacks the full source predicates. These are not lifecycle Step 5 fixes.

## Out of scope

Commercial store, splash content, ratings implementation, addon manager and macro execution are not introduced by this follow-up.
