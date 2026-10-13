# Glue/menu fidelity

The five assigned native frames use Retail template geometry and art; Forever changes atlas resolution, not frame inventory. Contract: [glue/menu fidelity](../../specs/glue-menu-fidelity.md). Verified: 2026-10-13.

## Source geometry

- CharacterSelectNavBar:55px root,51px buttons; zero-spacing tray with4px width padding; GlueFontNormal15px measured label+70px. End caps and selected lines follow `SetButtonVisuals` offsets, not guessed fixed button widths.
- MainMenuFrame:260px width; padding48/34 vertically,28 horizontally; zero spacing,20px sections. Six36px buttons plus three sections yield358px. Dialog header uses measured text+64px and39px height at TOP+11.
- GlueDialog:512x256 base container,450px text region,150x32 editbox,200x30 buttons; input inherits TooltipDefaultLayout, not a Common-Input-Border stand-in.
- GlueCollections:703x595 at CENTER+94; IconsFrame(4,60)..(-6,-53), grid inset13/38 and12/25. The resulting668x419 region fits three215x173 cards with11px horizontal and34px vertical spacing, hence six per page. Selected224x183 art is TOPLEFT(-4,+5); source paging controls are21px above grid bottom.
- SettingsPanel:920x724,175x20 categories; SettingsCheckbox30x29. Keybinding buttons160x22 use the XML12x6 silver-button corner crops. Dropdown controls use SettingsDropdownControlMixin width220 and WowStyle2 height25.

## Atlas slicing, not stretched borders

`UiTextureAtlasElementSliceData` supplies margins missing from the old atlas export. Local FDID5319042 (retail12.1.5.70077), layout32F4C664,241 rows. WoWDBDefs commit `d6f50d4f351193e84226aa62a36faa0f58528afa`; DBD SHA256 `d274387e9021972eb5235f64e89ab1248c15718a3947aa0961c3763e4ef5a6a7`.

Dialog element30482 has32px margins; campsite normal29533 has12px, hover29531 has17px, selected29532 has18px. Crop coordinates are relative to the atlas region, not the full backing image. `scripts/tests/test_glue_atlas_slices.py` asserts real local DB2→CSV dialog/camp rows. No canonical data was committed.

## Popup ownership and native input boundary

Options stores one ephemeral `OptionsDropdownState` in its screen's SharedContext. The chosen popup is detached from the ScrollBox's declarative subtree and attached to OptionsRoot, with its anchor measured below the clicked control. Shared `menu_primitives` owns dropdown surfaces, backdrop, radio rows and geometry. Selection retains `options_toggle:key:value`; Escape consumes the popup dismissal, outside clicks dismiss without preventing normal routing.

Native RED found a separate root cause: WowStyle2's decorative Background is a child Button carrying art state, with no action, and intercepted its owner's clicks. Native projection now ignores pointer input for actionless `common-dropdown-*` art surfaces. The actual dropdown receives input; art still follows its owner. Capturing and inspecting the hovered control distinguished this from an Options reducer failure.

## Subpixel shadow crops

Retail CollectionsBackgroundTemplate samples the final atlas column/row with `0.9999..1` UVs. Native AtlasTexture rounded the 0.0013px/0.0145px strip widths to zero and reported the full512px sheet width instead. That leaked unrelated atlas art as gold bars behind collection content. Projection now samples these exact fractional UVs directly using the existing atlas shader; its half-pixel inset cannot exceed half the authored crop. No source coordinates or template geometry changed.

Native RED isolates the actual production small-shadow TextureRect on a gray background and detects120 colored samples from the grayscale strip. The same fixture gates both skins after the fix. Selected card gold borders remain intentional and distinct from this bleed.

## Proof and limits

- Source geometry/action tests: character_select6, game_menu2, options_views12, options_policy15, menu_primitives5; exporter1.
- `godot/tests/glue_menus_capture.gd`: both skins, actual native mouse clicks and source root sizes,12 cases.
- `godot/tests/options_dropdown_capture.gd`: native one-open replacement, selection forwarding, Escape retaining Options, outside-click dismissal; both skins.
- PNGs: `/syncthing/AgentShared/2026-10-13/ui-glue-menus/`. Offline projection proves UI/dispatch, not authenticated deletion or server-side campsite persistence.
- Missing source textures were extracted locally and published by the lead:4190639,3058483,383587,383585,383586,921208,1318750. No CDN or canonical cache edits.

## Sources

- Retail AddOns: `Blizzard_CharacterSelectNavBar/CharacterSelectNavBar.{xml,lua}`; `Blizzard_GlueCollections/GlueCollections.xml`, `GlueWarbandSceneCollection.xml`; `Blizzard_StaticPopup_Glue/GlueDialog.{xml,lua}`.
- Retail `Blizzard_SharedXML`: Mainline/Frame/MainMenuFrameTemplates.xml; Shared/Frame/MainMenuFrameTemplates.lua; Shared/Dialog/DialogTemplates.xml; Mainline/SharedUIPanelTemplates.xml; Mainline/NineSliceLayouts.lua; Mainline/SharedCollectionTemplates.xml.
- Retail `Blizzard_Settings_Shared`: Blizzard_SettingsPanel.xml, Blizzard_CategoryList.xml, Blizzard_SettingControls.{xml,lua}, Blizzard_Keybindings.xml.
- [Shared menu templates](../systems/menu-primitives.md) — source art ownership.

## See Also

- [[menu-primitives]] — shared dropdown/menu art.
- [[godot-conversion]] — native UI projection.
