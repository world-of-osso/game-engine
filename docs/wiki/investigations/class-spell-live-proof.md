# Class spell live proof

Verified: 2026-10-10. Thirteen classes received bounded rendered-client proof against private realm UDP54113, not merely server unit tests. Engine `f64c85fca6439e51a1ea00b286ef6653d45f3ade`, server `986bd0b767e9426a3ee7df0872991aff26275e32`; native extension/CLI and server binaries were built through `agent-run`. No production code changed.

## Proven boundary

One representative scripted path per class: Bloodthirst, Holy Shock, Kill Shot, Fan of Knives, Penance, Howling Blast, Chain Lightning, Arcane Barrage, Demonbolt, Renewing Mist, Rake, Reap and Pyre. Evidence combines actual client-submitted casts, received combat logs, replicated resources, native aura/UI snapshots and inspected captures. Warrior, Paladin, Priest and Monk have both-skin proof. Additional Shield Block and Light of Dawn paths have bounded aura/heal/resource evidence.

The per-spell outcomes, exact amounts, artifact filenames and excluded contracts live in `/home/osso/.worktrees/handoff-spell-live.md`; durable receipts are under `data/diagnostics/spell-live-2026-10-10/`. PNGs are under `/syncthing/AgentShared/2026-10-10/spell-live/`. These are local evidence paths, not shipped assets.

This does **not** certify complete talents, damage formulas across stats/levels, optional procs, every area exclusion, guaranteed critical flags, or all cooldowns. Supplemental Provoke threat/statue behavior, Demon Spikes armor and Cobra Shot cooldown reduction remain without complete live proof. No server-script bug was established; setup refusals were not turned into production fixes.

## Reproduction lessons

- Explicit Modern selection must include `forever_default_migrated: true`; otherwise the old-layout migration can change the requested skin. Verify `account_state().ui_skin` or native frame geometry, not filename labels.
- Prepare private characters' map/position while offline. A new Draenei's default map blocked entry until its private saved position was corrected.
- Fund Light of Dawn with real Holy Shock/Crusader Strike casts before testing its injured-target heal. Three slowly spaced Holy Shocks alone lost Holy Power to out-of-combat decay.
- Chain Lightning's direct power mutation produces no Energize log entry. Ten three-hit casts admitted the real 60-Maelstrom Earth Shock; zero casts and nine casts refused with `Maelstrom`. This proves the resource path behaviorally without inventing a resource value in the client diagnostic.
- Select Devourer1480 before Reap. The earlier Havoc sample hit for zero; the authoritative specialization change yielded five positive child hits. Do not call a wrong-spec coefficient sample a script regression.

## Batches9–14 continuation — spell-live2

Verified: 2026-10-10. Private UDP54123 used the prebuilt server615fb90 and a native extension/CLI built from the `spell-live2` base recorded in `data/diagnostics/spell-live2-2026-10-10/revision.txt`. Twelve classes have new bounded script-path evidence; Monk has a Renewing Mist control only, because the supplied batches introduce no source-bound Monk path. This is not thirteen new-script passes.

The continuation covers Sacrifice's real damage split, an owned Hunter wolf's Kill Command, Rupture's paid-resource duration, Rake's persistent stealth snapshot, Surge's optional mana payment, Bolt-funded Earth Shock, and named damage/heal/aura children. Warrior, Rogue, Priest and Mage were observed in both authenticated skins. Generic Imp/Gargoyle cast presentation and two temporary Spirit Wolf actors were sampled separately; guardian damage amounts, guaranteed critical flags and non-demon inheritance remain uncertified.

Exact per-spell outcomes and exclusions live in `/home/osso/.worktrees/handoff-spell-live2.md`. Durable receipts, observer/action scripts, artifact assertions and the inspection/hash manifest live in `data/diagnostics/spell-live2-2026-10-10/`; forty inspected PNGs live in `/syncthing/AgentShared/2026-10-10/spell-live2/`. No production fix or confirmed server-script defect was established.

Reproduction corrections:

- Relearn selected fixture spells **after** changing specialization; the native known-spell synchronization can remove prior admin grants.
- `game-engine-cli spell cast` submits the network intent directly. Native `use_spell` establishes autoattack for melee paths; without that, training-dummy evade can clear otherwise long-lived bleeds after five seconds. Do not mistake evade for authored expiry.
- `game-server-admin damage` without an explicit entity damages the selected target. Use the observer's actual player entity when preparing injured-self healing; the first healthy Blossom casts correctly produced no injured-target heal.
- Sacrifice against Wilhelm Strang required a same-room teleport and waiting for the received aura before applying damage. Outside-LOS and premature samples are setup failures, not transfer defects.

## Class completeness continuation — 2026-10-12

Current server census174a2e84/shared2a48bdc:2889 working-boundary/4756 partial/182 broken across7827 memberships. Eleven additional classes now have private Modern/Forever spell receipts on5841, alongside retained Monk/Druid proofs: all13 representative live gates close, while the full13/all40 goal stays open. Exact per-class receipts, data gaps and revisions live in `/home/osso/.worktrees/handoff-class-completeness2.md` and the [server continuation](../../../../game-server/docs/class-completeness2.md). Current requested PNGs: `/syncthing/AgentShared/2026-10-12/class-completeness/`. Slot-local logs are `game-server-s6/data/diagnostics/class-completeness4-2026-10-12/live/`; no shared realm or canonical world.db/cache was changed.

Demon Hunter startup reproduced a metadata-loss bug, not a missing legacy replacement: pinned12.1.0.69933 item133320 binds appearance29124 with display0. The item-appearance importer discarded0, then the renderer attempted to resolve nonexistent display0 after that producer was fixed. a1c9a71f6 native RED → ee373d6a2 retained row → actual display0 RED →22137903a occupied slot/no-drawn-equipment. Six targeted core controls and one shared renderer integration control pass, including visible item38 and missing/invalid metadata rejection. This does not certify all invisible-gear mechanisms or transmog rules. Rebuilt9563568be DH proof renders the actual actor, completes162243 and produces250/253 damage with25 Fury in both skins. Evoker completes431443→361500 Fire1139/1141+431583 Arcane285/285 with431442 granted. Setup range/death/receipt failures are excluded. All26 requested PNGs are present; own private servers/clients stopped and slot cache/products links restored.

## Native process continuation — 2026-10-13

Enginee48b2904b/server34381431/shared29c40b8 matched private5841 only. Native specialization UI selected Marksmanship254 and showed Active; client requests produced Trueshot CastSuccess/aura/expiry, Penetrating Shots application, Aimed Shot1270 damage, and seven Rapid Fire137–141 damage/3-Focus pairs. Exact scope and data/build receipts are in the [server ledger](../../../../game-server/docs/class-completeness6.md#native-process-continuation--2026-10-13).

No new qualifying PNG: Ironforge capture is geometry-occluded, actual skin identity not independently confirmed, and broader NPC asset receipts remain partial. This proves private process delivery/consumption, not quantified crit/recovery, full talents or both-skin rendered parity. Own client/cage/server stopped; endpoint token and slot cache/products links restored. Canonical DB/shared realm unchanged.

## Sources

- `/home/osso/.worktrees/handoff-spell-dummy-scripts.md` and `handoff-spell-dummy2.md` through `handoff-spell-dummy8.md` — tested script contracts and exclusions.
- `/home/osso/.worktrees/handoff-spell-live.md` — per-spell live outcome table and publication/cleanup receipts.
- `data/diagnostics/spell-live-2026-10-10/` — native builds, private admin setup, actual IPC logs, GDScript observations and artifact acceptance ledger.
- `/home/osso/.worktrees/handoff-spell-dummy9.md` through `handoff-spell-dummy14.md`, `handoff-spell-mechanics.md`, `handoff-spell-guardians.md` — continuation contracts and explicit unimplemented boundaries.
- `/home/osso/.worktrees/handoff-spell-live2.md` and `data/diagnostics/spell-live2-2026-10-10/` — continuation live receipts, inspected PNG inventory and artifact assertions.
- [Private live-run recipe](../../headless-live-run.md) — isolation and native observation conventions.

## See Also

- [[forever-preset]] — account-scoped skin selection.
- [Native IPC](../../specs/native-ipc.md) — live combat/resource diagnostic boundary.
- [Native UI automation](../../specs/native-ui-automation.md) — production startup and input boundary.
