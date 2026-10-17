# Retail gameplay-panel fidelity

Retail XML/Lua under `Blizzard_*/retail/AddOns` defines controls, sizes, anchors and art for both skins; Forever swaps art only. Preserve existing reducers and authoritative decisions. Use shared panel tabs, dropdown/menu elements and MinimalScrollBar; never substitute defaultbutton rows or solid popup fills. Atlas-coordinate drift is a named data gap, not permission to hand-correct coordinates.

## UIErrors message hold

`Blizzard_UIErrorsFrame/Mainline/UIErrorsFrame.xml:3` specifies displayDuration2 and fadeDuration.5. Existing error/info messages stay opaque through2 seconds, begin fading afterward and disappear at2.5 seconds. Repeated messages restart that hold without adding a line. Preserve current text, color, ordering and geometry. The XML's fadePower3 curve, throttle/flash behavior, typography/shadow and temporary caller reanchors remain unqualified; this timing slice must not claim complete MessageFrame parity.

## Shared UIPanel buttons

`quest_art::panel_button` emits `UIPanelButtonNoTooltipTemplate`/`UIPanelButtonTemplate` art from `Blizzard_SharedXML/SecureUIPanelTemplates.xml:39–78` and its mouse-state Lua: UI-Panel-Button-Up/Down/Disabled strips and the additive highlight. Keep the authored rectangle, label and existing enabled/action decisions in both skins. Callers must not require postsetup texture repairs; other button template families retain their own art.

## Shared tiled window background

Portrait/ButtonFrame rock backgrounds follow Mainline `SharedUIPanelTemplates.xml:598–609,663–675`: one opaque region from TOPLEFT(2,−21) to BOTTOMRIGHT(−2,2), horizontally and vertically tiled from that shared origin. `_UI-Frame-TopTileStreaks` retains its source TOPLEFT(6,−21)/RIGHT−2,43px height and horizontal-only repeat (:76–78). Keep existing border, portrait, title, controls and behavior; no stretched source picture or restarted tile phase at a split inset.

## Acceptance

- [x] Customer orders: Place Order/My Orders PanelTabButtonTemplate and recipient/duration WowStyle1DropdownTemplate use authored anchors, pending state and existing actions. Personal orders show Order To; payment labels stay inside the right pane and share input centerlines. Back uses UIPanelButtonTemplate sheet art; the portrait shows the interacting NPC. Form.lua overrides recipient/duration widths to136/143px. [Crafting contract](crafting-ui.md).
- [ ] Gameplay menus: damage-meter radio menus, minimap tracking checkboxes, transmog source/completion filters and unit difficulty submenus use Retail menu backdrop/elements, source20px rows and measured title/label extents. [Shared menu contract](menu-primitives.md).
- [ ] Talent loadout dropdown/menu/dialog, actual talent choice entries, cooldown settings and Edit Mode LayoutDropdown use their named Retail templates without replacing supported behavior.
- [ ] Guild rank controls, scoreboard tabs, rated/casual arena rows and pet selection buttons match their Retail controls.
- [ ] Raid Info list uses MinimalScrollBar instead of arrow action buttons.

Each finished group requires behavior tests, XML-geometry assertions, native1920×1080 before/after captures in both skins, and inspection alongside the corresponding XML. Source screening alone is not visual acceptance. Each landable report names its commit group; incomplete groups remain explicit.

## Cooldown settings source boundary

`Blizzard_CooldownViewer/CooldownViewerSettings.xml` supplies the399×609 shell, LargeSideTab controls,38×38 icons,317×38 bars, pixel scroll viewport and Revert UIPanelButtonTemplate. Settings preserve per-specialization choices/order and a reopen restore point; the native fixture physically picks/drops, assigns visibility, reloads from disk and reverts in both skins. Modern Revert projects the authored sheet strips; Forever changes art only.

Named gaps: GroupBuffs has no current category data; named layouts, alerts, cross-category assignment and the unlearned/reset settings menu have no backing state. Disabled controls do not establish parity. Published atlas textures now load cleanly. Both-skin physical SearchBox typing/clear plus reorder/assignment/reload/Revert pass; four refreshed1920×1080 open/hidden captures under `AgentShared/2026-10-13/ui-gameplay-panels/after-review/` are inspected. Retrospective before captures use unmodified dc7c36424 renderers in the current native host; temporary renderers are removed after capture. Final integration verification remains open.

## Capture provenance (2026-10-13)

`AgentShared/2026-10-13/ui-gameplay-panels/before-review/` contains both-skin1920×1080 HUD/cooldown baselines from dc7c36424 and raid/scoreboard/rated/casual baselines from642fcee41. These are archived control renderers in the current native host/data, **not complete historical client builds**. PvP uses its original host SharedContext so physical wheel state reaches the old skirmish controls. Temporary baseline code is removed after capture; source and recipes remain in the private proof folder.

Inspected after captures in `after-review/` include actual talent choices, HUD menu/naming dialog, cooldown open/hidden states and PvP rated/Specific/skirmish rows. Native HUD/cooldown fixtures assert exact rendered rectangles alongside physical actions and file reload. Source gaps remain explicit; captures of disabled or missing-data controls do not establish supported behavior.

## Communities guild portrait

CommunitiesFrame PortraitOverlay follows Mainline XML328–353 and GuildUtil.lua115–127,180–202. Guilds without a registered design show GuildLogo-NoLogo460904 at64×64 (not the designed56×64 width), white full-file UVs; GuildFrame410251 background/border use the source UVs and no-design colors. No fabricated emblem data. Real persisted design, vendor registration, snapshot appearance and equipped-tabard rendering are the subsequent guild feature.

## Proof owners

- [Customer-order source/control proof](../wiki/systems/crafting-ui.md#customer-control-fidelity--verified-2026-10-13).
- [Shared menu rendering and source slicing](../wiki/systems/menu-primitives.md).
- `godot/tests/gameplay_panels_capture.gd` and native offline previews mount production screens without server mutation.
- `godot/ui-model/tests/{professions_orders,menu_primitives,minimap_tracking,transmog_wardrobe,forever_chat_meter,talent_footer,battleground}.rs` exercise changed controls and observable actions.
