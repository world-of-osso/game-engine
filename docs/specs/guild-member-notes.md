# Guild member details and notes

Retail Mainline member details and public/officer notes in Modern and Forever. Sources: installed `Blizzard_Communities/GuildRoster.lua`, `GuildRoster.xml`, `Blizzard_StaticPopup_Game/GameDialogDefs.lua:1834` and pinned TrinityCore a352b1fa `Guild.cpp:1597`. [Data flow](../wiki/systems/guilds.md#member-details-and-notes).

## What it must do

- [x] Member details show authoritative name, class/level, rank, public note and permitted officer note. Unknown zone and exact last-online times must not be invented. Public notes populate the roster Note column independently of officer text.
- [x] Mainline permits own-public-note editing without the general public-edit right. Other public writes require that right. Officer writes require edit-officer permission; the native editor additionally requires view permission. Officer payloads are stripped from unauthorized recipients.
- [x] Authenticated actor and same-guild target are validated server-side;31 Unicode characters maximum, no control characters. Cancel writes nothing. Popup target remains the member selected when editing began even if roster selection subsequently changes. Enter/Accept waits for authoritative profile replacement, including the interval between rank ACK and profile reply.
- [x] Both note types survive durable database close/reopen, preserving existing officer records. Leave/removal deletes both before rejoin. Native relog remains a separate acceptance boundary below.
- [ ] Group Invite uses the actual group service. Remove requires Mainline `REMOVE_GUILDMEMBER` Yes/No confirmation with the captured member, rechecks existing hierarchy authority on Yes, and closes the detail only after confirmation. No/Escape sends nothing; closing the guild window discards the popup. Physical native proof remains required.

## How it works

- [Guild state and member notes](../wiki/systems/guilds.md#member-details-and-notes).
- [Existing ranks/lifecycle](guild-ranks.md).

## Implementation inventory

| Files | Role |
|---|---|
| `godot/ui-model/src/guild_member_detail.rs`, `guild_ranks.rs`, `guild_communities.rs` | Shared both-skin view, note reducer and public roster column |
| `godot/rust/src/guild_ranks.rs`, `account.rs`, `ui/mod.rs` | Native input, Enter dispatch, group invite and authoritative reply |
| shared-protocol `src/protocol/guild_rank_messages.rs`, `protocol_snapshots.rs` | Note request, validator and per-recipient public/self fields |
| server `guild_ranks.rs`, `guild.rs`, `persistence_character.rs` | Authenticated validation, private/public projection and durable storage |

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs`: both-skin detail/public-note/privacy, self-edit, pending-authority and cancellation decisions.
- `godot/network/src/guild_rank_wire_tests.rs`: actual loopback note requests on GuildChannel.
- server `guild_ranks_tests.rs`: per-recipient privacy; Unicode31/32, same-guild/outside actor, restart and leave/rejoin cleanup.
- `godot/tests/guild_member_notes_live.gd`: private real-input master/self/restart readback and full-HD captures.

## Known gaps (current cycle)

- [x] Both-skin private5891 master/self/restart-readback: six physical native runs pass at engineaa77c226b, serverbd3a121e, protocol671fe2f; .so SHA256 prefixf06f87 is embedded in distinct skin-specific notes. Fourteen1920×1080 PNGs inspected in `/syncthing/AgentShared/2026-10-15/guild-completion/`: detail, public/officer input, removal confirmation, self privacy and restart readback in each skin. Note Escape and Remove No/Escape send nothing. Actual server process restarts preserve both note types.
- [ ] Final merged-head checks. UI22/22 atbea14a77f covers notes/removal capture and cancellation; earlier actualUDP4 passes both note requests and public_note/is_self fields. Server guild11/11 at10c5d019 covers durable reopen/privacy/rank rights; later Guild Master migration fix has a separate targeted proof. Earlier --lib0-test invocation is not proof.
- [ ] Source direct rank-assignment dropdown, online zone and elapsed-last-online metadata.

## Out of scope

News, lifecycle history, benefits/perks and sourced rewards are separate subsequent guild-completion slices, not waived from the overall goal.
