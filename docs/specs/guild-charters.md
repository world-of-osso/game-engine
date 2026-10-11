# Retail guild charters

Guild creation uses Mainline `GuildRegistrarFrame` and `PetitionFrame` in both skins; Forever changes art only.

## What it must do

- Authentic petitioner NPCs (flag262144) offer purchase and registration services.
- Purchase sends the entered guild name and actual NPC to authority; price comes from the registrar snapshot. No optimistic money/items/petition changes.
- Using owned Guild Charter5863 opens actual title, owner and authoritative signature quota/names.
- Owners request signatures from targeted players and rename through the Retail rename popup. Recipients may sign only when authority permits.
- Busy requests disable repeated input; refusals are visible. Successful purchases close the registrar; creation closes charter UI and refreshes guild state.
- Registrar closure and logout clear native windows; Loading must not discard early replies.
- Real server owns distinct signing accounts, permissions, money/inventory, petitions and durable guild creation. No world.db mutations or manufactured content.

## How it works

Native `guild_charters.rs` drives the dedicated registry canvas; portable decisions live in ui-model `guild_charters.rs`, art/layout in `guild_charter_frame.rs`.

## Tests

Real UDP charter reply1/1, native account dispatch1/1, portable purchase/rename/sign/register and both-skin quota/controls5/5. Private UDP54381: authentic Aldwin Laughlin → purchase5863 → bag use → rename → four distinct signing accounts → registration → five founding memberships and charter consumption, in Modern and Forever. Purchase/signatures/rename survive reopen; created guild/profile/date survive restart. Inspected PNGs: `/syncthing/AgentShared/2026-10-11/guilds/guilds4-*.png`; exact commands/results: canonical `data/diagnostics/guilds4-2026-10-11/`.

## Sources

Local Retail `Blizzard_UIPanels_Game/Mainline/{GuildRegistrarFrame,PetitionFrame}.{lua,xml}`; TrinityCore `a352b1fa` `PetitionsHandler.cpp` and realm defaults (1000 copper, four signatures). [Server authority contract](../../../game-server/docs/specs/guild-charters.md).
