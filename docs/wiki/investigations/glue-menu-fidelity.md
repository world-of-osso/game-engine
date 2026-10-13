# Glue/menu fidelity

The five assigned native frames use Retail template geometry and art; Forever changes atlas resolution, not frame inventory. Contract: [glue/menu fidelity](../../specs/glue-menu-fidelity.md). Verified: 2026-10-13.

## Source geometry

- CharacterSelectNavBar:55px root,51px buttons; zero-spacing tray with4px width padding; GlueFontNormal15px measured label+70px. End caps and selected lines follow `SetButtonVisuals` offsets, not guessed fixed button widths.
- MainMenuFrame:260px width; padding48/34 vertically,28 horizontally; zero spacing,20px sections. The original six36px buttons plus three sections yielded358px; [menu follow-up](../../specs/game-menu-settings.md) exposes eight supported36px buttons and retains three sections, yielding430px. Dialog header uses measured text+64px and39px height at TOP+11.
- GlueDialog:512x256 base container,450px text region,150x32 editbox,200x30 buttons; input inherits TooltipDefaultLayout, not a Common-Input-Border stand-in.
- GlueCollections:703x595 at CENTER+94; IconsFrame(4,60)..(-6,-53), grid inset13/38 and12/25. The resulting668x419 region fits three215x173 cards with11px horizontal and34px vertical spacing, hence six per page. Selected224x183 art is TOPLEFT(-4,+5); source paging controls are21px above grid bottom.
- SettingsPanel:920x724,175x20 categories; SettingsCheckbox30x29. Keybinding buttons160x22 use the XML12x6 silver-button corner crops. Dropdown controls use SettingsDropdownControlMixin width220 and WowStyle2 height25.

## Atlas slicing, not stretched borders

`UiTextureAtlasElementSliceData` supplies margins missing from the old atlas export. Local FDID5319042 (retail12.1.5.70077), layout32F4C664,241 rows. WoWDBDefs commit `d6f50d4f351193e84226aa62a36faa0f58528afa`; DBD SHA256 `d274387e9021972eb5235f64e89ab1248c15718a3947aa0961c3763e4ef5a6a7`.

Dialog element30482 has32px margins; campsite normal29533 has12px, hover29531 has17px, selected29532 has18px. Crop coordinates are relative to the atlas region, not the full backing image. `scripts/tests/test_glue_atlas_slices.py` asserts real local DB2→CSV dialog/camp rows. No canonical data was committed.

## Popup ownership and native input boundary

Options stores one ephemeral `OptionsDropdownState` in its screen's SharedContext. The chosen popup is detached from the ScrollBox's declarative subtree and attached to OptionsRoot, with its anchor measured below the clicked control. Shared `menu_primitives` owns dropdown surfaces, backdrop, radio rows and geometry; `menu_row_rect` computes the same source inset/pitch for ordinary and custom radio entries. Selection retains `options_toggle:key:value`; Escape consumes the popup dismissal, outside clicks dismiss without preventing normal routing.

Native RED found a separate root cause: WowStyle2's decorative Background is a child Button carrying art state, with no action, and intercepted its owner's clicks. Native projection now ignores pointer input for actionless `common-dropdown-*` art surfaces. The actual dropdown receives input; art still follows its owner. Capturing and inspecting the hovered control distinguished this from an Options reducer failure.

## Subpixel shadow crops

Retail CollectionsBackgroundTemplate samples the final atlas column/row with `0.9999..1` UVs. Native AtlasTexture rounded the 0.0013px/0.0145px strip widths to zero and reported the full512px sheet width instead. That leaked unrelated atlas art as gold bars behind collection content. Projection now samples these exact fractional UVs directly using the existing atlas shader; its half-pixel inset cannot exceed half the authored crop. No source coordinates or template geometry changed.

Native RED isolates the actual production small-shadow TextureRect on a gray background and detects120 colored samples from the grayscale strip. The same fixture passes both skins after the fix (zero colored samples). Selected card gold borders remain intentional and distinct from this bleed.

## Proof and limits

- Source geometry/action tests: character_select6, game_menu2, options_views12, options_policy15, menu_primitives5; exporter1.
- `godot/tests/glue_menus_capture.gd`: both skins, actual native mouse clicks and source root sizes,12 cases.
- `godot/tests/options_dropdown_capture.gd`: native one-open replacement, selection forwarding, Escape retaining Options, outside-click dismissal; both skins.
- PNGs: `/syncthing/AgentShared/2026-10-13/ui-glue-menus/`. Offline projection proves UI/dispatch, not authenticated deletion or server-side campsite persistence.
- Missing source textures were extracted locally and published by the lead:4190639,3058483,383587,383585,383586,921208,1318750. No CDN or canonical cache edits.

## Capture receipt

All five frames accepted at `c4d0a075d` after merging master `285c1da80`; source geometry and actions inspected against the XML above. PNG root: `/syncthing/AgentShared/2026-10-13/ui-glue-menus/`. Each filename below has `modern` and `forever` variants; original `before-*` captures retained for comparison.

| Frame | Source implementation SHA | Final PNG stem |
|---|---|---|
| Character-select nav | `5dfe1a878` | `final-nav-{skin}.png` |
| Main menu | `3a9678803` | `final-menu-{skin}.png` |
| Delete dialog | `5dfe1a878` | `final-delete-{skin}.png` |
| Campsite collection | `c69a81a73`, native crop `f01da0067` | `final-campsite-{skin}.png` |
| Settings/Options | `c69a81a73`, shared row geometry `c4d0a075d` | `final-options-{graphics,keybindings,radio}-{skin}.png` |

Passive dropdown hit-testing is isolated in `b633ba1e8` (projection only), native Background click assertion in `9363df7c7`; `context_menu_with_rows` first appears in `cc33a79fa`.

Final proof:40 focused Rust tests, exporter1, extension build/check and scoped Rust format check; native12 frame geometry/click cases, two isolated shadow-pixel cases, and two popup lifecycle cases. Logs under `/home/osso/.worktrees/logs/ui-glue-menus-final-*`. Source-only independent verifier passed the bounded popup/input/sampling changes; native execution and pixel inspection remain main-thread evidence.

A stale slot-local shared crate artifact initially hid already-exported Renown messages. Cleaning only the slot's shared artifacts and rebuilding resolved it; protocol source was not changed. Disabled/tiled desaturation material composition is outside the assigned frames and was not expanded into this patch.

## Menu follow-up — 2026-10-13

`fbf1a1ea0` adds supported Retail Game Menu entries: Edit Mode enters the existing HUD editor; Macros/Support open existing Options pages; AddOns opens its category set. Native admission now checks an actual playable InWorld session. No store/splash/ratings screen was invented; exact hidden capability/localization conditions and remaining macro/support functionality live in [the contract](../../specs/game-menu-settings.md).

Settings Game/AddOns tabs reuse `merchant_frame_component::{tab,tab_width}` with MinimalTab.xml art/37px height and MinimalTabMixin text+40 width. Header anchors are(32,27), preceding TOPRIGHT+5, and SearchBox350x22 at(552,34). Search projects matching rendered labels/descriptions across the active category set, preserving each row's controls and grouping categories; empty results are explicit. Search/category-set changes reset content scroll. Actual SearchBox borders use InputBoxVisualTemplate cap8x20, left-5, right0; text insets are left16/right20.

Model RED caught missing entries/tabs and unfiltered categories. `fbf1a1ea0` passed32 scoped options and two menu tests, including both-skin header geometry. Native SearchBox RED exposed font12 expanding22px height to23px; `32526824e` uses the template's Small font10 and renders22px. Build at `f3c50ae17` and `godot/tests/ui_menus_followup.gd` now pass eight1920x1080 captures, both-skin source anchors and actual menu/tab/clear clicks. Images were viewed against SettingsPanel.xml/MinimalTab.xml/GameMenuFrame.lua. PNGs: `/syncthing/AgentShared/2026-10-13/ui-menus-followup/{menu,settings,addons,search}-{modern,forever}.png`. Log: `/home/osso/.worktrees/logs/ui-menus-followup-capture-final.log`. Offline actions prove dispatch, not authenticated HUD editor persistence. Supporting tabs/search textures1318750/8086468 and6725697/3281887 already exist in shared data; no extraction was needed.

Inherited scroll proof distinguished row retention from control visibility. `56a439713` reproduces the Action Bar2 RED and proves its first22px button is fully clipped while its last button is fully inside the529px viewport; the first34px row legitimately retains a1px edge strip. `4d678d3bd`/`f3c50ae17` prove a first wheel notch clamps to the list's end when the pan exceeds remaining overflow. No production scroll workaround was added. After merging origin/master `f8ac5c70c`, `19d4c3cee` drives Graphics through actual registry scrolling and the production wheel entry point in both skins: positive overflow, first_row/projection offset equality, initially clipped last setting, fully visible last setting at bottom, and wheel restoration to0. All13 targeted native scroll tests pass; the inherited Graphics model fixture also passes through actual registry scrolling, with no injected end offset. Prior36 passing model/menu tests plus this corrected Graphics result cover37 tests (one diagnostic capture test intentionally ignored). Native input/captures remain valid: later changes are tests/docs and master's Filter-only resize flag does not affect WowStyle2. Scoped extension check/format proof is recorded in the follow-up handoff.

## Sources

- Retail AddOns: `Blizzard_CharacterSelectNavBar/CharacterSelectNavBar.{xml,lua}`; `Blizzard_GlueCollections/GlueCollections.xml`, `GlueWarbandSceneCollection.xml`; `Blizzard_StaticPopup_Glue/GlueDialog.{xml,lua}`.
- Retail `Blizzard_SharedXML`: Mainline/Frame/MainMenuFrameTemplates.xml; Shared/Frame/MainMenuFrameTemplates.lua; Shared/Dialog/DialogTemplates.xml; Mainline/SharedUIPanelTemplates.xml; Mainline/NineSliceLayouts.lua; Mainline/SharedCollectionTemplates.xml.
- Retail `Blizzard_Settings_Shared`: Blizzard_SettingsPanel.xml, Blizzard_CategoryList.xml, Blizzard_SettingControls.{xml,lua}, Blizzard_Keybindings.xml.
- [Shared menu templates](../systems/menu-primitives.md) — source art ownership.

## See Also

- [[menu-primitives]] — shared dropdown/menu art.
- [[godot-conversion]] — native UI projection.
