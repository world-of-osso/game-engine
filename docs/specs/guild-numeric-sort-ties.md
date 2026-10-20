# Guild numeric-header equal-key ordering

Existing Rank, Level and Class headers retain their numeric primary order; equal primary values use the already-qualified represented-field base order in both directions. Mainline CommunitiesMemberList.lua798–819/905–913 delegates equal values to FrameXMLUtil/CommunitiesUtil.lua53–78/172–182. [Guild system](../wiki/systems/guilds.md) owns implementation/proof details.

## What it must do

- [x] Rank, Level and Class equal keys use the existing `GUILD_ROSTER_CLUB_FIELDS_ABSENT` policy: online/offline/unknown profile, both-known level descending, guild rank ascending, raw character name ascending. Input snapshot order must not decide these ties.
- [x] Reverse only numeric primary ordering on a repeated click; equal-key base order stays fixed. Preserve current numeric primary values, missing-value placement and new-column direction reset.
- [x] Keep existing header geometry, filtering/scroll clamping, immutable snapshots and original member detail/context/action targets in both skins, including a real second viewport.
- [x] Preserve Name, public Note and Zone behavior; sharing the existing base comparison must not alter those contracts.

## How it works

- [Guild system](../wiki/systems/guilds.md).
- [Existing public Note tie policy](guild-ranks.md#public-note-sorting).

## Implementation inventory

- `godot/ui-model/src/guild_communities.rs`: existing numeric header rendering and presentation sort.
- `godot/ui-model/src/guild_note_sort.rs`: existing qualified base comparison.
- `godot/ui-model/tests/guild_numeric_sort_tests.rs`: observable existing-header ties, primary directions/reset, authority and registry conservation.

## Tests asserting this spec

- `guild_numeric_sort_rank_equal_keys_keep_base_order_both_skins`.
- `guild_numeric_sort_level_equal_keys_keep_base_order_both_skins`.
- `guild_numeric_sort_class_equal_keys_keep_base_order_both_skins`.
- `guild_numeric_sort_ties_preserve_registry_scroll_filter_and_original_targets`.
- Existing metadata/header, public Note and Zone tests remain controls for unchanged behavior.

## Known gaps (current cycle)

gn758 passed tests/fmt, including existing metadata/public Note/Zone controls, and landed at3b8b57980. Ignored tests/spec were pushed first at4858919f1; actual reversed-input/header/registry RED4 preceded implementation commit6ef84509b. Focused GREEN4/4 passed (21.44s compile/0.19s tests/no warnings); only ignore attributes were removed afterward, preserving every assertion.
- [ ] Native equal-key proof remains separate from model/registry coverage and requires a new exact-cohort/private-copy/window allocation.
- Full six-way Club presence, Club role/memberId and native name ordering remain absent/unqualified; this is the explicit represented-field policy, not universal Retail parity.

## Out of scope

No Name/Note/Zone behavior changes, new widgets, metadata/wire/server/persistence edits, professions or Communities text. No invented native members/ranks/timestamps, historical string-policy extension or new fallback compatibility paths.
