# Extra Action Bars

Native Options visibility for existing Action Bars 2–5 in both skins. Retail source: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_SettingsDefinitions_Frame/ActionBars.lua:29–47,86–91,127` registers Gameplay > Action Bars, labels `OPTION_SHOW_ACTION_BAR:format(n)` in 2–8 order, defaults false; `Blizzard_ActionBar/Shared/MultiActionBars.xml:104,133` hides side bars initially. [HUD edit mode](hud-edit-mode.md#action-bars) owns preset geometry and documented skin deviations.

## What it must do

- [x] Options has an Action Bars category with checkboxes Action Bar 2, 3, 4, 5 in Retail numeric order. No unsupported 6–8 controls.
- [x] Bars 4/5 default disabled in both gameplay and edit mode, with no registered mover or placeholder. Enabling both adds two movers (offline defaults 19 → 21).
- [x] Changes immediately show/hide production bars and edit-mode movers, persisted in the existing `options_settings.ron` HUD settings store. Older files lacking extra visibility retain side bars disabled.
- [x] Preserve unset bottom-bar defaults: Modern hidden in gameplay/editor previewed, Forever visible (approved reference deviation). Explicit checkboxes override them; category Defaults clears only extra-bar choices.
- [x] Enabled 4/5 retain existing vertical 12-slot defaults: right insets 6/53; Modern 45×562, Forever 47.7×595.72 UI units before pixel snapping. Tracker placement unchanged when enabling/disabling. Overlap is allowed by user decision 2026-10-08, not a reason to move tracker.

## How it works

- [Native HUD editor](../wiki/systems/native-hud-edit-mode.md)

## Implementation inventory

- `godot/core/src/client_options_data.rs` — serialized extra-bar choices.
- `godot/ui-model/src/ui/options_menu_data.rs` — toggle/commit/reset policy.
- `godot/ui-model/src/ui/screens/options_menu_{component,active_sections}.rs` — category and checkboxes.
- `godot/ui-model/src/ui/screens/main_action_bar_component.rs` — production bars 1–5 and visibility.
- `godot/rust/src/spells/action_bar.rs` — option snapshot to native bar state, side pages 3/4 and pointer casts.
- `godot/rust/src/ui/hud_edit_preview.rs` — editor and Options capture fixtures.

## Tests asserting this spec

- `godot/rust/src/ui/hud_edit_tests.rs` — actual checkbox actions, default/enabled/disabled gameplay/movers, both-skin rectangles and unchanged tracker.
- `godot/rust/src/ui/hud_edit_polish_tests.rs` — 19 default movers, manager/label clearance.
- `godot/ui-model/tests/options_policy.rs` — supported toggles, snapshots and category reset.
- `godot/ui-model/tests/options_views.rs` — category/action rendering.
- `godot/ui-model/tests/forever_quest_windows.rs` — Modern quest windows remain unchanged; combined quest/options fixture includes the added Action Bars tab and page.
- `godot/core/tests/extra_action_bars_options.rs` — file round trip and absent-field defaults.

## Known gaps (current cycle)

No remaining gap in this bounded visibility change. At implementation `67c50ec42`: RED HUD edit 16 passed/2 failed and Options 0 passed/1 failed; GREEN HUD edit 19, HUD layout 30, five scoped UI integration files 49 (one ignored capture helper), settings store 1, all zero failures. Extension build exits 0. Native cage captures `options-action-bars.png` and `forever-edit-defaults.png` each exit 0; inspected 1280×720 compositor output (recipe requests 1920×1080). Options shows 2–5 in order, all unchecked under Modern defaults; Forever editor has no right-side 4/5 boxes, tracker remains beneath minimap. Evidence: canonical `data/diagnostics/extrabars-2026-10-08/`, including RED/GREEN/build logs and `proof-ledger.json`. No whole-crate suite or live-server casting claim.

## Out of scope

- Bars 6–8: no existing native consumer; do not invent them.
- Side-bar keyboard binding definitions, drag assignment, action paging UI: separate features. Existing side-page snapshots and pointer casting are reused.
- Retail's Gameplay category grouping: native Options retains its existing flat category list.
- Tracker relocation: explicitly excluded by user decision.
