# Native follower dungeon Group Finder

Server gameplay contract: [follower dungeons](../../../game-server/docs/specs/follower-dungeons.md).

- Group Finder opens from `LFDMicroButton` into Retail Mainline `PVEFrame`/`LFDQueueFrame` follower mode. It requests the prepared server catalog, selects one authored entry and a tank/healer/damage role, and sends `QueueForLfg` on reliable `LfgChannel`. Unprepared entries are not invented or offered. The existing PvP tab remains available; Raid Finder and Premade Groups remain unavailable.
- The follower type dropdown, role icons/checks, source20px dungeon checkbox rows and ScrollBox/MinimalScrollBar use the same elements in Modern and Forever. Pending requests and active matches disable repeat admission; server refusals remain visible in the follower description and IPC status.
- Native `lfg queue <role> <dungeon IDs>` retains the same selected-role transport for diagnostics. Difficulty205 selects a follower dungeon in the imported server catalog.
- Queue-send acknowledgment is not server acceptance. `lfg status` exposes the latest authoritative snapshot, match roster and refusal; reconnect clears stale state.
- Existing LFG dequeue/role-check requests use the same transport. Completed follower admission is a running instance, not a human matchmaking queue entry.
- Both Modern and Forever show the same replicated followers and existing party frames; no skin-specific gameplay or invented UI frames.
- Local user-approved server workaround uses labelled-spec player spells and same-map Normal population/entrance; absent Rogue/DK/DH identities and full-rotation gaps remain explicit in the server contract.

## Tests

- `ui-model/tests/follower_dungeons.rs` — real both-skin registry, sourced catalog selection, selected-role queue actions, pending/refusal/match state and absent/stale-catalog refusal.
- `network/src/wire_tests.rs::follower_group_finder_default_bridge_receives_catalog_over_udp` — default bridge, real loopback UDP catalog delivery.
- `account_dungeonclient_tests.rs::follower_group_finder_account_dispatches_catalog_and_authoritative_state` — decoded catalog and LFG state reach the native host while IPC retains authoritative state.
- Private UDP5761 Group Finder admission now has both-skin proof: actual native microbutton/type/role/dungeon/Find Group clicks, authoritative five-member party and four companions following14m movement. [Exact proof and limits](../wiki/systems/follower-dungeons.md#both-skin-native-admission--2026-10-13). This does not prove combat, the walked route or dungeon completion.

- `account_dungeonclient_tests.rs::follower_queue_native_account_retains_server_rejection_and_match_state` — actual decoded account traffic.
- `follower_wire_tests.rs` — ephemeral UDP role/dungeon request and authoritative refusal round trip.
- Private UDP5750 Modern/Forever queue, correct five-role party, follow/combat and first Ruby boss kills now have live proof. [Exact proof/limits](../../../game-server/docs/wiki/investigations/follower-data.md#both-skin-first-boss-proof--2026-10-11) owns revisions, data receipts and evidence.
- Authored boss loot is absent and native walking did not complete the approach; admin reposition was used. These gates remain open, not end-to-end completion. Eight inspected PNGs: `/syncthing/AgentShared/2026-10-11/follower-ai/`.
