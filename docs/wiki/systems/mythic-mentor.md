# Native Mythic Mentor

Verified source/UI model: 2026-10-11. [Contract](../../specs/mythic-mentor.md) owns elements, behaviour and art identities; [server contract](../../../../game-server/docs/specs/mythic-mentor.md) owns party visibility and persisted bag authority.

## Content

`PartyKeystoneSnapshot` enters the normal network bridge and Account reducer. `ChallengeUi` owns the separate Mentor window lifetime and includes it in native window visitation. The launcher entry opens its rsx screen; Escape and the ordinary close button close it. Native `WindowDrag` clamps the title drag and the existing layout store preserves its position per selected character.

Snapshots may arrive while the Loading screen is still attaching the world. Clear them when stopping the transport (both fresh connection preparation and logout), not every non-InWorld frame: the latter discarded the only initial solo snapshot while the server's change cache correctly withheld duplicates. The live fixture asserts solo data before forming the party, exercising that exact process ordering.

The addon places a separate topLayer above the icon button. Native level/crown art needs the same ordering; sibling labels could otherwise be behind the deeper button texture, despite correct registry text. Initial native capture exposed this; the rsx top layer now owns icon overlays.

Final proof artifacts live at canonical `data/diagnostics/mythic-mentor-2026-10-11/`. Final integrated runtime d20f07cab and serverdb8caa43 (updated master with follower parties): private5811, fb_mentorone/Mentorone and fb_mentortwo/Mentortwo; both Modern and Forever initially receive solo data, then display both real +8/+11 keys and authored37:30. Physical launcher/open/drag/save/close/reopen/Escape pass in both skins. Modern peer leave and Forever leader uninvite prune the open panel back to self. PNGs in `/syncthing/AgentShared/2026-10-11/mythic-mentor/` are inspected1280×720 captures. UI/account/server targeted tests total4 pass, including the merged four-NPC companion-party boundary; accepted logout finding has separate RED→GREEN proof. Client/server/shared checks and targeted18-file formatting pass; no full suites.

## Expanded private proof — 2026-10-13

Fresh UDP5751, fb_mentor13* accounts and independent redb/SQLite source copy exercise observed rendered labels, not preview injections. `godot/tests/mythic_mentor_live.gd` now records rows/crown/rect, supports ordinary invite acceptance and typed chat commands, and publishes state atomically; world-geometry readiness alone does not establish asynchronous snapshot readiness. Cold server bind is likewise not startup readiness: require an admin ping before login.

Modern and Forever receive authentic +8/+11 Azure37:30, transfer leadership both directions, hide while converted to raid, reappear when converted to party, save title dragging across close/reopen, close with Escape, and prune an open panel when peer leaves. Forever also shows real no-key→online bag grant11 replacement. Proof JSON and native logs are at `/home/osso/.worktrees/game-engine-s3/data/diagnostics/mythic-mentor-2026-10-13/live/{modern5,forever}/`; inspected1280×720 PNGs at `/syncthing/AgentShared/2026-10-13/mythic-mentor/`. These captures cover engine baseline1b76b997e with fixture commits and server675bec22/protocolab7f8481, not subsequent unrelated master changes.

Initial prototype native runs failed snapshot-readiness assertions, including one60s wait; final native-first sessions succeeded and temporary server instrumentation confirmed changed-snapshot sends. Those early runs do not establish a transport root cause or a production fix. Catalog/teleport expansion is tracked by the contract, not claimed complete by this panel proof.

## Sources

- Installed `BossHelper/UI/{GroupFinderPanel,KeystoneWidget,MythicTeleport}.lua` — list geometry, ordering, no-key, crown and teleport behaviour.
- [UI model](../../../godot/ui-model/src/mythic_mentor.rs), [native host](../../../godot/rust/src/mythic_mentor.rs), [live fixture](../../../godot/tests/mythic_mentor_live.gd).
- [Server producer](../../../../game-server/crates/server/src/mythic_mentor.rs) — persistent bag-authoritative summaries.

## See Also

- [Mythic Keystone UI](../../specs/mythic-plus.md) — Font/run UI uses a separate challenge snapshot.
- [Native HUD Edit Mode](native-hud-edit-mode.md) — existing per-character layout storage.
