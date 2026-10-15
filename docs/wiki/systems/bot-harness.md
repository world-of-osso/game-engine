# Player-bot measurement harness

Contract: [bot-harness spec](../../specs/bot-harness.md). Verified evidence below is dated2026-10-15; it is not all-class or balance acceptance.

## Native players and source inputs

`bot_roster.py` reads installed ChrSpecialization ClassID>0/OrderIndex<4:40 playable specs, excluding Initial/Adventurer and retaining Devourer1480. This is a source census, not40 executed bots. For the Northshire fixture it selects source-valid Alliance playable races from ChrRaces/CharBaseInfo: explicitly authored Draenei11 for Shaman, otherwise the lowest valid race. Draenei is selected before provisioning, never retried after another race fails; absence of its source pair blocks. The observed Dwarf3/Shaman7 server-table rejection is retained separately from this fixture choice and its approved source-table fix. Old Orc-shaman captures are historical and are not rewritten.

`run_bot_roster.py` provisions `fb_bot_harness_<spec>` on an already-running private UDP5871. Each client has isolated XDG credentials/state, an explicit owned IPC socket and a preserved private-port token. Capacity ADD and1.5GiB free VRAM gate each launch. One Cage/Godot client is owned/stopped at a time. Extracted-only asset mode and a private cache copy prevent runtime extraction into shared asset links. The driver does not own shared realms or change game stats.

`bot_harness_observer.gd` selects the requested spec only after initial spec/metadata arrival, then publishes existing account/spell/target/aura/cooldown/assisted snapshots. After talent input completes, native Tab selects a dummy. The explicit `target` consumed by native evaluation/casting is verified against admin `npc-info` entry44548; `server_target` is retained as an echo, not substituted for that cast target. The original incorrect field choice and early spec request are harness setup bugs, not gameplay fixes.

Ordinary bounded movement approaches the observed dummy model origin to2.5 horizontal yards before measurement. The existing ground-tested private destination is only initial placement. Missing position, changed target or a100-step approach failure blocks; there is no teleport/collision bypass during approach.

## Sourced rotations and talents

All40 installed specs have AssistedCombatStep order. When order exists, the collector consumes the existing [native assisted evaluator](../investigations/cooldown-assist-data.md); unknown/loading predicates stop collection. Only an absent authored order permits the approved, labeled highest-available DB2 coefficient rule. No source gap silently enables that generic rule.

`bot_rotation.py` preserves SpellMisc/Power/Cooldowns/Effect/CastTimes rows and excludes passives. Explicit missile/spell triggers32/64 retain their damage identities: Frostbolt116 deals through228597. Cyclic direct triggers fail explicitly. Periodic, weapon-only and script-only damage classification remains incomplete.

`bot_talents.py` emits TraitTreeLoadoutEntry OrderIndex with node/entry/spell and the existing native button name. NumPoints is a cumulative target rank, not an additional purchase count. Pinned TC a352b1fa TraitMgr::InitializeStarterBuild subtracts existing Rank; its48138-byte cached source matches pinned tree blob`10ff78b0d921b36f7d7cb91e632eea7d69c9cca8`. TraitMgr::Load's last-encountered loadout policy selects Fire931. An explicit sourceID override is recorded; ambiguous unselected multi-entry nodes fail rather than invent a choice.

`bot_harness_talents.gd` drives actual N/mouse input. Already-present ranks are not repurchased, unavailable/disabled nodes are recorded, and the ordinary Apply button commits staged choices. Native UI observations are not automatically persistence proof. Ret32's12 positive source-plan ranks were separately retained across a stopped client and fresh credential login (`talent-persistence.json`). The controller also opens the existing CharacterFrame and reads visible source stats-pane labels/values using the maintained `sheet_stats_live.gd` path. Each bot retains admin equipment ownership output; neither receipt alone asserts cross-class stat comparability.

## Capture and detection

`run_dummy_bots.py` requires an explicit socket/state/DB2 directory/dummy. It saves every IPC reply, native samples, run manifest and raw combat. Overlapping ring polls preserve true event multiplicity without recounting overlap; sequence gaps use the observed200-event retention, not the requested1024 lines.

Server timestamps delimit a fixed half-open window. Only the bot and its owned combat units contribute damage to the chosen dummy. Spell-created short-lived units use Summon records; persistent pets use `pet_bar_state.local_pet`, whose native lookup requires replicated UnitIsPet and UnitSummonedBy for the local player. The observer and collector retain the earliest sampled relation for each pet; restored-login pets do not need synthetic Summon events. Follower roster actors and their descendants are excluded. Ownership timestamps are respected, conflicts fail explicitly, and unknown positive damage attribution withholds DPS rather than silently dropping a possible guardian contribution. A cast success can satisfy one request only; deadline-ordered interval matching avoids false failures from nested windows. When overlapping requests make attribution ambiguous, a missing-cast finding has a null request timestamp instead of falsely identifying one submission. Deadlines not yet observed after interruption remain pending. Findings distinguish unused active abilities, a successfully cast damage spell with zero observed damage, missing casts, native rejections, participant deaths and server errors. Collateral NPC deaths are raw evidence, not bot/follower failures.

Rejection strings currently lack spell IDs, so `native_cast_error` uses null attribution. Charge/resource/aura-effect/reset violations are not certified from combat text or aura presence alone. Existing native assisted/cooldown/aura snapshots are retained for further detection; these checks remain explicitly unobserved.

Incomplete collection exposes DPS as null. Setup failures also receive JSON/Markdown. Summaries remain short; full per-ability findings live in JSON.

## Spell-driven ownership events

`CombatLogKind::Summon` is appended to the protocol enum. Effect28 wild/guardian creation and effect56 controlled-pet creation log only after successful creation, retaining the actual child/root spell ID in that context. Recalling an existing pet and non-spell login restoration do not create summon events. Native IPC exposes `summon src=... dst=... spell=...` with the server timestamp. Log delivery follows temporary summon ancestry to the owner, independently of gameplay credit policies.

Native regression proof: corrected root51533 and child228562 each create two wolves29264 whose owner events remain after expiry (two tests GREEN). Gargoyle49206/27829 damage reaches the owner without a guardian client (one test GREEN); its AP100 is explicit fixture state because non-demon inheritance remains unsupported, not live stat/balance proof. Imp creation/recall and reopened-login controls pass three tests, including no fabricated login event. Protocol wire round-trip passes one test; native IPC text passes six focused tests; forty Python harness tests and all three observer/controller check-only parses pass. Engine/CLI, server and protocol scoped checks pass at their merged revisions. Logs are `summon-*-green.log`, `*-merged-green.log`, `summon-integrated-tests.log` and `*-summon-parse.log` beneath the durable evidence root.

Pre-merge root51533 produced four wolves because script and generic summon count both applied. Upstream master now casts child228562 once; this harness branch made no gameplay correction. Those server/client CPU fixtures do not establish an all-spec rendered live run or balance acceptance.

## Damage comparison

The baseline leaves Neck, Finger1/2 and Trinket1/2 empty. The gear controller finds a visible native bag button absent from the occupied owner snapshot, clicks the equipped jewelry then that empty destination, and waits for the equipment slot to disappear and the item ID to appear at that destination. Missing space or confirmation blocks setup; no admin unequip path. Bag1–4 remain equipped and receipts retain their owner identities separately from budgeted combat items. Eight focused gear tests pass, including all five jewelry refusals with four retained native storage bags; integrated harness Python50 and Godot gear parse pass. Native removal proof remains pending.

The native driver now writes comparison.json only after owner equipment receipt validation and an observed CharacterStats level matching the fixture. The shared comparison key contains the equipment budget, actual dummy level/max-health/difficulty, source world.db SHA-256 and full duration; each spec's exact item IDs/GUIDs and visible stats remain in fixture-receipt.json. Current dummy damage does not change its max-health fixture identity.

The roster writes balance.json plus comparison.md, computing medians only when all expected DPS repetitions are complete; missing/incomplete specs are not silently dropped. Fewer than three DPS specs or mismatched receipts explicitly withhold the MVP comparison.

Comparison requires equal explicit receipts for level, stats, dummy and duration. ChrSpecialization.Role2 alone enters the DPS median. Equal-duration repetitions first produce one mean DPS per spec; unequal repeat counts and tank/healer damage cannot weight that median. Reports give max/median and min/median with no tolerance or pass/fail. No external DPS numbers are used. Comparable gear/stat receipts remain open; ordinary source-derived starting characters alone do not establish them.

## Recorded native proof and remaining gates

- Ret32, sourced talents and physical melee approach: first complete60s run deals2406 own-dummy damage,40.1DPS, with15 unmatched submissions and15 native rejections. Two later60s windows record33.9333 and20DPS. These are actual measurements, not bug-free or balanced-class claims.
- Ret talent fresh-login evidence retains12/12 previously positive source-plan ranks. Stopped/started client PIDs, account identity, source manifests and realized ranks are preserved.
- Earlier default-kit runs stop on Arcane12408/condition20, Ret6425/equality, Elemental38530/condition17; Frost reports33686/condition67. Sourced talents/melee approach later permit the Ret windows; the other evaluator semantics remain owner gaps.
- Detection regressions retain an actual submitted20271 request1792056099829 without success, alongside three successes/899 damage. The separate Stockade fixture has an explicitly injected absent request; neither fixture fabricates combat values.
- Three-spec collection has six completed windows (Ret70,Arms71,Fury72), but inherited1H+shield gear makes them ineligible for a spec-correct balance claim. Comparable weapon fixtures, all-spec execution, full detection observability and repeated Stockade/follower completion remain open.

Durable evidence: `/home/osso/.worktrees/bot-harness-2026-10-15/`, linked from slot `data/diagnostics/bot-harness-2026-10-15/`. Initial extension/CLI helper build passed698.7s; server helper passed9m36s. Those binaries predate later master UI/terrain merges; live receipts do not certify unrelated merged Rust changes. Source revisions and targeted RED/GREEN command logs live in the handoff/proof ledger.

## Sources

- `godot/rust/src/ipc/{combat,world}.rs`: server timestamps/text and ordinary validated SpellCastIntent.
- `data/db2/active/`: installed spell, specialization, race and talent rows; selected authentic fixtures retain provenance.
- `godot/rust/src/spells/{snapshot,assisted_combat,cooldown_viewer}.rs`: existing native observability and source evaluator.
- Pinned TC a352b1fa `TraitMgr.cpp`: starter rank and loadout semantics, matching blob above.
- [Follower native controller proof](follower-dungeons.md), [class proof](../investigations/class-spell-live-proof.md), [private terrain destination](../../../../game-server/docs/wiki/investigations/private-terrain-destinations.md).

## Spec-correct source gear fixture

`bot_gear.py` derives weapon kinds from source starter Item references and each installed ChrSpecialization preferred-weapon description; those references are not the newly equipped items. Exact private-world Item/ItemSparse rows select the lowest eligible ID at the [contract's common budget](../../specs/bot-harness.md#comparable-equipment-budget), with positive appropriate primary-stat allocation, valid class/level, no race restriction and a single raw squish era. ChrClasses.ArmorTypeMask selects the native armor subclass; SpecTabInfo/TC Player::GetPrimaryStat defines the specialization priority mapping. All nine body/cloak slots and preferred weapons share this budget; missing source coverage blocks rather than selecting a different tier. All40 source plans are checked against61 authentic Item/ItemSparse row pairs, not claimed as40 live equips. Fury retains dual-two-hand and requires learned Titan's Grip; master now implements its hand rules, while the full-budget bot's acquisition/equip still requires native proof. No one-hand substitute.

The ordinary bag-to-paperdoll controller now handles all planned slots. Before comparison, `equipment_receipt` requires the native owner snapshot to match every planned item ID, rejects extra stat-bearing slots, and retains item GUIDs, ilvl/quality and observed native primary stats. Cosmetic Shirt/Tabard do not enter the stat budget. Native full-budget equip remains the gate; the following weapon-only captures predate this standardization.

Native `native-source-gear1` proves Ret70 and Arms71 equip2361 via ordinary bag-to-MainHand input and authoritative equipment snapshots; the prior shield moves out normally. Arms completes two60s windows:770/776 damage,12.8333/12.9333DPS. Ret's source2H/source-talented run still stops on6425 equality after1788 damage (DPSnull), so talents never established a fix for that source gap. Enhancement263 provisioning stops before a client: installed CharBaseInfo permits Dwarf3/Shaman7, but the server's hardcoded race_class table rejects it. Preserve that source/runtime gap, not a silent race substitution. All owned clients/server were stopped after these captures; comparable three-spec and all-spec/Stockade gates remain open.

Gear preparation grants only missing source items into owned bags via the ordinary admin catalog grant, then performs native bag-to-paperdoll cursor swaps. Pickup is local CursorItem state, not a bag-removal ACK; the server validates the final SwapItem and its equipment snapshot must match each requested slot. Explicit MainHand/OffHand avoids auto-equip choosing an unintended empty/low-level slot. No trusted admin equipment override or old-weapon fallback is used. Required private world.db and input hashes identify the actual item source.
