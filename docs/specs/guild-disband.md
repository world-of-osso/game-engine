# Guild disband

## Contract

- Only the authenticated current Guild Master can disband the current guild. Other members, unguilded actors and actors from another guild cannot delete it. The request carries no actor or guild ID.
- `/gdisband` opens a local Yes/No confirmation for the Guild Master. Opening, No and Escape send no destructive request. There is no confirmation timeout. Yes sends once; pending duplicate input sends nothing. A leadership or membership change invalidates an open confirmation.
- Both Modern and Forever expose a Guild Master-only Disband Guild action on the Info pane. It uses the same confirmation and authority path as the slash command. This placement is a project UI entry, not a claim that Retail has this exact button position.
- Disband is allowed with other members present. One durable transaction removes every member (including offline members), guild ranks, notes, profile, bank contents/money/logs, guild event/news records, registered appearance and creation metadata, pending invitations to this guild, and the reserved guild name. Other guilds and player-owned inventories remain unchanged.
- Online members receive authoritative empty rank/profile state and lose replicated guild membership/appearance. Reconnect and storage reopen remain unguilded. No member is silently transferred to another guild.

## Source

Local Retail cache root: `/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_ChatFrameBase/Shared/SlashCommands.lua:805–809`: `IsGuildLeader()` gates `StaticPopup_Show("CONFIRM_GUILD_DISBAND")`.
- `Blizzard_StaticPopup_Game/GameDialogDefs.lua:662–673`: Yes/No, `OnAccept` calls `C_GuildInfo.Disband()`, `timeout = 0`, `whileDead = 1`, `hideOnEscape = 1`.
- TrinityCore pinned `a352b1fa`, `src/server/game/Guilds/Guild.cpp:2140–2148`: `HandleDelete` authorizes only `_IsLeader`.
- Same source `:1199–1265`: `Disband` notifies online members, removes all members in a database transaction, deletes guild/rank/bank/event data and removes the guild. No last-member prerequisite is present.

## Proof

Behavioral wire, persistent authority/reopen, authenticated two-client dispatch and both-skin confirmation tests are required. Native acceptance must use two actual clients against a private local UDP server, exercise member refusal, UI/slash confirmation, No/Escape and Yes, and capture both actors after removal in both skins. Historical creation/rank/bank/chat/ReadyCheck evidence remains separately qualified in the end-to-end coverage matrix.
