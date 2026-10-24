# Communities guild Chat

Mainline Communities guild Chat in Modern and Forever uses the existing authorized guild/officer chat and history service. Layout follows installed `Blizzard_Communities/CommunitiesFrame.xml:458–501`, `CommunitiesFrame.lua:1663–1674`, `CommunitiesChatFrame.xml` and `CommunitiesChatFrame.lua`; stream labels use installed GlobalStrings18130/18131 and messages36348. [Guild system](../wiki/systems/guilds.md) owns data flow.

## What it must do

- [ ] Chat renders the source maximized transcript/inset/MinimalScrollBar,32px composer,160px stream dropdown and compact roster in both skins. Guild Chat and Officer Chat select existing server channels, not invented Club streams.
- [ ] Opening Chat queries existing recipient-authorized history. Render only actual guild/officer messages for the selected permitted channel; no sample lines, fabricated dates or local optimistic echoes. Reopening refreshes history without duplicating a received snapshot.
- [ ] Composer Enter sends one actual ChatMessage through the same service as /g or /o; empty input sends nothing,512-byte source limit is enforced and Escape only releases focus. A successful send clears the native input, not the transcript.
- [ ] Current rank's independent listen/speak rights control stream visibility and composer availability. Server authority remains decisive; an ordinary member's officer send is refused without disclosure to unauthorized recipients/history.
- [ ] Membership loss/logout clears guild-only presentation and late history cannot reopen it. Closing/switching tabs does not send a message.
- [ ] Actual two-actor native guild/officer delivery, non-officer denial, history-on-open and both-skin original full-HD captures are published under `/syncthing/AgentShared/2026-10-24/guild-chat/`.

## How it works

- [Guild system](../wiki/systems/guilds.md).
- [Existing main chat](chat-frame.md).

## Implementation inventory

- `godot/ui-model/src/guild_communities.rs`: Communities shell/tab composition.
- `godot/ui-model/src/guild_ranks.rs`: authoritative guild session and command output.
- `godot/rust/src/guild_ranks.rs`: native input and guild presentation lifecycle.
- `godot/rust/src/chat.rs`, `account.rs`: existing chat service boundary.
- `godot/network/src/lib.rs`: typed existing chat/history receiver relay.

## Tests asserting this spec

- `godot/ui-model/tests/guild_chat_tests.rs`: rendered controls, history, rights and exact command behavior.

## Known gaps (current cycle)

- [ ] Implementation and targeted RED/GREEN.
- [ ] Real two-actor native acceptance and inspected originals; lead owns final integration gate.

## Out of scope

New chat services, Club/mobile/BNet/voice/Discord, message editing/deletion, fabricated stream/message IDs or timestamps, guild-bank tab metadata and internal moves/splits. Tabard vendor full-path proof is the next separate approved slice.
