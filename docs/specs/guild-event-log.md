# Guild lifecycle log

Retail Mainline's member-only lifecycle history in Modern and Forever. Sources: installed `Blizzard_Communities/GuildInfo.lua:158–195`, `GuildInfo.xml:385–459`, `CommunitiesFrame.xml:589–597`, `CommunitiesFrame.lua:549–554`; active `GlobalStrings.csv` names its title **Log** and footer action **View Log**. [Data flow](../wiki/systems/guilds.md#lifecycle-log).

## What it must do

- [ ] Info alone exposes View Log; opening queries the authenticated player's current guild, with no selectable guild/actor ID. No elevated rank is required.
- [ ] Render source384×432 TranslucentFrameTemplate, tooltip container and real scrolling text. ScrollFrameTemplate wheel/steppers move30px per source ScrollUtil.lua:203; the oldest retained entry must become fully visible and clamp at the end. Close button or Escape closes the log independently of Communities.
- [ ] Render only real invite/join/promote/demote/remove/quit entries newest first, with captured event-time names/rank and source relative-time suffix. Empty real history stays empty; no sample/welcome/backfilled rows.
- [ ] Preserve query waiting independently of pending rank/note authority; typed failures display their real refusal and no history. Logout/removal clears member-only data; late replies never reopen a closed log.
- [ ] Actual private UDP lifecycle actions, query and process restart yield the same persisted history in both skins, with inspected1920×1080 PNGs.

## How it works

- [Native guild lifecycle flow](../wiki/systems/guilds.md#lifecycle-log).

## Implementation inventory

| Files | Role |
|---|---|
| `godot/ui-model/src/guild_ranks.rs`, `guild_event_log.rs` | Independent query/open/cache state and source log rendering |
| `godot/rust/src/account.rs`, `lib.rs`, `guild_ranks.rs`, `ui/guild_event_log_preview.rs` | Typed native flow and debug read-only rendered-reply receipt |
| `godot/network/src/lib.rs`, `examples/native_guild_lifecycle_peer.rs` | Typed receiver and actual private invitation/quit/permission target |
| shared-protocol `src/protocol/guild_rank_messages.rs` | Query and six-kind event payload |
| `godot/tests/guild_lifecycle_live.gd` | Both-skin physical UI mutations, genuine overflow and raw-reply restart comparison |

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs::guild_event_log_*` — source entry/dialog RED→GREEN, real text/order, pending-authority independence, late close/membership cleanup and six actual GlobalStrings sentences.
- `godot/network/src/guild_rank_wire_tests.rs::guild_event_log_wire_preserves_six_kinds_order_and_typed_refusal` — actual UDP query, all six payload kinds, insertion order and typed refusal pass1/1 at42aff7a54/protocol4fad8c7.
- `guild_lifecycle_live.gd` + `native_guild_lifecycle_peer` — pending: each skin performs actual invite/join/promote/demote/confirmed-remove/reinvite/quit; normal-member read/refused promotion and post-quit privacy use real UDP. Another100 physical rank changes create a genuine second page. A fresh process/server must preserve the exact100 raw events/timestamps and both-skin rendering.
- UI/source subset5/5 passes at3c327b62e/protocol4fad8c7, including independent authority/late reply behavior and unchanged shared invitation chrome.
- `godot/rust/src/ui/scroll_lists_tests.rs::guild_lifecycle_log_scroll_moves_source_pan_and_fully_reveals_oldest` — native controller/visible geometry RED reproduces missing30px stride; GREEN pending.

## Known gaps (current cycle)

- [ ] Source dialog/list and atomic server lifecycle producer GREEN.
- [ ] End-to-end acceptance and merged-head gates. First private run completed both skins' eight-action sequences and five inspected PNGs, but hit900s during100 physical rank changes. Replace inherited ten-frame settling sleeps with actual drawn-frame readiness; preserve all100 actions/overflow/restart assertions under a bounded1800s functional watchdog.

## Out of scope

News, perks and rewards are subsequent separately landed slices. Existing bank history is not a substitute for this log.
