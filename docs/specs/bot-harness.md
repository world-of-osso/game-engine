# Player-bot test harness

Private real player characters exercise the native controller path and measure class/spec damage without tuning gameplay. Sources and proof boundaries live in [the harness wiki](../wiki/systems/bot-harness.md). Source: `scripts/bot_harness.py`.

## What it must do

- [ ] Maintain one real player per class/spec, including player stats, talents and sourced rotations.
- [ ] An explicitly selected, user-approved pinned SimulationCraft APL provider may supply priorities; record raw URL, commit and SHA-256. It is not an alternate path after a native authored-evaluator error. Unknown potentially applicable predicates block; spells remain constrained by actual learned/native usability facts, with no buffs, kit or budget changes.
- [ ] Read-only native `spell_priority_state(target, spell_ids)` exposes requested learned/readiness/affordability/range facts, owner haste, native spell GCD metadata, observed target casting and the unchanged server-authoritative `TraitConfigSnapshot` received for the owner. Missing trait state is an explicit error; absence must not be inferred from known-spell names. Invalid queries or unavailable native data return explicit errors; querying an unselected unit must neither select/cast at it nor alter its health.
- Native priority snapshots also expose requested charged spells' received/projected current/max counts and remaining/recharge milliseconds; missing charge authority is an explicit error, not zero or ready. Cast-success observations retain server timestamps, and owner CastFailed observations retain spell IDs and monotonic receipt sequences so identical repeated failures remain distinguishable from UI text.
- [ ] Deliver three DPS-spec dummy runs first, then all specs and repeated Stockade runs with followers, headless on private UDP 5871.
- [x] Derive generic direct-target abilities from SpellMisc, SpellPower, SpellCooldowns, SpellEffect and SpellCastTimes; preserve coefficients and exclude passive spells.
- [x] Where installed AssistedCombatStep orders a spec, consume the existing native authored evaluator; unsupported/loading predicates block rather than silently enabling generic behavior. Only absent authored order permits the labeled generic coefficient rule.
- [ ] Capture casts that error or never fire, abilities never used, zero-damage damage spells, ineffectual auras, cooldown/charge/resource violations, deaths and server errors per run.
- [ ] Report unavailable detection evidence explicitly, never treating missing observability as passing.
- [x] Compute DPS over the full specified dummy window, excluding follower damage and other targets; incomplete collections cannot qualify as balance evidence. Require observed positive local-player health at readiness and every collector read; input eligibility alone does not prove alive. Missing health or an already-dead fixture blocks live measurement. Qualify retained reports from their before/in-window health observations too: derived DPS is null and comparison withheld on failed health qualification, while original reports remain unchanged.
- [x] Attribute owned pet/guardian damage through spell-driven Summon events and observed persistent-owner relations; exclude follower roster actors and their pets. Unknown attribution withholds DPS.
- [x] Emit Retail-style Summon only for actual spell-created units, with source=summoner, target=spawned unit and the actual summoning spell. Non-spell/login/journal spawns do not fabricate events or spell IDs.
- [ ] Compare DPS specs only with recorded comparable stats, level, dummy and duration. Report median-relative spread without pass/fail until lead/user chooses tolerance. Never use external DPS numbers.
- [ ] Write JSON and short Markdown per run beneath `data/diagnostics/bot-harness-<date>/`.
- [x] Behavioral detection tests feed actual native combat records and a known missing request; distinguish zero damage from no cast and consume each server success once. Own private Ret32 fixture retains actual submitted20271 request1792056099829 without a success, three real successes and899 damage.
- [x] Merge overlapping combat polls without double counting or collapsing real duplicate events; identify overflow using observed ring retention.

## Native Stockade runs

- Run Fury72 first, adding specs only after current dummy predicates qualify; preserve the same native/IPC player path and equal-budget equipment, without stat/kit overrides.
- A dead assisted target needs no authored recommendation and receives no cast. Retain target-bound native health and resume ordinary assist observation rather than waiting for an evaluation that intentionally omits dead targets.
- Before repeating a persisted in-instance owner, walk the authored area-trigger exit through strict native navigation and confirm its destination map. No admin reposition or persistent-position rewrite.
- Use existing approved Normal Stockade entry through authored WorldSafeLoc3599 and ordinary MapTransfer/WorldPortAck, with source-profile party attachment. Do not mutate the follower205 catalog or describe this diagnostic entry as native LFG205 UI proof.
- Take one capacity ADD before each whole live run, not per route/combat phase. Preserve owned server/client cleanup, actual binary hashes, admitted copy, source route and player equipment receipts.
- Report authoritative three-boss credit separately from combat-metric qualification. Clear time uses monotonic entry-confirmation→final Done observation. Per-boss player/owned-pet damage excludes follower/descendant and trash damage, over first observed boss combat→native boss death, including damage in the final death timestamp bucket.
- Missing death/continuity/ownership or zero-resolution duration yields DPSnull. Missing authoritative Done never becomes a completed boss measurement. Preserve observed player/party deaths from combat events and copy-bound native roster death states without double counting the same actor. A roster-confirmed death missing from the combat stream cannot report zero or certify death totals; retain the native observation and mark death capture incomplete. Before finalizing, keep the owned receiver alive and poll up to10s for the corresponding real combat death; native roster replication may precede reliable combat delivery. Record the delivery deadline/result, never synthesize the event. Total deaths remain unknown after capture gaps. No balance pass/fail tolerance.

## Native race fixture

Northshire Shaman bots use source-valid Draenei11 up front, not a retry-based alternative after Dwarf rejection. Other classes use the lowest source-valid Alliance race. Missing authored source pairs block provisioning. This fixture choice is separate from the server's installed-DB2 race/class policy; the Dwarf/Shaman admin creation/persistence regression now passes.

## BM gameplay pet preparation

- BM253 explicitly prepares its missing pet before dummy collection, not as an alternate rotation after a predicate failure. Existing matching replicated owned pet and received pet bar are reused.
- Select the nearest authored non-exotic tameable Beast spawn on the player's current map, require its installed CreatureFamily row and native learned Tame Beast1515. Verify selected live NPC identity/health/level against ordinary native observations and current server entry lookup.
- The preparation stage owns target selection while the observer keeps publishing state; dummy Tab selection must not interrupt the channel. Native spell usability and strict baked navigation gate the single ordinary tame request. Confirm matching replicated owned-pet and received bar IDs; no grants, persistence edits, substitute creature after failure or repeated uncertain casts.
- Retain source plan, native target, cast request and resulting ownership. Restore ordinary dummy selection before collecting; missing source/native facts or a failed tame blocks collection.

## Native pre-combat forms

Feral103 prepares Cat Form768/form1 and Guardian104 prepares Bear Form5487/form5 from their installed specialization descriptions, SpellEffect aura36 and SpellShapeshiftForm rows. Require the spell to be natively known; use one ordinary validated self-cast, skip toggling an already-observed own form aura, and confirm the replicated own aura before collection. Missing source/confirmation blocks; do not reorder rotation steps or choose another form. Receipts retain source rows and before/after snapshots. Visible sheet stats are labeled as observed after talents but before form preparation; subsequent CharacterStats/combat observations retain actual post-preparation health and effects.

## Comparable equipment budget

Every stat-bearing armor slot and preferred weapon uses authentic Item/ItemSparse at the same raw item-level30, quality2 and squish-era0, within the character's level requirement. Armor follows ChrClasses.ArmorTypeMask (cloaks use their native cloth class); positive primary-stat allocation follows ChrSpecialization.PrimaryStatPriority. Record item IDs, raw ilvl, quality, native owner GUIDs and visible native primary stats per spec in fixture-receipt.json. Comparison keys include observed character level, native dummy level/max-health/difficulty, source world.db SHA-256 and duration; current native dummy creature_armor is retained too, never transient threat/victim. Missing or malformed current admin combat metadata blocks fixture parsing. Roster comparison withholds medians when any expected DPS repetition is absent/incomplete or fewer than three DPS specs qualify. Neck, both rings and both trinkets must be empty for every bot, removed through ordinary paperdoll-to-empty-bag input and confirmed by authoritative equipment/bag snapshots. Storage bags remain equipped and are recorded separately from the combat budget. Provision missing items only after authenticated owned bag/equipment observation; count existing bag copies and confirm granted copies before normal gear input. Retries must not grant duplicate kits after login/setup failures. Missing source slots, mismatched native equipment or unbudgeted stat-bearing equipment block comparison; no changed item/stat definitions or alternate budget.

## How it works

- [Evidence and detection boundaries](../wiki/systems/bot-harness.md)
- [Native automation contract](native-ui-automation.md)

## Maintained routed-controller interface

- `bot_harness.routed_player(...)` constructs one owned controller with the concrete `NavmeshRoute` adapter and correlated input bridge. Caller supplies the agent-run/query-binary argv prefix, baked navmesh directory, output directory, checked native IPC and fresh observations. The adapter preserves raw query/error receipts; empty, malformed or unbaked results block.
- `PlayerController.walk_route(goals, position, observe, timeout)` consumes caller-authored goals, reobserves after each bounded physical step, and services loot before combat/recovery observation ticks. The caller's observation callback retains encounter/party-health/credit policy; `True` yields input to combat/recovery, never substitutes a priority.
- `scripts/bot_controller.py` exposes `PlayerController(invoke, snapshot, route, bridge)`. Callers provide checked native IPC, fresh target-bound facts and an explicit authored/navmesh route query. `step_toward` submits one bounded physical forward step toward the first unreached sourced corner; empty/exhausted routes block, never substitute direct movement.
- `cast_next(target, priority)` accepts an explicitly selected per-spec provider. Provider errors propagate without another rotation; submitted spells must be learned and native-ready, affordable and in range. Ordinary native `Ok(false)` waits without submitting; unavailable/error facts block explicitly. No hardcoded class/spec rotation lives in this controller. Source/trait coverage remains the provider's responsibility. One cast may be outstanding: IPC enqueue, stale ready/GCD facts and elapsed time never release it. Observe a new matching owner CastSuccess or sequenced CastFailed before another submission; a ten-second missing-outcome deadline stops without repeating input.
- `assist(leader_name)` selects the named party leader through existing interaction, then requests native F press/release. An unchanged leader produces no victim; no entity-selection shortcut is introduced.
- `FileBridge` atomically publishes correlated per-client input requests, waits at most ten seconds and never resubmits input on timeout. One controller/writer owns each output directory. Terminal ACK/error/timeout removes its request file, preventing replay by a later client using that directory.
- `vote_loot()` clicks the visible semantic non-Need choice via the native viewport bridge and preserves its actual Greed/Transmog name. Submission ACK is not authoritative vote, win, delivery or pixel proof. Callers must service offers during recovery as well as after encounters.

Launch `godot/tests/bot_controller_live.gd` with an existing `BOT_CONTROLLER_OUTPUT` directory and `BOT_CONTROLLER_SPELL_IDS` JSON array. It observes the real client and exposes the correlated F/mouse bridge; it does not change spec, talents, gear or learned spells. Server admission, encounter credit, natural recovery, route assets and measurement orchestration remain caller-owned. Generic interface tests do not establish an implemented priority for every spec or full-dungeon acceptance.

## Approved pinned Arcane provider

`scripts/bot_simc_arcane.py::ArcaneSimcProvider(receipt, apl_path, captured_kit, db2_dir, initial_snapshot, context)` is an explicitly selected provider, never an alternate path after native authored-evaluator failure. Caller passes it directly to `PlayerController.cast_next`. It verifies SimC commit `515f4a3fe9d1f35e1dbaf253745b6105fba906fb`, the approved raw URL and APL SHA-256 `b1d85887ec62431864cc2db45b95993868bc02a0c0852b5189997542f47cabd4`; all file paths are explicit inputs.

Coverage is only the captured level32/spec62 kit: Counterspell APL24 when observed interruptible casting, four-charge Barrage APL36 at the declared DungeonRoute iteration end, and Blast APL64 filler. Caller explicitly supplies DungeonRoute2700s context, not target TTD or the separate600s combat safety bound. Received server traits and native kit identity are frozen at preflight; changed/unsupported kit, level, talents or context block. DB2 spell names and trait entries resolve source actions; missing data never becomes inferred absence. No OOM Barrage, consumables, external/on-use buffs, other-spec or full-APL claim.

Portable captured [fixtures and provenance](../../scripts/tests/fixtures/bot_harness/simc_arcane32/provenance.json) preserve exact approved source bytes and native DB2 row slices. These are offline decision tests, not native sustained damage or balance proof.

## Implementation inventory

- `godot/rust/src/ipc/combat.rs`: preserve received crit/glancing/periodic, school mask, overflow and absorbed/resisted/blocked values in combat-log/recap output; never infer historical missing fields.
- `scripts/run_stockade_bot.py`: one already-admitted owned native player, equal-budget/role/copy preflight, maintained F-assist/IPC casts/strict native movement, raw events and per-boss reports; caller owns processes and single capacity admission.
- `scripts/bot_stockade.py`: read-only encounter/template/spawn/entrance plan, exact native party facts and explicit authored native priority provider; no Mage cycle or direct-line route.
- `scripts/bot_dungeon.py`: native per-boss ownership/DPS and player/party deaths, separate authoritative completion/clear timing; capture/source gaps remain unqualified.
- `scripts/bot_harness.py`: native combat parsing, request/result reconciliation, fixed-window DPS and comparison reporting.
- `scripts/bot_ability_summary.py` / `bot_combat_metrics.py`: read-only expected-roster per-ability requests/successes/hits/crits/misses/event damage, auto-attack share, sampled GCD/idle intervals, observed raw power net changes and raw DB2 effect/cost/cooldown/auto-attack metadata. Preserve absent specs/repetitions, real child event IDs, and unattributed native errors. Missing crit flags remain unknown, resource net change is never spending, sampled timing is not exact server occupancy.
- `scripts/tests/test_bot_harness.py`: behavioral detector and comparable-fixture regressions.
- `scripts/bot_rotation.py`: sourced metadata, absent-order generic rule and fail-closed native authored selection.
- `scripts/run_dummy_bots.py`: repeated explicit-socket fixed-duration collection and raw receipts; actual submissions use `PlayerController` with the existing explicitly chosen native-authored/absent-order source provider. Atomic observer samples include requested native usability/trait facts; unavailable or wrong-target facts block rather than bypassing the shared boundary.
- `godot/tests/bot_harness_observer.gd`: existing native account/spell/aura/charge/assisted observation and private spec selection.
- `godot/rust/src/spells/assisted_combat.rs`: shared native usability facts and read-only requested-spell snapshot; no target mutation.
- `godot/tests/spell_priority_facts_live.gd`: native selected/unselected query and invalid-input response fixture.
- `scripts/tests/fixtures/bot_harness/`: authentic native Stockade and own private dummy combat with provenance. Stockade's absent request is a labeled injection; Ret's missing request is an actual captured submission.
- `scripts/bot_roster.py` / `scripts/run_bot_roster.py`: source40-spec census and isolated private provisioning/client lifecycle.

## Tests asserting this spec

- `scripts/tests/test_bot_harness.py`
- `scripts/tests/test_bot_rotation.py`
- `scripts/tests/test_bot_controller.py` (generic-spec action boundary, strict route, native-input file correlation; not gameplay acceptance)
- `scripts/tests/test_bot_routes.py` (query-process CSV/failure receipts, busy-loop loot service and physical blocked-route boundary)
- `scripts/tests/test_bot_simc_arcane.py` (pinned source/context/trait/kit decision coverage; not live-game proof)

## Known gaps (current cycle)

- [x] Native-controller-parity three-DPS follow-up: Arms71/Fury72/BM253 each complete two60s common-budget windows through shared native casting with observed auto-attack and received outcome fields. Historical raw-intent Arms/Fury/Havoc captures remain diagnostic only. Havoc's fresh source21566/condition3 refusal remains DPSnull; selecting BM for the new explicit MVP roster never rewrites that failed comparison. No balance or all-spec correctness claim.
- [x] Comparable spec-correct gear/stat receipts for the three-spec MVP: Arms10 slots, Fury/Havoc11 slots at30/quality2/era0, visible native stats and owner GUIDs; Fury native dual-two-hand25129 succeeds. Havoc jewelry moves normally into bag0/1 and retains all four storage-bag IDs/GUIDs/counts. Other specs remain unverified; no gear fallback.
- [ ] Aura-effect, cooldown reset, charge and resource-spending observability; combat-log aura presence alone proves none of these.
- [ ] All-spec comparable damage receipts beyond the completed three-spec MVP.
- [x] Native Feral/Guardian pre-combat Cat768/Bear5487 confirmation through ordinary own-form casts and replicated auras.
- [ ] Feral/Guardian complete follow-up measurements: native attempts stop on source-unverified rules5968/5741 condition3; aura presence is not form-effect correctness.

## Out of scope

Gameplay fixes, stat tuning and balance changes belong to owning agents via the lead. No external damage oracle or fabricated rotations.
