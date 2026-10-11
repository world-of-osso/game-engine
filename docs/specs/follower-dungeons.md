# Native follower dungeon transport

Server gameplay contract: [follower dungeons](../../../game-server/docs/specs/follower-dungeons.md).

- Native `lfg queue <role> <dungeon IDs>` sends the selected role and dungeon IDs through the existing reliable LFG protocol. Difficulty205 selects a follower dungeon in the imported server catalog.
- Queue-send acknowledgment is not server acceptance. `lfg status` exposes the latest authoritative snapshot, match roster and refusal; reconnect clears stale state.
- Existing LFG dequeue/role-check requests use the same transport. Completed follower admission is a running instance, not a human matchmaking queue entry.
- Both Modern and Forever show the same replicated followers and existing party frames; no skin-specific gameplay or invented UI frames.
- Local user-approved server workaround uses labelled-spec player spells and same-map Normal population/entrance; absent Rogue/DK/DH identities and full-rotation gaps remain explicit in the server contract.

## Tests

- `account_dungeonclient_tests.rs::follower_queue_native_account_retains_server_rejection_and_match_state` — actual decoded account traffic.
- `follower_wire_tests.rs` — ephemeral UDP role/dungeon request and authoritative refusal round trip.
- Private UDP5750 Modern/Forever queue, correct five-role party, follow/combat and first Ruby boss kills now have live proof. [Exact proof/limits](../../../game-server/docs/wiki/investigations/follower-data.md#both-skin-first-boss-proof--2026-10-11) owns revisions, data receipts and evidence.
- Authored boss loot is absent and native walking did not complete the approach; admin reposition was used. These gates remain open, not end-to-end completion. Eight inspected PNGs: `/syncthing/AgentShared/2026-10-11/follower-ai/`.
