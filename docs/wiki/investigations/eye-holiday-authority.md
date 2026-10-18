# Eye7 holiday492 authority — read-only70448 audit

Verified2026-10-17. **Interval evaluation is sourced; the effective holiday492 schedule is not established.** Named gap: `HOLIDAY492_EFFECTIVE_EVENT_AUTHORITY`. No calendar policy, normal/inactive default, runtime code or RED payout test was adopted. Existing Eye completion Honor remains separate from this resource-threshold gap.

## Authenticated installed data

Installed Retail12.1.5.70448 build key `88d5e8ef05ec5a8446cdec93d142d7c6`, root content key `e84483ee756c69a7dc5fbff04af81316`. The published local-CASC export's CSV hashes/row counts, installed build-config SHA and exact pinned WoWDBDefs schema SHA were verified. Holidays FDID996357/layout432931C0 has811 rows and0 unreadable records; all seven selected supporting tables are readable. No canonical data was changed or re-exported.

Literal representation is one `Holidays` record with `Date[26]` and `Duration[10]` arrays. Separate `HolidayDates`/`HolidayDuration` tables are absent from the pinned manifest and installed exported inventory; they must not be invented as additional modeled tables.

| Holiday492 field | Installed value |
|---|---|
| `Date` |128366592 (`0x07A6B800`), then25 zeros |
| `Duration` |72,1776 hours, then8 zeros |
| `Looping` / `CalendarFilterType` |0 /2 |
| `CalendarFlags` |3, then9 zeros |
| `Region` / `Flags` |0 /1 |

`HolidayNameID=24` resolves to “Call to Arms: Eye of the Storm.” Pinned `WowTime::SetPackedTime` decodes Date0 as fixed **November27,2007 00:00 UTC**, not a Unix epoch integer and not an unspecified-year recurrence. `GetUnixTimeFromUtcTime` uses year+2000, zero-based month/day and `timegm`. The misleading epoch-style comment in DB2Structure does not override the actual decoder.

BattlemasterList7 is Eye of the Storm, broad admission20–90, `HolidayWorldState=2851`; BattlemasterListXMap370 links it to map566. PVPDifficulty supplies nine map566 ranges:20–29,30–39,40–49,50–59,60–69,70–79,80,81–89,90. Pinned TC maps type7 to `HOLIDAY_CALL_TO_ARMS_ES=492`. Installed HolidayXTimeEvent has only holiday1405→event901, not a replacement492 schedule.

## Pinned TC semantics

`GameEventMgr::LoadFromDB` reads `game_event` start/end, occurrence, length, holiday, holidayStage and state. For normal events, `CheckOneGameEvent` uses:

```text
start < now && now < end
&& ((now - start) % (occurrence_minutes * 60)) < length_minutes * 60
```

The absolute first start is excluded; later periodic windows begin at remainder0 and exclude their length boundary, subject to the absolute bounds. `IsHolidayActive` reads authoritative active-event membership, not direct membership in the DB2 Date array. Forced/world-event state is consequently not reducible to a guessed calendar alone.

`SetHolidayEventTime` returns immediately for holidayStage0. Otherwise it selects `Duration[stage-1]`, offsets by preceding stage durations, and processes dates. Filter2 leaves occurrence unchanged; `Looping=0` skips duration-sum recurrence. Date0 has a fixed2007 year and no later nonzero entries, so at the audit date it supplies no future candidate; TC retains the SQL start/period. **Summing72+1776 into a77-day repeating calendar is not this source behavior.** Neither does an old client date prove current inactivity.

Clock details must stay literal: the normal predicate uses epoch game time; date selection compares the system-zone-adjusted `GameTime::GetWowTime`, while a chosen packed start converts through UTC `timegm`. No realm/DB timezone policy was inferred.

The Eye script constructor samples weekend state once and selects160 versus260. It does not dynamically switch the threshold on each point tick. `AddPoint` adds source points to the team's Honor remainder, grants one bonus-kill equivalent when at/above the selected threshold, then subtracts that threshold once per call.

## Named authority gap / implementation boundary

`HOLIDAY492_EFFECTIVE_EVENT_AUTHORITY`: source the effective492 game-event row, holidayStage, start/end/occurrence and applicable state/override/clock authority before implementing active/inactive resource-Honor branches. A read-only schema inventory of current world.db found event-spawn link tables but **no `game_event` schedule table**. Spawn membership does not establish event timing.

The evaluator arithmetic and DB2 payload are known; the actual schedule feeding that evaluator is not. No260 default,77-day schedule, arbitrary active dates or unsupported calendar service was introduced. The requested260/160 RED with real active/inactive boundaries remains blocked on this authority, not waived or replaced by synthetic dates.

## Sources

- [Installed-source receipts](../../../data/diagnostics/pvp-holiday-authority-2026-10-17/installed-source-receipt.json) — build identity, seven verified CSV/layout/content receipts.
- [Finding and date decode](../../../data/diagnostics/pvp-holiday-authority-2026-10-17/finding.json), [packed-date receipt](../../../data/diagnostics/pvp-holiday-authority-2026-10-17/packed-date-receipt.json) and [table inventory](../../../data/diagnostics/pvp-holiday-authority-2026-10-17/holiday-table-inventory.json) — concrete492 row, absence scope and read-only world schema inventory.
- [Pinned TC artifacts/symbol lines](../../../data/diagnostics/pvp-holiday-authority-2026-10-17/tc-source-receipts.json) — a352b1fa GameEventMgr, BattlegroundMgr, SharedDefines, Eye script, WowTime and GameTime; exact file hashes/URLs.
- [Pinned schema downloads](../../../data/diagnostics/pvp-holiday-authority-2026-10-17/source-download-receipts.json) — WoWDBDefs d6f50d4 and decoder source hashes; Holidays schema matches the authenticated export.

## See Also

- [PvP progression contract](../../specs/pvp-progression.md) — existing payout boundaries and unresolved authority.
- [Native battlegrounds](../systems/battlegrounds.md) — source brackets, established payouts and bounded release20 proof.
