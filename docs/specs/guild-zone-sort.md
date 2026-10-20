# Guild Zone header sorting

Activate the existing Mainline100px Zone header using already-authoritative zone/logout snapshots. Mainline CommunitiesMemberList.lua115–120/798–819/822–838/871–896 supplies header/direction, known-offline priority and zone/nil structure, subject to the explicitly approved offline policy below.

## What it must do

- [x] Existing Zone header sorts presentation only in both skins; first/new-column click uses the ascending direction and repeated click reverses primary ordering.
- [x] Known offline logout rows precede zone/nil rows ascending; remaining known online zones use the allowed string comparison, missing values follow. Reverse primary direction on repeat; equal primary values keep the existing ascending character-name order.
- [x] **GUILD_ZONE_SORT_OFFLINE_BY_LOGOUT_SECONDS:** known offline rows use raw last_logout_unix_secs, most recent first ascending. Retail relative counters and sub-hour ties are not reproduced. Online presence ignores stale offline time; unknown history is not guessed.
- [x] Zone strings follow the explicitly allowed historical aso behavior-prose packet at `/home/osso/.worktrees/logs/guild-note-sort-2026-10-18/strcmputf8i-prose-packet.md`: mapped-codepoint lexical/prefix ordering, stated case ranges only, no normalization/expansion, astral values map to U+FFFD. No native Lua binding or Retail70448 parity claim.
- [x] Preserve source header geometry, existing filtering/scroll clamping, immutable member/rank snapshots, original member detail/action targets and other sort columns.

## How it works

- [Guild system](../wiki/systems/guilds.md); [Last Online authority](guild-last-online.md).

## Implementation inventory

- `godot/ui-model/src/guild_communities.rs`: existing header and presentation list.
- `godot/ui-model/src/guild_ranks.rs`: existing sort-click decisions.
- `godot/ui-model/tests/guild_zone_sort_tests.rs`: rendered policy/packet/header/overflow/filter/identity regressions, registered under guild_rank_widgets.

## Tests asserting this spec

- `guild_zone_sort_offline_seconds_direction_reset_and_name_ties_both_skins`.
- `guild_zone_sort_allowed_prose_collation_and_fixed_name_ties_both_skins`.
- `guild_zone_sort_real_header_dispatch_preserves_scroll_filter_and_member_identity`.

## Known gaps (current cycle)

- [ ] Ignored tests/spec were pushed first; actual RED3 observed (empty existing header action, unchanged mixed-presence order and unchanged raid/RAID order). Implementation committed before focused GREEN; gate and allocated real two-member both-skin native proof remain pending.
- **GUILD_ZONE_SORT_NATIVE_BINDING_AND_LIMIT_UNQUALIFIED:** allowed historical prose does not establish native Lua binding/Retail70448 parity or a character-count limit. Invalid-UTF8 historical decoding is outside Rust String inputs. Do not invent those missing semantics or open decompiled source.

## Out of scope

Protocol/server/persistence changes, new widgets/layout, mobile/BNet, Communities text/history/voice, professions, leadership/self-policy changes and backdated native presence data. Larger offline-time differences and packet Unicode cases are deterministic model proof, not fabricated native rows.
