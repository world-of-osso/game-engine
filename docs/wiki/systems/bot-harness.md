# Player-bot measurement harness

Contract: [bot-harness spec](../../specs/bot-harness.md).

`python3 scripts/bot_harness.py RUN.json COMBAT.log OUTPUT` reads submitted cast attempts and native IPC combat text. Server timestamps delimit a fixed half-open measurement window. Actor/target identity excludes companions and unrelated damage. Cast successes are consumed once when reconciling requests; requests include their explicit response deadline. Expected damage spells may list child `damage_ids`.

The first regression fixture retains unmodified combat lines from the private native Mage32 Stockade controller at `/home/osso/.worktrees/follower-dungeons-2026-10-13/stockade-diagnostic/physical-los-run/command-00236.log`. The test adds a deliberately absent Arcane Barrage request after that capture; this is a detector test, not evidence that a live Arcane Barrage request failed. A second controlled test removes actual damage records while keeping actual successes.

Reports separate findings from unobservable checks. Combat text alone does not expose charges, authoritative resource spending, aura effect state or cooldown resets. The observer now also captures existing native assisted/cooldown/aura snapshots; their live detection coverage remains unverified. An applied aura alone cannot certify effect/no-effect. No clean gameplay claim is made for these checks.

Comparison groups require an explicit equal `comparison` receipt (level, stats, dummy, duration). Median-relative spread has no pass/fail threshold and is not an external DPS comparison. A raw report defaults to incomplete until native fixed-duration collection provides its receipt.

## Proof ledger — 2026-10-15

- Detector RED: seven focused unittest cases fail against empty implementation; native combat fixture and missing-request assertion reproduce absent detection.
- Native execution: pending; extension build succeeded, CLI build still running; no client launched yet.

Related: [native follower controller proof](follower-dungeons.md), [class proof](../investigations/class-spell-live-proof.md).

## Native collector

`godot/tests/bot_harness_observer.gd` publishes existing account/spell/target/aura/cooldown/assisted snapshots to an owned output directory and selects its requested specialization through the existing private method. `scripts/run_dummy_bots.py` requires an explicit socket, observed state, DB2 directory and dummy entity. It collects overlapping1024-event ring snapshots, preserving duplicate multiplicity, and saves every IPC reply plus native samples. Repeats remain separate runs; no automatic stat equality is assumed. Static DB2 cooldown scheduling is a generic driver, not proof of authoritative cooldown correctness.

## Sources

- `godot/rust/src/ipc/combat.rs`: native text serialization and server timestamps.
- `godot/rust/src/ipc/world.rs`: ordinary SpellCastIntent path, server validation retained.
- `data/db2/active/{SpellEffect,SpellMisc,SpellPower,SpellCooldowns,SpellCastTimes}.csv`: retained source rows and generic direct-damage coefficients; passive flag follows pinned TC SPELL_ATTR0_PASSIVE.
- [Follower native proof](follower-dungeons.md): owned private real-player controller precedent.

## Authored order discovery

[Existing assisted data](../investigations/cooldown-assist-data.md) already supplies AssistedCombatStep.OrderIndex and the native evaluator. The collector uses that evaluator when the installed table orders the specialization; failure to load/evaluate blocks that run, never enabling generic behavior. The approved generic coefficient rule applies only to a spec absent from the authored step table. This supersedes the initial direct-only driver plan. A retained actual private Ret snapshot proves exact native selection35395, not an invented top ability.

## Source roster and client lifetime

`bot_roster.py` enumerates installed ChrSpecialization ClassID>0/OrderIndex<4 and chooses each class's lowest valid CharBaseInfo RaceID. Installed70391 census is40 playable specs, excluding Initial/Adventurer slots and retaining Devourer1480. This is a source census, not40 executed bots. `run_bot_roster.py` provisions fb_bot_harness_<spec> on an already-running private5871, sets the common requested level, uses the existing ground-tested cooldown-assist destination and selects an actual44548 dummy through native Tab. Each bot has its own XDG credentials/state; private port tokens are preserved between clients, never reused across accounts. Capacity ADD and1.5GiB free VRAM gate each launch. Extracted-only mode prevents runtime CASC extraction into shared asset links; cache is a private copy. It owns/stops one client at a time and never starts/stops the shared realm.

## Native setup boundary — first attempt

Own5871 native Mage32 selected a dummy in prediction while the authoritative target still named another entity. The identity guard rejected this sample instead of reporting DPS on the wrong target. Retained actual snapshot reproduces readiness RED; initial mitigation waited for target/server_target equality and spell metadata; the next root-cause check supersedes that target gate below. Fire remained on default62 after its early one-shot spec request; observer now waits for initial spec plus metadata before sending, pending live confirmation. These are harness readiness changes, not gameplay/stat fixes.

The collector had incorrectly chosen server_target for its explicit IPC cast target. Native assisted facts and native selection use target; the explicit spell request independently names that verified entity. The driver now verifies target against admin npc-info44548 and uses that same ID for evaluation/casting. A differing echoed target is retained as diagnostic evidence, not a requirement that can redirect a cast to an unrelated entity. Actual snapshot regression exercises this exact mismatch.

Aborted observations retain requests whose response deadline has not elapsed as pending, rather than falsely classifying them never-fired. Setup failures also get JSON and Markdown without claiming a measurement. For authored rotations, all known active metadata participates in never-used coverage; the selected source order remains separate from coverage.

Native cast-rejection strings are reported as native_cast_error with a null spell ID because the current public snapshot does not identify their originating spell. The collector retains only newly appended errors; it never invents an attribution. Private Ret32 produced No charges remain during the live60s run; source/gameplay cause remains owner investigation, not a harness balance fix.
