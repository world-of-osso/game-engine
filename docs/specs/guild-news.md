# Guild news

Retail Mainline guild news in Modern and Forever. Installed sources: `Blizzard_Communities/GuildNews.lua`, `GuildNews.xml`, `CommunitiesFrame.xml:171–177`, `Blizzard_FrameXML/GuildUtil.lua`; active `GlobalStrings.csv`. Real producer conditions: TrinityCore a352b1fa `AchievementMgr.cpp:475`, `Player.cpp:23904,27194`, `SpellEffects.cpp:1452`, `Guild.h:260`. [Data flow](../wiki/systems/guilds.md#news).

## What it must do

- [x] Info renders the source316px news pane, header/link,300×18 rows, scroll list and MinimalScrollBar in both skins. Current nonempty real MOTD appears first; no synthetic welcome/history rows.
- [x] Display only actual source-qualified player achievements and item loot/craft/purchase events, with real names/IDs/metadata and source date/sticky ordering. Retain250 durable news rows, independent of lifecycle and bank logs.
- [x] Queries use authenticated current membership. Sticky writes require the source CanEditMOTD right, preserve immutable entry identity and wait for authoritative replies. Nonmembers receive no news.
- [x] Source filter dialog toggles actual news categories; source row tooltip/context-menu actions use real metadata. Closing, membership loss and late replies preserve privacy and independent request pacing.
- [x] Real private UDP actions, process restart and relog preserve history/stickies; inspect1920×1080 Modern/Forever captures against source XML.

## How it works

- [Guild systems](../wiki/systems/guilds.md#news).

## Implementation inventory

| File | Role |
|---|---|
| `godot/ui-model/src/guild_news.rs` | Source news pane, real rows/dates/MOTD, filter dialog and context/sticky controls |
| `godot/ui-model/src/guild_rank_frame.rs` | Info composition |

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs::guild_news_info_renders_source_controls_and_only_real_motd_both_skins` — RED reproduces absent news header, source controls and real MOTD display.
- `godot/tests/guild_news_live.gd` + `godot/network/examples/native_guild_news_peer.rs` — private UDP5891 actual purchases/sales, native filters/sticky ACKs,250 retained rows, wheel overflow, permissions/privacy and fresh-process readback; no injected news.

## Known gaps (current cycle)

- [x] Actual server producers, authenticated query/sticky and durable storage; backend6/6.
- [x] Source filter dialog, actual rows/tooltips/context actions and independent news authority; post-merge model4/4. Private native query/filter/sticky and both-skin250-row physical overflow pass; actual ordinary-member sticky refusal and post-quit privacy pass.
- [x] Source440557/446203 extracted locally and published by lead; current-artifact controls and fresh-process exact readback pass in both skins at engine5e1661761/server5001d9d1/protocol9bb88b6. Sixteen inspected full-HD PNGs; proof ledger in `/home/osso/.worktrees/logs/guild-news-2026-10-15-run1/`.
- [ ] Independent lead integration gate/landing.

## Out of scope

Guild-achievement, dungeon-encounter and calendar news lack guild producers; never fabricate them from player achievements, ordinary NPC kills or invented dates. Perks/rewards and invitation tabards remain separate slices.
