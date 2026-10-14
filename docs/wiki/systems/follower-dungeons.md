# Native follower dungeon Group Finder

Retail Mainline's `LFDMicroButton` opens the native follower-mode `PVEFrame`/`LFDQueueFrame`. Modern and Forever share catalog selection and admission behavior. Full companion gameplay/data limits remain in the [server investigation](../../../../game-server/docs/wiki/investigations/follower-data.md), not this UI page.

## Catalog and admission

`QueryFollowerDungeons`/`FollowerDungeons` use reliable `LfgChannel`. Server publishes only imported difficulty205 entries with prepared population and an authored entrance. Client keeps IDs/names/maps from that snapshot; absent/stale choices cannot queue. `QueueForLfg` sends the selected role and single dungeon. `LfgStateUpdate` reaches both the native view and retained IPC status; initial request acknowledgment is not successful admission. Pending requests/active matches disable duplicate admission. Refusal clears pending and remains visible. Reconnect destroys the native view/catalog.

## Native frames

`follower_dungeons_component.rs` constructs source role icons/checks, follower type dropdown, description,20px checkbox list, MinimalScrollBar and Find Group button. `RegistryUi` supplies actual input, skin resolution and existing HUD parent. `follower_dungeons.rs` hosts the screen, routes actions and catalog/state events, and retains the existing PvP tab. Raid Finder/Premade Groups are not implemented by this slice.

## Retail template layout repair — 2026-10-13

The initial layout used generic panel-button tabs and incorrectly stretched dungeon-wall337489. The repair uses shared merchant PanelTabButtonTemplate art, all three source tab anchors, PVEFrame blue-menu crops and InsetFrame borders, native category ring/icon/backdrop and shared WowStyle1 dropdown/menu. Native quest paper337492 keeps512×256 at BOTTOMLEFT6,26; the projection clips `LFDParentFrame` and `LFDParentFrameInset`, preserving source dimensions rather than rescaling texture art. Disabled categories retain source backdrop UVs and desaturated ring/icon; category icons reuse the native portrait-mask composer with XML66px icon/62px mask inset2.

`follower_layout_capture.gd` captures the actual RegistryUi projection with an offline Ruby catalog snapshot, without server admission. Its geometry gate observes rendered global Control rectangles at1920×1080 against XML/Lua values, including the no-scenarios Lua101px/30px category anchors. Before captures and RED reproduce456px outer height instead of428px. PNG evidence goes to `/syncthing/AgentShared/2026-10-13/group-finder-layout/`; source logs stay in the private follower run directory. Rendered geometry/pixel GREEN and XML-side inspection pass at code5b6f57417: both skins retain native paper size/clipping, all category/tab anchors, readable overlapping-tab labels and description text above parchment. Initial inspection exposed useParentLevel/layer ordering; a real pixel RED reproduces the covered description before the source-layer fix. Targeted UI/micro5/5, native-mask regressions5/5, extension build/library checks and both-package fmt pass. Three missing unrelated talent masks in the stale private overlay were refreshed from authentic current read-only files before native-mask GREEN. No gameplay/admission rerun is claimed.

Eight full1920×1080 PNGs (`before/after` × `Modern/Forever` × `closed/open dropdown`) are published there; private command/revision/asset/binary hashes are recorded in `group-finder-layout/receipt.json`. New source UI files337492/517165/593917/593919/341547/922034 were extracted locally only into the retained private texture overlay; canonical publication remains lead-owned.

A subsequent LFDFrame.lua:382–424 review caught a remaining menu-shape mismatch: CreateRadio was incorrectly represented as the shared helper's title/divider/button menu. Native RED observes76px rather than the source one-row43px menu. The incoming shared `context_menu_with_rows` API composes the existing radio element; heading-aware shared row geometry and omission of an empty heading preserve ordinary titled button menus and Options radio consumers. Post-master code46cdad1cf passes all12 targeted follower/menu registry cases, extension build/library checks and both-package fmt. Updated native geometry/pixel gate passes both skins closed/open at1920×1080, including43px headerless menu and its selected radio tick; all four after PNGs were replaced and inspected against XML/Lua. This supersedes the earlier titled-popup captures without changing admission/gameplay proof. The single merge-conflict compile probe was a harness ordering error, not a behavioral RED; actual radio RED is `radio-menu-red.log`. The source-decimal parser also dropped the category highlight's bare.8 opacity to0; `category-highlight-red.log` reproduces the actual CanvasItem alpha. The authored fraction now uses the maintained string-attribute representation, and the native fixture checks.8 rather than only presence of a highlight node. Final codeb11d6ba28 passes12 targeted cases/build/check/fmt and all four rendered geometry/pixel/radio/highlight cases; final after PNGs are inspected and published. This section does not supersede the admission or full-gameplay proof boundary.

## Binding role-template repair — 2026-10-13

Verifier item5 reproduced missing solo Leader in `role-templates-red.log`. Codecf9ea8f1b restores its48×48 button at DPS RIGHT+32 (queue274,46), source disabled Leader atlas and unavailable checkbox hiding. Retail Shared/LFGFrame.lua:457 enables the guide for solo players/current group leaders and disables it for grouped non-leaders; guide selection does not change the combat role or invent a wire field. The lead's original solo-disabled instruction was superseded by a binding source-governed correction, reproduced by two actual registry RED cases (`solo-leader-red.log`). Tank/Healer/DPS now inherit centered80×80 BACKGROUND atlases from LFGRoleButtonWithBackgroundTemplate. Leader inherits LFGRoleButtonTemplate directly; TextureUtil.lua supplies no GUIDE background, so none is invented.

Post-master14/14 targeted follower/menu cases, extension build/library checks and both-package fmt pass (`role-final-gates.log`, `role-fmt-check.log`). Four native1920×1080 captures pass rendered role/background rectangles, solo Leader availability/checkbox, existing tab/paper/radio/highlight geometry and description-pixel checks (`role-native-green.log`). All four replacement after PNGs were inspected against XML/Lua and published under `/syncthing/AgentShared/2026-10-13/group-finder-layout/`. Private `receipt.json` retains binary and image hashes. Manual changed-Rust readability inspection found no new warning suppression or overnested logic. Admission/full gameplay proof remains unchanged.

Corrected solo-enable source e63432861a passes14 targeted follower/menu cases, extension build/library checks and fmt (`solo-leader-final-gates.log`, `solo-leader-fmt-check.log`). Two preceding registry RED cases prove the incorrect solo-disabled policy (`solo-leader-red.log`). Both skins now render the enabled solo guide/check; grouped non-leader disablement and selection refusal are behaviorally asserted. All four recaptured1920×1080 after PNGs pass native role/background/checkbox geometry and existing gates, are visually inspected and replace the published after files (`solo-leader-native-green.log`). This supersedes the original solo-disabled captures without changing gameplay acceptance.

## Proof boundary

Targeted RED reproduces missing catalog response, default-bridge UDP catalog delivery and unhandled native account catalog; both-skin registry tests reproduce missing controls/selection/match handling. Engine0746dbca5 passes six targeted registry/UDP/account tests and the helper's library checks; server2d62a2dc passes the new catalog loopback and existing companion/LFG tests (22 plus the Mythic Mentor companion test with its required read-only database environment). All three format checks pass. Both-skin native admission proof is bounded below; full dungeon and class/gear acceptance remain open. Earlier CLI/first-boss evidence does not establish this new UI or complete dungeon navigation.

## Both-skin native admission — 2026-10-13

Live engine75d55239a/servera58abcc5/protocol8c6d4a2, matching helper-built extension/CLI/server/admin. Private UDP5761, fresh redb, consistent read-only world snapshot, `fb_follower_dungeons`; Human Mage12 characters30Follmodern/31Follforever, un-geared native stats. Owned Modern/Forever layout files select their skins and expose the existing micro menu.

Actual native JS input opens `LFDMicroButton`, selects follower type, Damage role and Ruby2436, then clicks Find Group. Authoritative replies list the player plus Garrick Tank, Crenna Healer and Shuja/Meredy Damage; all five online and map2521 terrain9/9 loaded. Both runs physically move14m to1400.2/-169.7; all four NPCs follow to1395.3/-169.7. No admin teleport, HP priming or combat buffs were used. This proves admission and bounded follow, not combat/navigation/full-instance completion.

Three inspected1280px PNGs: `/syncthing/AgentShared/2026-10-13/follower-dungeons/{modern-group-finder,forever-group-finder,forever-follower-admission}.png`. Private logs/rosters/terrain/UI/argv/process identities: `/home/osso/.worktrees/follower-dungeons-2026-10-13/{modern,forever}/`. The admission PNG proves UI/roster only: camera/geometry presentation is poor, and two model receipts, one texture receipt plus belt/shoulder Bone[0] bindings remain real world/gear render gaps. Do not promote it to full companion/world pixel proof.

Initial private appearance snapshot lacked current `uses_display_geosets`; refreshing only private SQLite from current authenticated source fixed schema drift without guessed defaults or runtime bypass. The first wait targeted hidden portrait-party frames; the observed active `CompactPartyFrameMember1` corrected the private script. All owned clients/server stopped after captures; no canonical data writes or shared-realm operations. Post-master CPU proof is server22/22 and engine19/19 plus checks/fmt; no full suite.

## Sources

- Local Retail `Blizzard_GroupFinder/Mainline/{PVEFrame,LFDFrame,LFGFrame}.xml/.lua` — frame dimensions/anchors, role/type/list controls, follower filter and queue behavior.
- Local `Blizzard_SharedXMLBase/TextureUtil.lua` — `UI-LFG-RoleIcon-*` atlas names.
- Local `data/community-listfile.csv` — source quest-paper337492, blue-menu593918/593919/593917, role background517165, checkbox130755/check130751 and Group Finder art IDs.
- [Contract](../../specs/follower-dungeons.md) — behavior and acceptance boundary.
- [Server catalog](../../../../game-server/crates/server/src/lfg/follower_catalog.rs) — prepared source-backed entries.

## See Also

- [[battlegrounds]] — retained PvP tab and queue behavior.
