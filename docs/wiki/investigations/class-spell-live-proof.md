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

## Sources

- `/home/osso/.worktrees/handoff-spell-dummy-scripts.md` and `handoff-spell-dummy2.md` through `handoff-spell-dummy8.md` — tested script contracts and exclusions.
- `/home/osso/.worktrees/handoff-spell-live.md` — per-spell live outcome table and publication/cleanup receipts.
- `data/diagnostics/spell-live-2026-10-10/` — native builds, private admin setup, actual IPC logs, GDScript observations and artifact acceptance ledger.
- [Private live-run recipe](../../headless-live-run.md) — isolation and native observation conventions.

## See Also

- [[forever-preset]] — account-scoped skin selection.
- [Native IPC](../../specs/native-ipc.md) — live combat/resource diagnostic boundary.
- [Native UI automation](../../specs/native-ui-automation.md) — production startup and input boundary.
