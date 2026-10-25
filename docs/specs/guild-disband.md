# Guild disband

The current Guild Master can destroy the current guild through a confirmed native action in Modern and Forever. Protocol requests live in shared `guild_rank_messages.rs`, authority in server `guild_ranks.rs`, and presentation in the native guild screen. [Guild systems](../wiki/systems/guilds.md) owns implementation details.

## What it must do

- [ ] Only the authenticated current Guild Master can disband the current guild. Other members, unguilded actors and actors from another guild cannot delete it. The request carries no actor or guild ID.
- [ ] `/gdisband` opens a local Yes/No confirmation for the Guild Master. Opening, No and Escape send no destructive request. There is no confirmation timeout. Yes sends once; pending duplicate input sends nothing. A leadership or membership change invalidates an open confirmation.
- [ ] Both Modern and Forever expose a Guild Master-only Disband Guild action on the Info pane. It uses the same confirmation and authority path as the slash command. This placement is a project UI entry, not a claim that Retail has this exact button position.
- [ ] Disband is allowed with other members present. One durable transaction removes every member (including offline members), guild ranks, notes, profile, bank contents/money/logs, guild event/news records, registered appearance and creation metadata, pending invitations to this guild, and the reserved guild name. Other guilds and player-owned inventories remain unchanged.
- [ ] Online members receive authoritative empty guild state and lose replicated guild membership/appearance. Reconnect and storage reopen remain unguilded. No member is silently transferred to another guild.

## How it works

- [Guild systems](../wiki/systems/guilds.md) — native session, projection and server authority.
- [Source rules below](#retail-sources) — confirmed destructive operation, not a last-member-only leave shortcut.

## Implementation inventory

- Shared `src/protocol/guild_rank_messages.rs`: current-guild request and wire test.
- Server `crates/server/src/guild_ranks.rs`: authenticated request dispatch and online refresh.
- Server `crates/server/src/guild_disband.rs` (planned): atomic durable deletion.
- `godot/ui-model/src/guild_ranks.rs`: local confirmation and pending-authority decisions.
- `godot/ui-model/src/guild_rank_frame.rs`, `guild_communities.rs`: both-skin native entry and confirmation.
- `godot/ui-model/src/ui/chat_frame.rs`, `godot/rust/src/chat.rs`, `guild_ranks.rs`: slash routing and actual packet boundary.

## Tests asserting this spec

- Shared `src/protocol/guild_rank_messages.rs`: identity-free request round-trip.
- Server `crates/server/src/guild_disband_tests.rs`: refusal, offline persistence/reopen, isolated deletion, authenticated two-client refresh and replicated cleanup.
- `godot/ui-model/tests/guild_disband_tests.rs`: both-skin eligibility, No/Escape, Yes-once, pending authority and invalidation.
- `godot/tests/guild_disband_live.gd`: two actual native clients, both skins, UI/slash confirmation, member refusal, No/Escape, Yes and fresh server/client readback.

## Known gaps (current cycle)

- [ ] Behavioral GREEN, touched checks/format and native acceptance are pending.

## Out of scope

No unrelated guild feature is added by disband. Historical creation/rank/bank/chat/ReadyCheck evidence remains separately qualified in the end-to-end coverage matrix. Physical audio audibility is not claimed.

## Retail sources

Local Retail cache root: `/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_ChatFrameBase/Shared/SlashCommands.lua:805–809`: `IsGuildLeader()` gates `StaticPopup_Show("CONFIRM_GUILD_DISBAND")`.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:662–673`: Yes/No, `OnAccept` calls `C_GuildInfo.Disband()`, `timeout = 0`, `whileDead = 1`, `hideOnEscape = 1`.
- TrinityCore pinned `a352b1fa`, `src/server/game/Guilds/Guild.cpp:2140–2148`: `HandleDelete` authorizes only `_IsLeader`.
- Same source `:1199–1265`: `Disband` notifies online members, removes all members in a database transaction, deletes guild/rank/bank/event data and removes the guild. No last-member prerequisite is present.
