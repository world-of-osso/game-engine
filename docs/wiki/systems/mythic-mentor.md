# Native Mythic Mentor

Verified source/UI model: 2026-10-11. [Contract](../../specs/mythic-mentor.md) owns elements, behaviour and art identities; [server contract](../../../../game-server/docs/specs/mythic-mentor.md) owns party visibility and persisted bag authority.

## Content

`PartyKeystoneSnapshot` enters the normal network bridge and Account reducer. `ChallengeUi` owns the separate Mentor window lifetime and includes it in native window visitation. The launcher entry opens its rsx screen; Escape and the ordinary close button close it. Native `WindowDrag` clamps the title drag and the existing layout store preserves its position per selected character.

Snapshots may arrive while the Loading screen is still attaching the world. Clear them at transport preparation, not every non-InWorld frame: the latter discarded the only initial solo snapshot while the server's change cache correctly withheld duplicates. The live fixture asserts solo data before forming the party, exercising that exact process ordering.

The addon places a separate topLayer above the icon button. Native level/crown art needs the same ordering; sibling labels could otherwise be behind the deeper button texture, despite correct registry text. Initial native capture exposed this; the rsx top layer now owns icon overlays.

Proof artifacts live at canonical `data/diagnostics/mythic-mentor-2026-10-11/`. Initial private5811 received +8/+11 and authored37:30 for two real accounts; physical launcher/open/drag/close/reopen/Escape exercised. Final revised both-skin capture/check evidence is recorded in the completion handoff, not inferred from this initial run.

## Sources

- Installed `BossHelper/UI/{GroupFinderPanel,KeystoneWidget,MythicTeleport}.lua` — list geometry, ordering, no-key, crown and teleport behaviour.
- [UI model](../../../godot/ui-model/src/mythic_mentor.rs), [native host](../../../godot/rust/src/mythic_mentor.rs), [live fixture](../../../godot/tests/mythic_mentor_live.gd).
- [Server producer](../../../../game-server/crates/server/src/mythic_mentor.rs) — persistent bag-authoritative summaries.

## See Also

- [Mythic Keystone UI](../../specs/mythic-plus.md) — Font/run UI uses a separate challenge snapshot.
- [Native HUD Edit Mode](native-hud-edit-mode.md) — existing per-character layout storage.
