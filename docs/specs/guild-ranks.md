# Guild ranks and permissions

## Required behavior

Guild settings → Guild Ranks in the guild/communities frame, in both Modern and
Forever skins, provides rank list, per-rank flags, bank tab rights and daily gold
input. Forever changes presentation, not Retail rules. Only the Guild Master edits
rank configuration. Rank 0 is immutable; ranks number 2–10. Names are at most 15
characters. Occupied ranks cannot be deleted. Reordering follows rank identity.

Member context menus expose Promote/Demote only with the respective rank flag and
strictly higher actor rank; promotion must stop below the actor's rank. State is
server-authoritative: sending a request never changes displayed ranks, rights or
limits until `GuildRanksState` arrives. Refusals use Retail GlobalStrings.

Gold input is whole gold, sent as copper/day; guild repairs share that allowance.
Tab view/deposit and stacks/day are separate controls. Zero means no withdrawals;
only Guild Master is unlimited. An inaccessible tab disables deposit/withdraw.

## Implementation status

- Portable `GuildRanksSession` exposes authoritative selected-rank data and creates
  exact `GuildRankRequest` values for controls. It is not mounted in a native window.
- `/o` and `/officer` send the rank-gated Officer channel and use its chat styling.
- Pending: native guild/communities frame, settings entry point, Modern/Forever
  rank/permission widgets, edit boxes and tab controls, member context menu,
  native send/receive bridge integration and actual widget rendering tests.

## Retail sources

Root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:4,536–620`: rank count/order,
  immutable leader row and occupied-rank delete restriction.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.xml:7`: 15-letter rank names.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:137–238,420–459`: tab rights,
  withdrawal count, flags and gold/repair input.
- `Blizzard_Communities/GuildRoster.lua:137–141`: member rank actions.
- `Blizzard_Communities/CommunitiesFrame.xml:265`: GuildControlUI from communities.
- `data/GlobalStrings.csv`: Retail refusal strings.

## Tests

`godot/ui-model/tests/native_guild_ranks.rs` covers exact request payloads, unchanged
state until server acknowledgment, selected-rank state from replies, GM-only edits,
rank-zero protection, occupied deletion, hierarchy and officer chat classification.
These are model tests, not proof that native settings widgets exist.

See [guild bank](guild-bank-frame.md) for vault actions and the server
`docs/specs/guild-ranks.md` for authority, persistence and enforcement.
