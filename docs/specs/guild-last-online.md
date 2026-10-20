# Guild Last Online recency

Existing roster offline Zone cells and member detail Last Online fields display actual elapsed recency. Retail GuildRoster.lua115–120, CommunitiesMemberList.lua1285–1297 and TimeUtil.lua24–43 define the behavior: elapsed years/months/days/hours, otherwise less than an hour, not a calendar date. Authenticated active local-CASC GlobalStrings.csv rows10842/11461/11518–11520 (12.1.5.70448, FDID1394440) supply English plural text.

## What it must do

- [x] Render optional authoritative observed logout time as source recency in both existing fields, identically in both skins.
- [x] Unknown offline history displays Offline; online displays Online/current zone even if a stale offline timestamp was supplied.
- [x] Preserve source controls/layout, member identity and saved filtering; no synthetic timestamp from local receipt/query time.

## How it works

- [Guild system](../wiki/systems/guilds.md).

## Implementation inventory

- `godot/ui-model/src/guild_communities.rs`, `guild_member_detail.rs`: existing display fields.
- `godot/ui-model/tests/guild_rank_widgets.rs`: both-skin elapsed units/unknown/online regression.

## Tests asserting this spec

- `guild_last_online_elapsed_recency_and_unknown_online_fields_both_skins` (enabled after actual rendering RED→GREEN).

## Known gaps (current cycle)

None within the approved slice. gl728 landed protocol50517c9/server1a2f13fd4/engine51f1af4f8; actual two-member disconnect, both-skin recency, reconnect and fresh-server/client readback were accepted. See [native acceptance](../wiki/systems/guilds.md#last-online-native-acceptance-2026-10-19). Numeric epoch bounds, unknown-history provenance and large elapsed-unit boundaries retain automated proof, not backdated native actor state.

## Out of scope

Sorting, extra UI/geometry, mobile/BNet/remote chat, inactivity dethroning, professions and leadership/self-policy changes.
