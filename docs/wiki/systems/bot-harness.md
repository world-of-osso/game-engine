# Player-bot measurement harness

Contract: [bot-harness spec](../../specs/bot-harness.md). Verified evidence below is dated2026-10-15; it is not all-class or balance acceptance.

## Native players and source inputs

`bot_roster.py` reads installed ChrSpecialization ClassID>0/OrderIndex<4:40 playable specs, excluding Initial/Adventurer and retaining Devourer1480. This is a source census, not40 executed bots. For the Northshire fixture it selects the lowest valid Alliance playable race from ChrRaces/CharBaseInfo; old Orc-shaman captures are historical and are not rewritten.

`run_bot_roster.py` provisions `fb_bot_harness_<spec>` on an already-running private UDP5871. Each client has isolated XDG credentials/state, an explicit owned IPC socket and a preserved private-port token. Capacity ADD and1.5GiB free VRAM gate each launch. One Cage/Godot client is owned/stopped at a time. Extracted-only asset mode and a private cache copy prevent runtime extraction into shared asset links. The driver does not own shared realms or change game stats.

`bot_harness_observer.gd` selects the requested spec only after initial spec/metadata arrival, then publishes existing account/spell/target/aura/cooldown/assisted snapshots. After talent input completes, native Tab selects a dummy. The explicit `target` consumed by native evaluation/casting is verified against admin `npc-info` entry44548; `server_target` is retained as an echo, not substituted for that cast target. The original incorrect field choice and early spec request are harness setup bugs, not gameplay fixes.

Ordinary bounded movement approaches the observed dummy model origin to2.5 horizontal yards before measurement. The existing ground-tested private destination is only initial placement. Missing position, changed target or a100-step approach failure blocks; there is no teleport/collision bypass during approach.

## Sourced rotations and talents

All40 installed specs have AssistedCombatStep order. When order exists, the collector consumes the existing [native assisted evaluator](../investigations/cooldown-assist-data.md); unknown/loading predicates stop collection. Only an absent authored order permits the approved, labeled highest-available DB2 coefficient rule. No source gap silently enables that generic rule.

`bot_rotation.py` preserves SpellMisc/Power/Cooldowns/Effect/CastTimes rows and excludes passives. Explicit missile/spell triggers32/64 retain their damage identities: Frostbolt116 deals through228597. Cyclic direct triggers fail explicitly. Periodic, weapon-only and script-only damage classification remains incomplete.

`bot_talents.py` emits TraitTreeLoadoutEntry OrderIndex with node/entry/spell and the existing native button name. NumPoints is a cumulative target rank, not an additional purchase count. Pinned TC a352b1fa TraitMgr::InitializeStarterBuild subtracts existing Rank; its48138-byte cached source matches pinned tree blob`10ff78b0d921b36f7d7cb91e632eea7d69c9cca8`. TraitMgr::Load's last-encountered loadout policy selects Fire931. An explicit sourceID override is recorded; ambiguous unselected multi-entry nodes fail rather than invent a choice.

`bot_harness_talents.gd` drives actual N/mouse input. Already-present ranks are not repurchased, unavailable/disabled nodes are recorded, and the ordinary Apply button commits staged choices. Native UI observations are not automatically persistence proof. Ret32's12 positive source-plan ranks were separately retained across a stopped client and fresh credential login (`talent-persistence.json`).

## Capture and detection

`run_dummy_bots.py` requires an explicit socket/state/DB2 directory/dummy. It saves every IPC reply, native samples, run manifest and raw combat. Overlapping ring polls preserve true event multiplicity without recounting overlap; sequence gaps use the observed200-event retention, not the requested1024 lines.

Server timestamps delimit a fixed half-open window. Only the bot's damage to the chosen dummy contributes to DPS. A cast success can satisfy one request only. Deadlines not yet observed after interruption remain pending. Findings distinguish unused active abilities, a successfully cast damage spell with zero observed damage, missing casts, native rejections, participant deaths and server errors. Collateral NPC deaths are raw evidence, not bot/follower failures.

Rejection strings currently lack spell IDs, so `native_cast_error` uses null attribution. Charge/resource/aura-effect/reset violations are not certified from combat text or aura presence alone. Existing native assisted/cooldown/aura snapshots are retained for further detection; these checks remain explicitly unobserved.

Incomplete collection exposes DPS as null. Setup failures also receive JSON/Markdown. Summaries remain short; full per-ability findings live in JSON.

## Damage comparison

Comparison requires equal explicit receipts for level, stats, dummy and duration. ChrSpecialization.Role2 alone enters the DPS median. Equal-duration repetitions first produce one mean DPS per spec; unequal repeat counts and tank/healer damage cannot weight that median. Reports give max/median and min/median with no tolerance or pass/fail. No external DPS numbers are used. Comparable gear/stat receipts remain open; ordinary source-derived starting characters alone do not establish them.

## Recorded native proof and remaining gates

- Ret32, sourced talents and physical melee approach: first complete60s run deals2406 own-dummy damage,40.1DPS, with15 unmatched submissions and15 native rejections. Two later60s windows record33.9333 and20DPS. These are actual measurements, not bug-free or balanced-class claims.
- Ret talent fresh-login evidence retains12/12 previously positive source-plan ranks. Stopped/started client PIDs, account identity, source manifests and realized ranks are preserved.
- Earlier default-kit runs stop on Arcane12408/condition20, Ret6425/equality, Elemental38530/condition17; Frost reports33686/condition67. Sourced talents/melee approach later permit the Ret windows; the other evaluator semantics remain owner gaps.
- Detection regressions retain an actual submitted20271 request1792056099829 without success, alongside three successes/899 damage. The separate Stockade fixture has an explicitly injected absent request; neither fixture fabricates combat values.
- Three-DPS MVP, all-spec execution, full detection observability, comparable-stat spread and repeated Stockade/follower completion remain open.

Durable evidence: `/home/osso/.worktrees/bot-harness-2026-10-15/`, linked from slot `data/diagnostics/bot-harness-2026-10-15/`. Initial extension/CLI helper build passed698.7s; server helper passed9m36s. Those binaries predate later master UI/terrain merges; live receipts do not certify unrelated merged Rust changes. Source revisions and targeted RED/GREEN command logs live in the handoff/proof ledger.

## Sources

- `godot/rust/src/ipc/{combat,world}.rs`: server timestamps/text and ordinary validated SpellCastIntent.
- `data/db2/active/`: installed spell, specialization, race and talent rows; selected authentic fixtures retain provenance.
- `godot/rust/src/spells/{snapshot,assisted_combat,cooldown_viewer}.rs`: existing native observability and source evaluator.
- Pinned TC a352b1fa `TraitMgr.cpp`: starter rank and loadout semantics, matching blob above.
- [Follower native controller proof](follower-dungeons.md), [class proof](../investigations/class-spell-live-proof.md), [private terrain destination](../../../../game-server/docs/wiki/investigations/private-terrain-destinations.md).
