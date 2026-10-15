# Guild lifecycle log

Retail Mainline's member-only lifecycle history in Modern and Forever. Sources: installed `Blizzard_Communities/GuildInfo.lua:158–195`, `GuildInfo.xml:385–459`, `CommunitiesFrame.xml:589–597`, `CommunitiesFrame.lua:549–554`; active `GlobalStrings.csv` names its title **Log** and footer action **View Log**. [Data flow](../wiki/systems/guilds.md#lifecycle-log).

## What it must do

- [ ] Info alone exposes View Log; opening queries the authenticated player's current guild, with no selectable guild/actor ID. No elevated rank is required.
- [ ] Render source384×432 TranslucentFrameTemplate, tooltip container and real scrolling text; close button or Escape closes the log independently of Communities.
- [ ] Render only real invite/join/promote/demote/remove/quit entries newest first, with captured event-time names/rank and source relative-time suffix. Empty real history stays empty; no sample/welcome/backfilled rows.
- [ ] Preserve query waiting independently of pending rank/note authority; typed failures display their real refusal and no history. Logout/removal clears member-only data; late replies never reopen a closed log.
- [ ] Actual private UDP lifecycle actions, query and process restart yield the same persisted history in both skins, with inspected1920×1080 PNGs.

## How it works

- [Native guild lifecycle flow](../wiki/systems/guilds.md#lifecycle-log).

## Implementation inventory

| Files | Role |
|---|---|
| `godot/ui-model/src/guild_ranks.rs`, `guild_event_log.rs` | Independent query/open/cache state and source log rendering |
| `godot/rust/src/account.rs`, `lib.rs`, `guild_ranks.rs` | Typed GuildChannel command/reply to native session |
| `godot/network/src/lib.rs` | Receives the typed event-log message |
| shared-protocol `src/protocol/guild_rank_messages.rs` | Query and six-kind event payload |

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs::guild_event_log_opens_from_info_and_closes_independently_both_skins` — source entry/dialog RED reproduced; GREEN pending.
- `guild_event_log_renders_real_newest_entries_and_keeps_rank_authority_independent_both_skins` — actual reply text/order, pending-authority independence, late close and membership cleanup.
- `guild_event_log_formats_only_the_six_source_lifecycle_kinds` — active GlobalStrings sentences.
- `godot/network/src/guild_rank_wire_tests.rs::guild_event_log_wire_preserves_six_kinds_order_and_typed_refusal` — actual UDP query and all six payload kinds; pending run.
- Overflow scrolling and native/restart tests remain pending.
- UI/source subset5/5 passes at3c327b62e/protocol4fad8c7, including independent authority/late reply behavior and unchanged shared invitation chrome.

## Known gaps (current cycle)

- [ ] Source dialog/list and atomic server lifecycle producer GREEN.
- [ ] End-to-end acceptance and merged-head gates.

## Out of scope

News, perks and rewards are subsequent separately landed slices. Existing bank history is not a substitute for this log.
