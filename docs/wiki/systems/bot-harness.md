# Player-bot measurement harness

Contract: [bot-harness spec](../../specs/bot-harness.md).

`python3 scripts/bot_harness.py RUN.json COMBAT.log OUTPUT` reads submitted cast attempts and native IPC combat text. Server timestamps delimit a fixed half-open measurement window. Actor/target identity excludes companions and unrelated damage. Cast successes are consumed once when reconciling requests; requests include their explicit response deadline. Expected damage spells may list child `damage_ids`.

The first regression fixture retains unmodified combat lines from the private native Mage32 Stockade controller at `/home/osso/.worktrees/follower-dungeons-2026-10-13/stockade-diagnostic/physical-los-run/command-00236.log`. The test adds a deliberately absent Arcane Barrage request after that capture; this is a detector test, not evidence that a live Arcane Barrage request failed. A second controlled test removes actual damage records while keeping actual successes.

Reports separate findings from unobservable checks. The present IPC combat text does not expose charges, authoritative resource spending, aura effect state or cooldown resets. An applied aura alone cannot certify effect/no-effect. No clean gameplay claim is made for these checks.

Comparison groups require an explicit equal `comparison` receipt (level, stats, dummy, duration). Median-relative spread has no pass/fail threshold and is not an external DPS comparison. A raw report defaults to incomplete until native fixed-duration collection provides its receipt.

## Proof ledger — 2026-10-15

- Detector RED: seven focused unittest cases fail against empty implementation; native combat fixture and missing-request assertion reproduce absent detection.
- Native execution: pending; capacity HOLD has prevented builds/client launch.

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
