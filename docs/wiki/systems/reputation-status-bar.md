# Watched reputation status bar

[Behavior contract](../../specs/reputation-status-bar.md). This is a dedicated RegistryUi reputation canvas applying Retail's container priority/offset, not a new Lua StatusTrackingBarManager instance.

## Data and native lifetime

The realm's existing `ReputationStateUpdate` contains `ReputationSnapshot.watched`: normalized watched Faction ID/name, value/maximum, capped status, major-faction classification and standard reaction. Initial in-world publication reads persisted watch; accepted watch/clear and existing reputation/quest publications update it without sending the Journeys reward catalog. `None` explicitly removes a watch. Refusals preserve it.

`GameClient.reputation` receives that snapshot. `update_reputation_bar` mounts only in world with a watched payload, synchronizes the shared screen, and frees it on clear/outside-world. The canvas participates in the existing HUD skin/scale traversal and topmost-frame hover. Source `StatusTrackingManager` priority selects the reputation container based on XP visibility. The approved Forever preset translates the same17px container relationship from the existing top XP anchor; action bars and XP remain unchanged.

## Proof — 2026-10-14

UI API RED becomes four behavioral passes: container promotion/geometry in both skins, progress/text/cap/leave, no watch, and actual source sheet crops. Server proofs cover normalized standard/renown/capped values, redb reopen plus login publication without RenownRequest, clear/refusal retention at the delivery frame, and committed quest reputation publication. The ignored offline fixture is explicitly invoked only for assigned private characters; it persists Stormwind72 at3800, not synthetic MajorFaction eligibility.

Native source3010f78da/serverc411c123/protocolc9d7fdf passes both skins: login restores the bar without opening/fetching Journeys, source geometry and real green pixels represent800/6000, physical hover shows `Stormwind 800 / 6000`, and leave hides text. Four inspected original1920×1080 PNGs are under `/syncthing/AgentShared/2026-10-14/reputation-status-bar/3010f78da/`; private `reputation-bar-anchor-runtime-receipt.json` binds exact binaries, fixtures and PNG hashes. An earlier Forever pixel failure caught overlap with its default action bars; the approved relative XP anchor fixes it, without weakening the painted-pixel assertion.

These captures do not certify native MajorFaction eligibility, watch clicking/clearing, XP-cap promotion, gameplay progress changes, or complete manager parity. Gain/transition animations, paragon/friendship/account-wide metadata, always-on text CVars and Edit Mode bar settings remain open. Current first-open Journeys displays remain unloaded after90s even with the published UDP receive queue. The old count observes View.renown, copied only after local spell/item readiness, not received Account.renown; it does not prove a missing reply. Received-cache/prerequisite observations are being separated, with precise physical-open timestamps and no retry or packet-correlation claims.

## Sources

- [Shared screen](../../../godot/ui-model/src/reputation_status_bar.rs), [native host](../../../godot/rust/src/reputation_status_bar.rs), [native fixture](../../../godot/tests/reputation_status_bar_live.gd).
- [Realm reputation](../../../../game-server/crates/server/src/reputation.rs), [Renown watch publication](../../../../game-server/crates/server/src/renown.rs), [wire contract](../../../../shared-protocol/docs/reputation-status-bar.md).

## See Also

- [[xp-bar]] — first container and existing presets.
- [[adventure-guide]] — Journeys eligibility, watch backing and delivery proof boundaries.
