# Native raid difficulty and Raid Info

Verified: 2026-10-11. Both skins share Retail controls and requests. [Contract](../../specs/raid-entry.md); [server behavior/data derivation](../../../../game-server/docs/wiki/systems/raid-entry.md) is the source of truth for admission, credit and unresolved bindings.

## Content

Player portrait right-click now opens Dungeon Difficulty and Raid Difficulty submenus. Raid offers Normal/Heroic/Mythic then Legacy Raid Size10/25. Retail `SetRaidDifficulties` maps primary Normal/Heroic onto the retained legacy size; Mythic disables legacy selection. Inside-instance and non-leader entries are disabled; server independently validates authority. Background catalog results must be polled even while Raid Info is closed, otherwise every menu entry remains disabled.

`/raidinfo` opens the saved-instance frame and requests InstanceInfo. Local pinned Map/Difficulty/MapDifficulty CSVs supply names and extension flags. Rows retain instance IDs, completed masks, relative reset countdown, expired state and extension marker. Selection enables Extend, Cancel Extension or Reactivate; responses replace the authoritative snapshot. Expired snapshot timers trigger one refreshed request, not one per frame. Close/Escape dismisses the frame. Lists longer than three rows scroll; tests include a fourth save. Forever only changes existing shared chrome/font/art presentation.

### Retail scrollbar ownership — 2026-10-13

`RaidInfoFrameSocialView.xml` places the292×157 ScrollBox at(16,48) and the151px scrollbar beside it at(318,50). Putting the scrollbar inside the ScrollBox clipped away its thumb despite passing model scroll tests. Native capture regression reproduced the clipped8×55 thumb at(678,209); sibling placement passes both-skin1920×1080 clipping checks. Native controller association must also resolve those sibling thumb/track/stepper names to the registered list, and GameClient must route Raid Info pointer events. `ScrollBoxListViewMixin:GetPanExtent` derives the40px saved-row extent; wheel scalar2 gives80px steps. The native regression requires dragging to the eighth save, not merely setting a registry offset.

Source/control code: [saved-list geometry](../../../godot/ui-model/src/raid_info.rs), [native scrolling](../../../godot/rust/src/ui/scroll_lists.rs), [native drag regression](../../../godot/rust/src/ui/raid_scroll_tests.rs), [render clipping fixture](../../../godot/tests/gameplay_panels_capture.gd). The same clipped-parent pattern was removed from the cooldown settings scrollbar. Current captures: `/syncthing/AgentShared/2026-10-13/ui-gameplay-panels/after-review/{modern,forever}-menus-raid.png`; no new server authority is implied.

### Portal ownership bug

Native map0→Molten Core portal replay intermittently left the client Loading(409) while terrain was fully loaded for Azeroth, with zero pending assets/failures. Periodic LoadTerrain hints from the departed map could overwrite NewWorld's destination. Terrain refresh now bootstraps an empty world or refreshes the selected map; a mismatching initialized-world hint is rejected explicitly. Only NewWorld replaces an initialized world. Native post-fix replays in both skins reached InWorld409 with no pending port; stale Azeroth hint rejection was observed in the log. Temporary traces were retired; `account_state` exposes literal world_map_id/pending_world_port for proof.

### Evidence boundary

Private UDP5760, two disposable accounts, both rendered skins. Native invitation/raid conversion, difficulty mapping, saved-instance selection/extension/cancellation/reactivation and weekly-reset states were observed. Server restart preserved copy1/mask1; controlled authentic boss death advanced both saves to mask3. Ordinary combat and complete raid encounter mechanics are not claimed. Ten inspected PNGs: `/syncthing/AgentShared/2026-10-11/raid-entry/`; logs/snapshots/scripts remain in canonical engine `data/diagnostics/raid-entry-2026-10-11/`.

## Sources

- [Raid contract](../../specs/raid-entry.md)
- [Saved-row model](../../../godot/ui-model/src/raid_info.rs)
- [Native frame](../../../godot/rust/src/raid_info.rs), [unit menu](../../../godot/rust/src/unit_menu.rs), [account boundary](../../../godot/rust/src/account.rs)
- Local Retail Blizzard_UnitPopup/Mainline/UnitPopup.lua and Blizzard_RaidFrame/Mainline/RaidInfoFrameSocialView.lua/XML.

## See Also

- [[world-entry-stalls]] — world/terrain readiness; portal owner selection must survive stale refresh hints.
