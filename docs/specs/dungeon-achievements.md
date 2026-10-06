# Dungeon objectives and achievement browsing

## What it must do

- Show the active dungeon name and ordered boss objectives before watched quests, without moving the approved tracker anchor, in Modern and Forever.
- Replace boss state from server snapshots; check defeated bosses, label only verified optional bosses, clear on transfer/leave/disconnect, reject other-map/copy snapshots.
- Open Achievements from the launcher, micro-menu and configured ToggleAchievements binding; close with its button or Escape's CloseAllWindows.
- Browse authored category parents/order, achievement icons/names/descriptions/points/earned dates and criterion progress. Preserve unknown dates and unevaluated criteria.
- Request cursor pages only on opening or browsing; cache received pages in SharedContext. Refresh the visible category on live achievement updates and show earned toasts.
- Show earned points as a bare, thousands-separated number with Retail's adjacent shield in the window header in both skins.
- Use local Retail achievement art in Modern and the Forever panel family in Forever. No search or comparison.

## Tests

`godot/ui-model/tests/dungeonclient.rs` exercises Stockade entry, ordered kills and leave under both skins; category/achievement/criteria cursors, earned achievement 633 and launcher requests. Account tests exercise protocol dispatch. Offline capture entry points use `godot/tests/capture_ui_screen.gd`.

## How it works

[Client state and sources](../wiki/systems/dungeon-achievements.md).
