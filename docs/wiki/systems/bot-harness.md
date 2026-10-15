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
