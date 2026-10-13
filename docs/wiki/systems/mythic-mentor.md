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

## Authored summaries and teleports — 2026-10-13

Server display summaries load imported MapChallengeMode names/timers separately from full playable run qualification. A real bag key for Kings' Rest249, Temple250, Ruby399, Blinding584, Voidscar585, Nalorakk586, Murder587 or Altar588 is no longer misreported as “No key”; Azure401 remains supported. Native icons use MapChallengeMode.MapID→JournalInstance.MapID/ButtonSmallFileDataID from local build12.1.0.69933; nine square128×128 LFG images were extracted locally and published by the lead, not fabricated from panoramas.

The existing native post-InWorld ChallengeRequest::Query now publishes the recipient's complete party list even if the one-second change cache matches. This is the explicit ready-reader handshake, not retries. HostClient regression primes/discards the bootstrap, freezes periodic refresh and observes the identical query reply. Initial native send/receipt timeouts do not establish a lower-level transport root cause; final plain fresh-account sessions pass.

All eight installed-addon teleport IDs have local DB2 RecoveryTime0, CategoryRecoveryTime28800000ms and shared category1407. PlayerSpells retains that server-reported category clock; both-party icon countdowns use it even when another teleport started the clock. Shared countdown formatting shows8h, sourced from Retail TimeUtil's hour abbreviation. Native hover uses actual spell catalog text and addon's “Spell not learned.” presentation; mapped icon hover darkens50%, without a swipe.

Plain engine2a16e922f/server06fbe740/protocol29c40b8 runs in `live/hour-{modern,forever}/` prove Ruby+8/28:00 + Kings+11/33:00, unknown-spell click denial, learned-current-teleport denial for missing SpellTargetPosition without movement/cooldown spending, and actual sourced Scarlet131229 cast/success/position driving both icons' real8h category clock. No snapshot injection or fabricated current destination. Earlier `live/{modern5,forever}/` retains both-skin crown/raid/party/drag/save/Escape/no-key/bag-grant proof. Slow post-travel probes initially expired, then recorded correct pruning; they are not permanent-state-loss proof.

Native pixel inspection found tooltip canvas8 below Mentor dialog10, obscuring otherwise correct text. Engine37979322b sets tooltip canvas30 above dialog/launcher10/20 and below cursor100. `live/layer-{modern,forever}/*foreground-unlearned.png` shows complete post-fix overlapping tooltip text in both skins. Source install auto-patched to12.1.5.70340 at11:08 on2026-10-13 and no longer contains pinned builddcfc90fffd79ba00406ae46f5f657592; subsequent client startup fails. Final post-layer **active** cooldown pixels therefore remain blocked, not claimed from the earlier layer8 captures. `layer-modern/*foreground-cooldown.png` has only the base8h description and is not active-clock evidence.

All eight current teleport destinations are absent from local TDB; full playable Mythic+ runs remain Azure401-only and no realm active season is inferred. Counts of promoted creatures alone do not assert complete dungeon mechanics. Exact revision/command/pixel bounds and lead-only data publication are in `/home/osso/.worktrees/handoff-mythic-mentor.md`.

## Sources

- Installed `BossHelper/UI/{GroupFinderPanel,KeystoneWidget,MythicTeleport}.lua` — list geometry, ordering, no-key, crown and teleport behaviour.
- [UI model](../../../godot/ui-model/src/mythic_mentor.rs), [native host](../../../godot/rust/src/mythic_mentor.rs), [live fixture](../../../godot/tests/mythic_mentor_live.gd).
- [Server producer](../../../../game-server/crates/server/src/mythic_mentor.rs) — persistent bag-authoritative summaries.

## See Also

- [Mythic Keystone UI](../../specs/mythic-plus.md) — Font/run UI uses a separate challenge snapshot.
- [Native HUD Edit Mode](native-hud-edit-mode.md) — existing per-character layout storage.
