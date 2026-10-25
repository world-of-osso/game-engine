# Communities guild Chat

Mainline Communities guild Chat in Modern and Forever uses the existing authorized guild/officer chat and history service. Layout follows installed `Blizzard_Communities/CommunitiesFrame.xml:458–501`, `CommunitiesFrame.lua:1663–1674`, `CommunitiesChatFrame.xml` and `CommunitiesChatFrame.lua`; stream labels use installed GlobalStrings18130/18131 and messages36348. [Guild system](../wiki/systems/guilds.md) owns data flow.

## What it must do

- [x] Chat renders the source maximized transcript/inset/MinimalScrollBar,32px composer,160px stream dropdown and compact roster in both skins. Guild Chat and Officer Chat select existing server channels, not invented Club streams. Compact MemberList uses its literal frame-right anchors and remains independent of the expanded-roster Show Offline preference.
- [x] Opening Chat queries existing recipient-authorized history. Render only actual guild/officer messages for the selected permitted channel; no sample lines, fabricated dates or local optimistic echoes. Reopening refreshes history without duplicating a received snapshot. Snapshot replacement and subsequent live deliveries retain server ChatChannel order across the native typed receiver buffers.
- [x] Composer Enter sends one actual ChatMessage through the same service as /g or /o; empty input sends nothing,512-byte source limit is enforced and Escape only releases focus. A successful send clears the native input, not the transcript.
- [x] Current rank's independent listen/speak rights control stream visibility and composer availability. Server authority remains decisive; an ordinary member's officer send is refused without disclosure to unauthorized recipients/history.
- [x] Membership loss/logout clears guild-only presentation and late history cannot reopen it. Closing/switching tabs does not send a message.
- [x] Actual two-actor native guild/officer delivery, non-officer denial, history-on-open and both-skin original full-HD captures are published under `/syncthing/AgentShared/2026-10-25/guilds/chat/`.

## How it works

- [Guild system](../wiki/systems/guilds.md).
- [Existing main chat](chat-frame.md).

## Implementation inventory

- `godot/ui-model/src/guild_chat.rs`: source stream picker/transcript/composer, recipient-history state and exact service commands.
- `godot/ui-model/src/guild_communities.rs`: Communities shell and compact actual-member roster.
- `godot/ui-model/src/guild_ranks.rs`: authoritative guild session and command output.
- `godot/rust/src/guild_ranks.rs`: native input and guild presentation lifecycle.
- `godot/rust/src/chat.rs`, `account.rs`: existing chat service boundary.
- `godot/network/src/lib.rs`: one message-ID-ordered relay for existing ChatChannel live/history replies.

## Tests asserting this spec

- `godot/ui-model/tests/guild_chat_tests.rs`: rendered controls, history, rights and exact command behavior.
- `godot/network/src/guild_rank_wire_tests.rs`: real UDP existing history and held-worker snapshot/live ordering.
- `godot/tests/guild_chat_projection.gd`: existing offline fixture authority, physical Chat/stream/composer controls and resolved source rectangles in both skins; not live-service proof.
- `godot/tests/guild_chat_live.gd`: two actual native actors, source rectangles, physical composer/stream changes, delivered packets/history, ordinary-rank speak/listen privacy and full-HD captures. Debug-only `guild_chat_observation` exposes actual received data without a setter; no reply fixture injection.

## Acceptance — 2026-10-25

The combined Chat/disband branch passes 78 guild widget controls, two real UDP Chat/history tests and four server Chat rights/history controls. Actual native cohorts `chat/modern-1792949640` and `chat/forever-1792949934` pass delivery, focused composer Escape, source rectangles, history-on-open and demotion/privacy refusal. Original full-HD captures are inspected; each cohort retains actor logs and exact artifact hashes. [Disband acceptance](guild-disband.md#acceptance--2026-10-25) additionally proves membership-loss clearing through actual guild destruction. The parent owns final integration.

## Out of scope

New chat services, Club/mobile/BNet/voice/Discord, message editing/deletion, fabricated stream/message IDs or timestamps, guild-bank tab metadata and internal moves/splits. Tabard vendor full-path proof is the next separate approved slice.
