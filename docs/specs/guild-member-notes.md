# Guild member details and notes

Retail Mainline defines mechanics for both Modern and Forever. Sources: installed `Blizzard_Communities/GuildRoster.lua`, `GuildRoster.xml`, `Blizzard_StaticPopup_Game/GameDialogDefs.lua:1834` and pinned TrinityCore a352b1fa `Guild.cpp:1597`.

- [x] Member detail shows authoritative name, class/level, rank, public note and permitted officer note; unknown zone and exact last-online times remain blank/unknown, not invented.
- [x] Public notes are distinct from officer notes and populate the roster Note column.
- [x] A member can edit their own public note (Mainline Lua); other public writes require public-note permission. Officer writes require officer-note permission; the native editor additionally requires view permission.
- [x] Authenticated actor and same-guild target are revalidated server-side;31 Unicode characters maximum, no control characters. Popup accepts/cancels without optimistic profile replacement.
- [x] Public notes persist separately from existing bitcode officer-note records, so preexisting officer notes remain readable. Leaving/removal deletes both notes before rejoin.
- [x] Officer-note data remains stripped from unauthorized recipients. Group Invite uses the actual group service; member Remove uses existing hierarchy authority.
- [ ] Direct rank-assignment dropdown, online zone and exact last-online elapsed metadata remain gaps.
- [ ] Targeted tests, current checks and both-skin private5891 live/relog PNG proof are pending; source implementation is not acceptance.

Wire: `GuildRankRequest::SetMemberNote`, `GuildMemberSnapshot::{public_note,is_self}`. Changed peers must land together. Public edit permission follows pinned TC0x2000, distinct from view/edit officer rights0x4000/0x8000.
