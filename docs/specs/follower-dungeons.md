# Native follower dungeon Group Finder

Server gameplay contract: [follower dungeons](../../../game-server/docs/specs/follower-dungeons.md).

- Group Finder opens from `LFDMicroButton` into Retail Mainline `PVEFrame`/`LFDQueueFrame` follower mode. It requests the prepared server catalog, selects one authored entry and a tank/healer/damage role, and sends `QueueForLfg` on reliable `LfgChannel`. Unprepared entries are not invented or offered. The existing PvP tab remains available; Raid Finder and Premade Groups remain unavailable.
- The follower type dropdown, role icons/checks, source20px dungeon checkbox rows and ScrollBox/MinimalScrollBar use the same elements in Modern and Forever. Pending requests and active matches disable repeat admission; server refusals remain visible in the follower description and IPC status.
- Native `lfg queue <role> <dungeon IDs>` retains the same selected-role transport for diagnostics. Difficulty205 selects a follower dungeon in the imported server catalog.
- Queue-send acknowledgment is not server acceptance. `lfg status` exposes the latest authoritative snapshot, match roster and refusal; reconnect clears stale state.
- Existing LFG dequeue/role-check requests use the same transport. Completed follower admission is a running instance, not a human matchmaking queue entry.
- Both Modern and Forever show the same replicated followers and existing party frames; no skin-specific gameplay or invented UI frames.
- Local user-approved server workaround uses labelled-spec player spells and same-map Normal population/entrance; absent Rogue/DK/DH identities and full-rotation gaps remain explicit in the server contract.

## Retail layout fidelity

- `PVEFrame` retains XML563×428/TOPLEFT100,-84. All three tabs use the shared `PanelTabButtonTemplate` helper; Tab1 BOTTOMLEFT19,-30, Tab2/3 LEFT-to-RIGHT-16. Mythic+ remains unavailable in this follower slice.
- Category buttons use `GroupFinderGroupButtonTemplate`203×60, source224×80 bluemenu backdrop,95×96 ring,66px inset-masked icon and GameFontNormalLarge. Current no-scenarios anchors come from `PVEFrame.lua`101px/30px spacing, not the scenarios-enabled XML70px/23px branch.
- Source left inset, blue frame texture crops and the right inset remain native geometry. `UI-LFG-BACKGROUND-QUESTPAPER`337492 is512×256, BOTTOMLEFT6,26, clipped to the right inset; never stretched wall art337489.
- The type chooser uses shared Retail WowStyle1 dropdown/menu primitives in both skins. `LFDFrame.lua` creates a checked follower radio, without a menu title/divider; the one supported follower option must preserve that shape.

- Combat roles inherit centered80×80 background atlases behind their48×48 icons. Leader uses the base role template at DPS RIGHT+32 (274,46), with source disabled leader art and hidden checkbox when unavailable. Retail Shared/LFGFrame.lua:457 allows solo players and actual group leaders to toggle the guide check without changing the queued combat role; grouped non-leaders retain the disabled guide icon with its checkbox hidden. Solo admission remains server-authoritative; guide preference is not a new wire field.

## Tests

- `godot/tests/follower_layout_capture.gd` — native rendered Control rectangles from XML/Lua, all three tab anchors and parent clipping;1920×1080 Modern/Forever closed/open dropdown PNGs. `GODOT_FOLLOWER_LAYOUT_ASSERT=1` enables the geometry gate.
- `ui-model/tests/follower_dungeons.rs` — real both-skin registry, sourced catalog selection, selected-role queue actions, pending/refusal/match state and absent/stale-catalog refusal.
- `network/src/wire_tests.rs::follower_group_finder_default_bridge_receives_catalog_over_udp` — default bridge, real loopback UDP catalog delivery.
- `account_dungeonclient_tests.rs::follower_group_finder_account_dispatches_catalog_and_authoritative_state` — decoded catalog and LFG state reach the native host while IPC retains authoritative state.
- Private UDP5761 Group Finder admission now has both-skin proof: actual native microbutton/type/role/dungeon/Find Group clicks, authoritative five-member party and four companions following14m movement. [Exact proof and limits](../wiki/systems/follower-dungeons.md#both-skin-native-admission--2026-10-13). This does not prove combat, the walked route or dungeon completion.

- `account_dungeonclient_tests.rs::follower_queue_native_account_retains_server_rejection_and_match_state` — actual decoded account traffic.
- `follower_wire_tests.rs` — ephemeral UDP role/dungeon request and authoritative refusal round trip.
- Private UDP5750 Modern/Forever queue, correct five-role party, follow/combat and first Ruby boss kills now have live proof. [Exact proof/limits](../../../game-server/docs/wiki/investigations/follower-data.md#both-skin-first-boss-proof--2026-10-11) owns revisions, data receipts and evidence.
- Authored boss loot is absent and native walking did not complete the approach; admin reposition was used. These gates remain open, not end-to-end completion. Eight inspected PNGs: `/syncthing/AgentShared/2026-10-11/follower-ai/`.
