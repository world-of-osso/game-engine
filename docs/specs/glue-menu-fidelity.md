# Glue/menu Retail fidelity

Retail XML defines geometry; Forever swaps art only. Existing actions, category values, campsite selection/paging and delete safety gates remain unchanged.

## Requirements

- [x] Character select uses `CharacterSelectNavBar.xml`:55px bar,51px buttons, zero-spacing tray,4px padding, Lua measured label+70px widths and end-cap overrides.
- [x] Main menu uses `MainMenuFrameTemplates.xml`:260px width,48/34px vertical padding,28px side padding,zero spacing; `MainMenuFrameMixin:AddSection`20px. `DialogBorderTemplate` and `DialogHeaderTemplate` own art, not a second panel.
- [ ] Delete confirmation uses `GlueDialog` and `GlueDialogButtonTemplate` background, input and buttons.
- [ ] Campsite entries/journal use `WarbandSceneGlueTemplate`/`GlueWarbandSceneJournalTemplate` and inherited source templates.
- [ ] Options use `SettingsPanel`920x724, `SettingsCategoryListButtonTemplate`175x20, `SettingsCheckboxTemplate`, and `KeyBindingFrameBindingButtonTemplate`.
- [ ] Every finished frame has behavior/geometry tests and inspected both-skin before/after captures in `/syncthing/AgentShared/2026-10-13/ui-glue-menus/`.

## Sources

Local Retail AddOns under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`: `Blizzard_CharacterSelectNavBar/CharacterSelectNavBar.{xml,lua}`, `Blizzard_SharedXML/Mainline/Frame/MainMenuFrameTemplates.xml`, `Blizzard_SharedXML/Shared/Frame/MainMenuFrameTemplates.lua`, `Blizzard_SharedXML/Shared/Dialog/DialogTemplates.{xml,lua}`, `Blizzard_SharedXML/Shared/Button/ThreeSliceButtonTemplate.{xml,lua}`.

## Proof

`godot/ui-model/tests/{character_select,game_menu}.rs`: preserved actions, live hover/pressed selection, source geometry. `godot/tests/glue_menus_capture.gd`: offline production projections, both skins; captures are evidence, not automatic acceptance.

No extraction, auth, networking or canonical asset mutation. Shared dropdown/context-menu art belongs to `menu_primitives`.
