# PvP progression

Retail Mainline defines behavior/layout in both Modern and Forever art skins.

- Honor1792 and Conquest1602 use the existing currency wallet and durable PvP state. Lifetime honor XP is separate from spendable currency.
- Honor rewards, PvP talent eligibility/slot unlocks and War Mode buff must come from locally extracted pinned Retail12.1.0.69933 records. No numeric reward or season-cap policy may be invented.
- Blizzard_PVPUI/Mainline exposes Quick Match, Rated and Premade Groups categories; honor-level display uses lifetime XP, badge and next achievement reward. War Mode/PvP talents belong to Blizzard_PlayerSpells/PvPTalents, not new Forever-only frames.
- Queue pairing is not a combat result. Pairing alone cannot generate rating, honor or conquest.
- War Mode changes require authoritative rest-area/combat/instance checks and the authored buff. Talent selection requires specialization, level, slot/category and conditions.

## Lifetime XP continuation — 2026-10-16

The authoritative snapshot exposes lifetime honor XP independently of spendable1792. `HONOR_LEVEL_XP_CURVE` remains a named source gap: unknown level is `None`, displayed as an em dash; no wallet15000 substitution for the lifetime maximum, no fabricated radial fill, badge level or reward grant. Retail Mainline HonorInset/128x128 HonorLevelDisplay uses the same XML anchors in both art skins. Its source-owned Lifetime Honor tooltip displays the actual accumulated XP without an invented denominator. This does not certify account-wide aggregation; active70391 LIFETIME_HONOR_DESC says Warband, while pinned TC stores player honor. Account aggregation remains a named gap.

Authority: local `Blizzard_PVPUI/Mainline/Blizzard_PVPUI.xml`804–861,1670–1728 and `.lua`1904–1947,1975–1983. Both-skin native full-HD acceptance remains required; a registry test is not render proof. Reward identity catalogs do not imply progression through an unknown curve.

## Local source export

`scripts/export_db2_csv.py` accepts exact pinned layouts for CurrencyTypes1095531, PvpTalent1302853, PvpTalentCategory1891534, PvpTalentSlotUnlock1950197 and PrestigeLevelInfo1275869. Exported fields describe source records, not runtime implementation guarantees. TrinityCore DB2Structure.h locally preserved in `data/diagnostics/scrap-2026-10-10/` supplies field semantics; CASC supplies values.

## Current source boundary

Honor cap15000 is authored. Conquest has MaxQtyWorldStateID31270 and AwardConditionID146857; static0 does not prove an uncapped season. Slot masks1/2/4/8 unlock at10/20/30/40. PrestigeLevelInfo contains honor-achievement reward identities (e.g. level5/achievement12893); these alone do not define an XP curve. The extracted `gametables/honorlevel.txt` contains old Prestige0..32 curves and is not certified as Mainline lifetime-honor progression. Honor per win/loss, conquest per result, season cap/reset authority and the lifetime XP curve remain gaps until locally derived. No manufactured rewards.

## Native talent controls

War Mode occupies the existing ClassTalentsFrame footer's100x100 control (RIGHT BottomBar -6/+2), using authored ring/swords/orb/indent atlases. PvP talent controls use the same Mainline three58x52 slots (tray RIGHT -114) and220x40 flyout rows in both skins. Flyout ordering follows unlocked/required level/selected elsewhere/ID. Selection is immediate, server-owned, and pending mutations cannot be duplicated; absent permissions never enable a mutation. Source PlayerCondition-unavailable choices remain disabled. Existing TalentError carries server refusal. Normal talents/source loadouts stay separate from PvP selections, which are persisted per specialization in existing PvP progress.

Source authorities: Blizzard_PlayerSpells/PvPTalents/Blizzard_PvPTalentSlotTemplates.xml/.lua and Blizzard_PvPTalentListTemplates.xml/.lua, plus ClassTalents/Blizzard_ClassTalentsFrame.xml325–338. No Forever-only rows/slots/layout are added. Tooltip/drag remain unproved; registry tests do not imply those behaviors.

## Bounded acceptance — 2026-10-11

The landed WSG frame now has Quick Match, Rated and Premade Groups categories in both skins. Quick Match shows authoritative Honor wallet quantity/cap; Rated shows Conquest wallet quantity, not season-earned progress. Unknown rating and season-cap fields are `None`, never fabricated1500/1800. Premade listing and rated-match services remain explicitly unavailable.

Native private UDP5790, `fb_pvp2`/Pvpproof: both skins prove real pointer War Mode toggling in Stormwind rest area5148, Enlisted282559 replication, source Holy talent86/640 selection and wallet display. A process relog proves retained desired mode and talent86. Inspected PNGs: `/syncthing/AgentShared/2026-10-11/pvp-progression/`; receipts: `data/diagnostics/pvp-progression-2026-10-11/`. Fresh-character live balances are0; nonzero750/120 display is separately covered by both-skin registry tests, not a claim of earned live rewards.

Honor-level/badge/next-reward display and lifetime leveling remain blocked by the uncertified XP curve; importing reward identities alone does not establish a character's level. Per-match awards, season policy and rating progression remain data gaps. Enlisted XP/quest-XP amount consumers and talent tooltip/drag have no acceptance proof. This is bounded progression infrastructure, not complete Retail parity.
