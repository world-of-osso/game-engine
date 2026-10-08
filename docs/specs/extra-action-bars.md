# Extra Action Bars

Native Options visibility for existing Action Bars 2–5 in both skins. Retail source: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_SettingsDefinitions_Frame/ActionBars.lua:29–47,86–91,127` registers Gameplay > Action Bars, labels `OPTION_SHOW_ACTION_BAR:format(n)` in 2–8 order, defaults false; `Blizzard_ActionBar/Shared/MultiActionBars.xml:104,133` hides side bars initially. [HUD edit mode](hud-edit-mode.md#action-bars) owns preset geometry and documented skin deviations.

## What it must do

- [ ] Options has an Action Bars category with checkboxes Action Bar 2, 3, 4, 5 in Retail numeric order. No unsupported 6–8 controls.
- [ ] Bars 4/5 default disabled in both gameplay and edit mode, with no registered mover or placeholder. Enabling both adds two movers (offline defaults 19 → 21).
- [ ] Changes immediately show/hide production bars and edit-mode movers, persisted in the existing `options_settings.ron` HUD settings store. Older files lacking extra visibility retain side bars disabled.
- [ ] Preserve unset bottom-bar defaults: Modern hidden in gameplay/editor previewed, Forever visible (approved reference deviation). Explicit checkboxes override them; category Defaults clears only extra-bar choices.
- [ ] Enabled 4/5 retain existing vertical 12-slot defaults: right insets 6/53; Modern 45×562, Forever 47.7×595.82 UI units before pixel snapping. Tracker placement unchanged when enabling/disabling. Overlap is allowed by user decision 2026-10-08, not a reason to move tracker.

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
- `godot/core/tests/extra_action_bars_options.rs` — file round trip and absent-field defaults.

## Known gaps (current cycle)

- [ ] Targeted GREEN and native Options/default Forever edit-mode captures pending.

## Out of scope

- Bars 6–8: no existing native consumer; do not invent them.
- Side-bar keyboard binding definitions, drag assignment, action paging UI: separate features. Existing side-page snapshots and pointer casting are reused.
- Retail's Gameplay category grouping: native Options retains its existing flat category list.
- Tracker relocation: explicitly excluded by user decision.
