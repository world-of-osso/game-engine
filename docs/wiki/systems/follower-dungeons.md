# Native follower dungeon Group Finder

Retail Mainline's `LFDMicroButton` opens the native follower-mode `PVEFrame`/`LFDQueueFrame`. Modern and Forever share catalog selection and admission behavior. Full companion gameplay/data limits remain in the [server investigation](../../../../game-server/docs/wiki/investigations/follower-data.md), not this UI page.

## Catalog and admission

`QueryFollowerDungeons`/`FollowerDungeons` use reliable `LfgChannel`. Server publishes only imported difficulty205 entries with prepared population and an authored entrance. Client keeps IDs/names/maps from that snapshot; absent/stale choices cannot queue. `QueueForLfg` sends the selected role and single dungeon. `LfgStateUpdate` reaches both the native view and retained IPC status; initial request acknowledgment is not successful admission. Pending requests/active matches disable duplicate admission. Refusal clears pending and remains visible. Reconnect destroys the native view/catalog.

## Native frames

`follower_dungeons_component.rs` constructs source role icons/checks, follower type dropdown, description,20px checkbox list, MinimalScrollBar and Find Group button. `RegistryUi` supplies actual input, skin resolution and existing HUD parent. `follower_dungeons.rs` hosts the screen, routes actions and catalog/state events, and retains the existing PvP tab. Raid Finder/Premade Groups are not implemented by this slice.

## Proof boundary

Targeted RED reproduces missing catalog response, default-bridge UDP catalog delivery and unhandled native account catalog; both-skin registry tests reproduce missing controls/selection/match handling. Engine0746dbca5 passes six targeted registry/UDP/account tests and the helper's library checks; server2d62a2dc passes the new catalog loopback and existing companion/LFG tests (22 plus the Mythic Mentor companion test with its required read-only database environment). All three format checks pass. New both-skin live UI proof remains pending. Earlier CLI/first-boss evidence does not establish this new UI or complete dungeon navigation.

## Sources

- Local Retail `Blizzard_GroupFinder/Mainline/{PVEFrame,LFDFrame,LFGFrame}.xml/.lua` — frame dimensions/anchors, role/type/list controls, follower filter and queue behavior.
- Local `Blizzard_SharedXMLBase/TextureUtil.lua` — `UI-LFG-RoleIcon-*` atlas names.
- Local `data/community-listfile.csv` — source dungeon-wall337489, checkbox130755/check130751 and Group Finder art IDs.
- [Contract](../../specs/follower-dungeons.md) — behavior and acceptance boundary.
- [Server catalog](../../../../game-server/crates/server/src/lfg/follower_catalog.rs) — prepared source-backed entries.

## See Also

- [[battlegrounds]] — retained PvP tab and queue behavior.
