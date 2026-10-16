# Guild news

Retail Mainline guild news in Modern and Forever. Installed sources: `Blizzard_Communities/GuildNews.lua`, `GuildNews.xml`, `CommunitiesFrame.xml:171–177`, `Blizzard_FrameXML/GuildUtil.lua`; active `GlobalStrings.csv`. Real producer conditions: TrinityCore a352b1fa `AchievementMgr.cpp:475`, `Player.cpp:23904,27194`, `SpellEffects.cpp:1452`, `Guild.h:260`. [Data flow](../wiki/systems/guilds.md#news).

## What it must do

- [ ] Info renders the source316px news pane, header/link,300×18 rows, scroll list and MinimalScrollBar in both skins. Current nonempty real MOTD appears first; no synthetic welcome/history rows.
- [ ] Display only actual source-qualified player achievements and item loot/craft/purchase events, with real names/IDs/metadata and source date/sticky ordering. Retain250 durable news rows, independent of lifecycle and bank logs.
- [ ] Queries use authenticated current membership. Sticky writes require the source CanEditMOTD right, preserve immutable entry identity and wait for authoritative replies. Nonmembers receive no news.
- [ ] Source filter dialog toggles actual news categories; source row tooltip/context-menu actions use real metadata. Closing, membership loss and late replies preserve privacy and independent request pacing.
- [ ] Real private UDP actions, process restart and relog preserve history/stickies; inspect1920×1080 Modern/Forever captures against source XML.

## How it works

- [Guild systems](../wiki/systems/guilds.md#news).

## Implementation inventory

| File | Role |
|---|---|
| `godot/ui-model/src/guild_news.rs` | Source news pane, real rows/dates/MOTD, filter dialog and context/sticky controls |
| `godot/ui-model/src/guild_rank_frame.rs` | Info composition |

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs::guild_news_info_renders_source_controls_and_only_real_motd_both_skins` — RED reproduces absent news header, source controls and real MOTD display.

## Known gaps (current cycle)

- [x] Actual server producers, authenticated query/sticky and durable storage; backend6/6.
- [x] Source filter dialog, actual rows/tooltips/context actions and independent news authority; model4/4. Native interaction/pixels remain unproven.
- [ ] Both-skin real UDP/process-restart/pixel acceptance and independent integration gates.

## Out of scope

Guild-achievement, dungeon-encounter and calendar news lack guild producers; never fabricate them from player achievements, ordinary NPC kills or invented dates. Perks/rewards and invitation tabards remain separate slices.
