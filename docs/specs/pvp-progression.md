# PvP progression

Retail Mainline defines behavior/layout in both Modern and Forever art skins.

- Honor1792 and Conquest1602 use the existing currency wallet and durable PvP state. Lifetime honor XP is separate from spendable currency.
- Honor rewards, PvP talent eligibility/slot unlocks and War Mode buff must come from locally extracted pinned Retail12.1.5.70391 records. No numeric reward or season-cap policy may be invented.
- Blizzard_PVPUI/Mainline exposes Quick Match, Rated and Premade Groups categories; honor-level display uses lifetime XP, badge and next achievement reward. War Mode/PvP talents belong to Blizzard_PlayerSpells/PvPTalents, not new Forever-only frames.
- Queue pairing is not a combat result. Pairing alone cannot generate rating, honor or conquest.
- War Mode changes require authoritative rest-area/combat/instance checks and the authored buff. Talent selection requires specialization, level, slot/category and conditions.

## Lifetime XP continuation — 2026-10-16

The authoritative snapshot exposes lifetime honor XP independently of spendable1792. `HONOR_LEVEL_XP_CURVE` remains a named source gap: unknown level is `None`, displayed as an em dash; no wallet15000 substitution for the lifetime maximum, no fabricated radial fill, badge level or reward grant. Retail Mainline HonorInset/128x128 HonorLevelDisplay uses the same XML anchors in both art skins. Its source-owned Lifetime Honor tooltip displays the actual accumulated XP without an invented denominator. This does not certify account-wide aggregation; active70391 LIFETIME_HONOR_DESC says Warband, while pinned TC stores player honor. Account aggregation remains a named gap.

Authority: local `Blizzard_PVPUI/Mainline/Blizzard_PVPUI.xml`804–861,1670–1728 and `.lua`1904–1947,1975–1983. Both-skin native full-HD acceptance remains required; a registry test is not render proof. Reward identity catalogs do not imply progression through an unknown curve.

## Bounded lifetime XP acceptance — 2026-10-16

Private UDP5941, fb_pvpprog/Honorproof, source level30 versus opposing level30: private admin normal-damage path produces an actual player death, server lifetime0→47, native Forever tooltip47, fresh Modern process retains47, then another actual death produces94. Dead-target replay does not add another award. Both skins render source128px faction ring, unknown `Honor Level —`, and separate spendable Honor0/15000; no lifetime denominator/fill or reward is invented. Native input also selects sourced Arms Disarm3534 in Forever; Modern relog retains it. Pointer War Mode enable/disable projects desired/enlisted identities in both skins; this run does not independently inspect applied aura contents.

Fifteen inspected1920x1080 originals (including one explicitly named failed/dead-target diagnostic) live at `/syncthing/AgentShared/2026-10-16/pvp-progression/`; receipts, source hashes and exact revision/proof ledger at `data/diagnostics/pvp-progression-2026-10-16/`. These are existing private-admin damage stimuli, not native player-controlled PvP combat certification. Runtime source: engine1107f3f7e (later wiki-only90ff3656f), serverc5d4e9cd, protocol7f16f326. Server10/UI4/wire1 targeted tests and scoped Rust checks/fmt pass. No broader parity claim.

At lifetime-XP acceptance, Character Currency was a real implementation gap: native IPC returned `unported request CurrenciesStatus`, the native account did not consume CurrencyStateUpdate, and only Character/Reputation tabs rendered. Fresh wallets remain0; no award was fabricated to enable a currency tab. The Currency continuation below supersedes that implementation gap; the lifetime-XP captures alone did not certify it. Match-completion payout binding, current season-cap authority, rated-BG reference policy and account-wide aggregation remain separate named gaps.

## Native Currency consumer — continuation

The default native network bridge must relay the existing owner-only CurrencyStateUpdate. The account keeps the last supplied snapshot, distinguishes no receipt from an empty wallet, retains the last snapshot on a refusal with no replacement, and clears it on new connections/world resets. Currency IPC status exposes that authoritative cache, not PvP lifetime XP. Message/error feedback uses normal chat/UI error delivery. Character Currency/TokenFrame presentation requires a separate native gate.

For native presentation proof only, the parent approved a private server-owned saved discovered Honor1792 entry of quantity0. This is neither an earned balance nor an award. Fresh empty-wallet receipt/hidden-tab behavior must also be proved. No cap, reward, transfer or preference mutation policy is invented.

## Bounded Currency presentation

The discovered PvP wallet exposes Mainline CharacterFrameTab3 and TokenFrame in both skins: source bottom tabs, Player vs. Player category, authored header/entry art, received Honor/Conquest names and quantities, category collapse/re-expand, and owned hover tooltips. Honor uses the source15000 maximum; Conquest's unresolved dynamic cap is not replaced by zero or a guessed weekly maximum. Fresh empty-wallet receipts hide Currency. Existing undiscovered-wallet character chrome is outside this continuation; new discovered-wallet tabs must not inherit Forever side-tab geometry.

Authority: local Mainline CharacterFrame.xml159–184, Blizzard_TokenUI.xml/.lua, SharedXML/ListTemplates.xml and active70391 CurrencyTypes/CurrencyCategory records. Unknown currency metadata fails explicitly. Watched/unused preferences, transfer, details/settings controls, filters and non-PvP currencies remain unimplemented; this bounded readout does not claim full TokenUI parity. Final native acceptance: engine0fad631db/server0083ef232/protocol710590dbc, privateUDP5941, two fresh processes prove the approved server-saved discovered Honor1792 quantity0, source bottom Currency tab, gray quantity0 row, owned full tooltip, pointer collapse/re-expand and unchanged authoritative IPC wallet. Eleven frame rectangles match across skins. Six inspected1920×1080 final PNGs (`*-currency-source-final-*`) plus earlier both-skin empty-wallet hidden-tab captures live in `/syncthing/AgentShared/2026-10-16/pvp-progression/`; exact receipts/hashes/commands and nine behavioral tests (UI7/wire1/account1), source checks/builds/fmt are in the diagnostic proof ledger. Pre-fix Forever side-tab captures are explicitly named `legacy-tab-diagnostic`, not acceptance. Source-level and native proof are presentation-only, not earned currency or reward policy.

## Earned-wallet continuation — 70391

Lead-authorized World of Osso policy [HONOR_WALLET_MIRRORS_HONOR_XP](../../../game-server/docs/specs/pvp-progression.md#world-of-osso-policy-honor_wallet_mirrors_honor_xp) explicitly mirrors implemented TC-derived Honor awards into1792 while preserving separate lifetime XP. This is our event binding, not a TC wallet-mutation claim. Currency/PvP views must show received wallet amounts after real kills, keep the authored holding cap and zero-weekly behavior, and leave honor level unknown. Arena/Conquest1602 quantities remain unmapped under ARENA_COMPLETION_WALLET_BINDING; Korrak1166 is excluded. The [source audit](../../../game-server/docs/specs/pvp-progression.md#earned-wallet-award-binding-audit--70391) preserves provenance and remaining season/eligibility gaps. Native mirror acceptance on private5941: Modern actual lethal damage increases wallet0→47 and separate XP94→141; fresh Forever retains both, then another actual death increases wallet47→94 and XP141→188. Dead-target replays preserve both; another fresh process retains94/188. Both Currency rows and full Honor tooltips show received amounts and authored15000 cap. Four inspected1920×1080 originals (`{modern,forever}-earned-honor-{wallet,tooltip}.png`) are in the existing AgentShared directory; exact snapshots, damage receipts and scoped GREEN9/check/build evidence are in `data/diagnostics/pvp-progression-2026-10-16/honor-wallet-mirror/`. This proves private-admin normal-damage kills, not native player-controlled combat or unimplemented BG callbacks. Prior Currency screenshots remain presentation-only.

## Local source export

`scripts/export_db2_csv.py` accepts exact pinned layouts for CurrencyTypes1095531, PvpTalent1302853, PvpTalentCategory1891534, PvpTalentSlotUnlock1950197 and PrestigeLevelInfo1275869. Exported fields describe source records, not runtime implementation guarantees. TrinityCore DB2Structure.h locally preserved in `data/diagnostics/scrap-2026-10-10/` supplies field semantics; CASC supplies values.

## Random Battleground reward profile — 70448

`RANDOM_BG_WIN_REWARD_PROFILE` remains unresolved. Pinned TC a352b1fa compiled defaults and shipped assignments are first/repeat win27000/13500 and loss4500/3500, passed as kill equivalents rather than flat Honor. Daily reset defaults to06:00 server-local time; neither amounts nor reset authority is adopted by this project. No first-win/daily state or Random completion grant is authorized.

**`RANDOM_BG_LEVEL_ELIGIBLE_MAPS` — project policy based on Retail level-banded Random BG behavior, not a TC port.** Admission uses parent map566's bracket (first20–29), parent minimum5/team, capacity10/team and party≤5. Each player's original queue32 survives selection, transfer and reconnect separately from the concrete map.

Select only configured BML32 maps with their own PVPDifficulty bracket containing all queued player levels and satisfying their admission bounds. Use existing configured weights; Retail weighting is undocumented. Missing Deepwind2245 template excludes it; level25 excludes Deephaul2656, while level80 can select it. An ineligible-only level range is refused explicitly. Retain the selected level-matching bracket, not a copied TC RangeIndex. Existing map payouts and concrete-map start minimums stay unchanged. UI map exclusions are a later slice.

## Fill existing battleground copies

Fill compatible Waiting/InProgress copies before creating a new one: concrete map and selected player-level range must match; rated/arena and Ended copies are excluded. Random32 eligibility is unchanged, with no redraw. Keep every character's original direct/random queue ID, including mixed-origin copies. Outstanding invitations/offline reservations consume concrete-map capacity.

**`BG_BACKFILL_WHOLE_PARTY_SMALLER_TEAM` — project policy:** whole parties join the smaller team only if they fit its remaining capacity; never split or drop a nonfit. This replaces TC's one-player-at-parity limiter. **`BG_BACKFILL_MATCH_ORDER` — project policy:** deterministic ascending existing match-ID order, not TC push-front order.

Preserve existing copy, phase, score, clock and objective state. New characters receive normal invitations and reservation ownership, then ordinary acceptance/WorldPortAck entry; existing accepted players are not re-invited. Use landed late-arrival score-row insertion. Refill on queue/capacity-release events only; no timer, reward or minimum changes. Arathi5v5 Waiting must reach its unchanged8/team criterion naturally through later parties.

### Native pending invitation

An unaccepted Waiting entry or an unaccepted InProgress ordinary BG entry displays the existing ready dialog and permits one match-owned accept/decline request until authority responds. Accepted entries and Ended matches are not pending; running-arena behavior remains unchanged. Preserve existing dialog art/layout and response handling. Source: Retail `PVPHelper.lua`321–329 opens on queue status `confirm`;427 handles the pending port action independently of the destination match phase.

## Eye holiday authority — 70448 read-only

`HOLIDAY492_EFFECTIVE_EVENT_AUTHORITY` remains unresolved. Do not infer a77-day recurrence or treat missing calendar authority as inactive. Existing Eye completion Honor is unchanged; resource-Honor selection remains blocked.

- [ ] Establish the sourced effective holiday492 event/stage/schedule and active-state authority before choosing Eye7 resource thresholds260/160.
- [ ] Prove real active/inactive interval boundaries before implementing that conditional payout; do not manufacture dates or adopt an unsupported calendar policy.

The [70448 read-only audit](../wiki/investigations/eye-holiday-authority.md) records the concrete DB2 arrays, pinned TC evaluator semantics and the missing effective-event input.

## Current source boundary

Honor cap15000 is authored. Conquest has MaxQtyWorldStateID31270 and AwardConditionID146857; static0 does not prove an uncapped season. Slot masks1/2/4/8 unlock at10/20/30/40. PrestigeLevelInfo contains honor-achievement reward identities (e.g. level5/achievement12893); these alone do not define an XP curve. The extracted `gametables/honorlevel.txt` contains old Prestige0..32 curves and is not certified as Mainline lifetime-honor progression. Honor per win/loss, conquest per result, season cap/reset authority and the lifetime XP curve remain gaps until locally derived. No manufactured rewards.

## Ready-dialog placement

The standalone ready dialog anchors TOP-center to the fullscreen UI,135 UI units down, in both art skins and viewport sizes. Its Dialog stratum must keep the whole dialog and physical accept/decline targets above the still-visible ordinary queue frame (Medium); showing an invitation does not auto-close the queue. Preserve existing dialog size/art/internal layout, match/phase/pending authority and other HUD/scoreboard roots. Source: Retail cache PVPHelper.xml254, PVPHelper.lua340–418, StaticPopup.lua866–924 and PVPUI.lua158–168. Source multi-popup stacking/fullscreen overrides are not claimed implemented by this standalone slice.

## Native talent controls

War Mode occupies the existing ClassTalentsFrame footer's100x100 control (RIGHT BottomBar -6/+2), using authored ring/swords/orb/indent atlases. PvP talent controls use the same Mainline three58x52 slots (tray RIGHT -114) and220x40 flyout rows in both skins. Flyout ordering follows unlocked/required level/selected elsewhere/ID. Selection is immediate, server-owned, and pending mutations cannot be duplicated; absent permissions never enable a mutation. Source PlayerCondition-unavailable choices remain disabled. Existing TalentError carries server refusal. Normal talents/source loadouts stay separate from PvP selections, which are persisted per specialization in existing PvP progress.

Source authorities: Blizzard_PlayerSpells/PvPTalents/Blizzard_PvPTalentSlotTemplates.xml/.lua and Blizzard_PvPTalentListTemplates.xml/.lua, plus ClassTalents/Blizzard_ClassTalentsFrame.xml325–338. No Forever-only rows/slots/layout are added. Tooltip/drag remain unproved; registry tests do not imply those behaviors.

## Bounded acceptance — 2026-10-11

The landed WSG frame now has Quick Match, Rated and Premade Groups categories in both skins. Quick Match shows authoritative Honor wallet quantity/cap; Rated shows Conquest wallet quantity, not season-earned progress. Unknown rating and season-cap fields are `None`, never fabricated1500/1800. Premade listing and rated-match services remain explicitly unavailable.

Native private UDP5790, `fb_pvp2`/Pvpproof: both skins prove real pointer War Mode toggling in Stormwind rest area5148, Enlisted282559 replication, source Holy talent86/640 selection and wallet display. A process relog proves retained desired mode and talent86. Inspected PNGs: `/syncthing/AgentShared/2026-10-11/pvp-progression/`; receipts: `data/diagnostics/pvp-progression-2026-10-11/`. Fresh-character live balances are0; nonzero750/120 display is separately covered by both-skin registry tests, not a claim of earned live rewards.

Honor-level/badge/next-reward display and lifetime leveling remain blocked by the uncertified XP curve; importing reward identities alone does not establish a character's level. Per-match awards, season policy and rating progression remain data gaps. Enlisted XP/quest-XP amount consumers and talent tooltip/drag have no acceptance proof. This is bounded progression infrastructure, not complete Retail parity.
