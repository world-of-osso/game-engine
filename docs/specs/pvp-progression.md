# PvP progression

Retail Mainline defines behavior/layout in both Modern and Forever art skins.

- Honor1792 and Conquest1602 use the existing currency wallet and durable PvP state. Lifetime honor XP is separate from spendable currency.
- Honor rewards, PvP talent eligibility/slot unlocks and War Mode buff must come from locally extracted pinned Retail12.1.0.69933 records. No numeric reward or season-cap policy may be invented.
- Blizzard_PVPUI/Mainline exposes Honor, Rated and War Games tabs; honor-level display uses lifetime XP, badge and next achievement reward. War Mode/PvP talents belong to Blizzard_PlayerSpells/PvPTalents, not new Forever-only frames.
- Queue pairing is not a combat result. Pairing alone cannot generate rating, honor or conquest.
- War Mode changes require authoritative rest-area/combat/instance checks and the authored buff. Talent selection requires specialization, level, slot/category and conditions.

## Local source export

`scripts/export_db2_csv.py` accepts exact pinned layouts for CurrencyTypes1095531, PvpTalent1302853, PvpTalentCategory1891534, PvpTalentSlotUnlock1950197 and PrestigeLevelInfo1275869. Exported fields describe source records, not runtime implementation guarantees. TrinityCore DB2Structure.h locally preserved in `data/diagnostics/scrap-2026-10-10/` supplies field semantics; CASC supplies values.

## Current source boundary

Honor cap15000 is authored. Conquest has MaxQtyWorldStateID31270 and AwardConditionID146857; static0 does not prove an uncapped season. Slot masks1/2/4/8 unlock at10/20/30/40. PrestigeLevelInfo contains honor-achievement reward identities (e.g. level5/achievement12893); these alone do not define an XP curve. The extracted `gametables/honorlevel.txt` contains old Prestige0..32 curves and is not certified as Mainline lifetime-honor progression. Honor per win/loss, conquest per result, season cap/reset authority and the lifetime XP curve remain gaps until locally derived. No manufactured rewards.
