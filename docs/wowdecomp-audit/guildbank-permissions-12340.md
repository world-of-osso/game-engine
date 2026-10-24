# Guild-bank permission getters — build 12340

Inspection: 2026-10-24. Native reference W is `/home/osso/Repos/wowdecomp` at `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` (3.3.5a/build 12340 reconstruction). This is reconstructed C++/Lua-binding evidence, not a client-runtime or server implementation claim.

## `GetGuildBankTabPermissions(tab)`

The global binding requires a number-convertible first argument; a nonnumeric or missing argument raises a Lua usage error. It reads only argument 1 and ignores extras. It converts the value to `int32`, subtracts one, then narrows to `uint8`. There is no explicit 1–6 range check. For ordinary inputs whose normalized byte is at least six (for example, tab 0 or 7), the getter returns four values: nil, nil, nil, and numeric 0. Because of the narrowing conversion, some extreme out-of-range values can wrap to a valid tab index; the binding does not enforce a strict user-facing range.

For a valid normalized tab and selected rank, the exact result order is:

| Result | Meaning | Lua value |
|---|---|---|
| 1 | View permission, rights bit `0x1` | numeric 1 or nil |
| 2 | Deposit permission, rights bit `0x2` | numeric 1 or nil |
| 3 | Edit-tab-info permission, rights bit `0x4` | numeric 1 or nil |
| 4 | Per-tab stack-withdrawal limit (`m_bankTabStacksPerDay`) | number |

The fourth result is a count/limit, not a boolean. `GetGuildBankTabInfo` separately returns the first two rights markers and the stack-withdrawal count; `CanEditGuildTabInfo` checks bit `0x4`.

The binding reads `m_rankInfo[s_guildControlRank]`, not the active player's rank, and has no guild-membership or missing-rank guard. `GuildControlSetRank` takes a one-based rank and stores its zero-based index; rank 1 selects internal rank 0.

## `GetGuildBankWithdrawLimit()`

This global reads no arguments, does not validate argument count, and ignores extras. It returns one Lua number from the selected rank's `m_bankMoneyPerDay`. For nonnegative stored values it returns integer division by 10,000; negative values pass through unchanged. The field and API identify the selected rank's daily guild-bank money limit. The setter/save path's 10,000 multiplier is consistent with a gold-scale UI value, but this client source does not prove server-side unit policy.

## Backing state, defaults, and rank cases

`m_rankInfo` holds rank-wide permissions, the daily money limit, six tab-right masks, and six per-tab stack-withdrawal limits. The roster packet fills each rank from server data and clears unused rank slots. The permissions reply updates one rank from server data. Both packet handlers signal `GUILD_ROSTER_UPDATE`; neither getter emits an event.

On fresh initialization, static rank storage begins zeroed and `s_guildControlRank` is set to -1. `EnterWorld` requests permissions only when the active player has a guild ID. It also seeds internal rank 0's six tab-right masks and stack-withdrawal limits to -1; it does not seed that rank's money limit. Thus, before any server reply, selecting rank 0 yields all three tab markers set and a per-tab limit of -1; the money-limit getter returns 0 from its zero-initialized field. After a server reply, getters reflect the returned rank data.

There is no leader-specific branch in either getter. A leader's result is the selected rank's data; when internal rank 0 is selected before server data, it has the local tab defaults above. On a fresh no-guild path, `s_guildControlRank` remains -1 and no guild-permissions query is sent. For a normalized tab below six, `GetGuildBankTabPermissions` indexes `m_rankInfo[-1]` and has no source-defined result; for a normalized tab of six or more, it avoids that rank access and returns nil, nil, nil, numeric 0. `GetGuildBankWithdrawLimit` indexes the selected rank unconditionally, so rank -1 has no source-defined result. If a rank has been explicitly selected despite there being no guild, the getters read that rank's stored values instead.

The getters are registered global functions and do not require the guild-bank frame to be open. Rank selection affects which row they read; permission data arrives through the separate server packet handlers.

## Evidence boundary

Sources: `WoW/Source/Ui/GuildInfo_Scripts.cpp:286–313,352–390,525–558,726–761,803–834,1018–1025`; `WoW/Source/Ui/GuildInfo.cpp:32–34,73,78–86,88–114,201–222,244–267,269–360`; `WoW/Source/Ui/GuildInfo.h:12–16,67–76`; `WoW/Source/Ui/GuildBankFrame_Scripts.cpp:560–624`.

The exact behavior of the original 12340 client and the server's intended sentinel/unit semantics were not runtime-validated. No client, simulator, server, build, or tests were run.
