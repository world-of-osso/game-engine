# Engine crate split plan

Verified: 2026-10-23. Source: `4645578c7cf2eeb9afb712258c04faf57df53f7b`. **Extract leaf UI feature areas first; keep the existing `game-engine-ui-model` as their common dependency.** Housing and Adventure Guide can leave without first untangling every shared screen helper. Native rendering needs smaller, explicitly audited cuts: its directory names do not describe independent crates.

## Scope and gates

Step 1 delivers this plan, a reproducible dependency/churn graph and baseline edit measurements. No runtime changes, source relocation, profile tuning, asset writes or full-suite runs. Execution checkout: `/home/osso/.worktrees/game-engine-crate-split`, branch `engine-crate-split`. Each subsequent completed step is a pushed tip for the lead to gate; do not start the next extraction before approval. Push only `engine-crate-split:engine-crate-split`, never master.

Every build uses `scripts/agent/agent-run lead_esplit_15135 env TMPDIR=/var/tmp/osso ...`, the depot helper, explicit `-j4`, and `CAPACITY_ID=lead_esplit_15135` admission. ADD starts one job; HOLD waits 60 seconds; SHED stops heavy work. No concurrent builds. Keep the same target directory, dependencies, profiles and feature selection for paired measurements. Lead owns the full-suite/integration gate.

## Graph evidence

[Generated adjacency, SCC membership and churn](engine-crate-split-graph.md) comes from `scripts/engine-crate-graph.py`, adapted from the server tool at game-server `0d8542f46`. The 14-day churn interval ends exactly at the engine source commit's committer time, not the measurement wall clock:

```text
python3 scripts/engine-crate-graph.py --root <checkout-at-4645578c7> \
  --since 2026-10-09T21:15:10Z --until 2026-10-23T21:15:10Z \
  --output target/crate-split/graph.json \
  --markdown docs/wiki/investigations/engine-crate-split-graph.md
python3 -m unittest discover -s scripts -p test_engine_crate_graph.py
```

The appendix was generated at documentation tip `4640c228d`; `git diff 4645578c7 -- godot` was empty before and after generation. Rust inputs are the pinned source above. The scanner follows `mod` and `#[path]`, masks literals/comments, handles grouped `crate::{...}` paths, separates direct `cfg(test)` subtrees, and retains file/line witnesses in JSON. Root `module::` calls are included. `@main` is the inherited server-tool name for **lib.rs**, not a new executable; explicit `crate::GameClient`/extension/budget references return to that root. Three behavioral fixtures cover masking, path ownership, test exclusion, SCCs, distinct-commit churn, inline facades and root-client back edges. No unresolved module files.

This is a lexical **potential** graph, not rustc name resolution. Non-test cfg branches are unioned; aliases, `super::`/`self::`, glob imports, macro expansion and arbitrary root items are not fully resolved. Inline UI-facade LOC belongs to lib.rs in the census. Integration targets are intentionally not library-module nodes. Inspect actual imports and run the changed-crate check before declaring an extraction acyclic.

| Corpus | Disk Rust files / physical LOC | Reachable library file visits / LOC | Explanation |
|---|---:|---:|---|
| ui-model | 543 / 193,189 | 378 / 127,759 | 375 own source files plus three `#[path]` core sources; 168 integration files / 66,816 LOC are outside lib.rs. |
| GDExtension | 423 / 136,745 | 421 / 136,202 | Two integration files / 543 LOC are outside lib.rs. |

LOC includes comments, blank lines and tests. Wholly test-only reached files account for 8,851 ui-model LOC and 19,282 extension LOC; inline tests add more. These are source inventories, **not executed test counts**. No test deletion explains the difference from the supplied rounded census.

| View | ui-model directed edges / nontrivial SCC sizes | GDExtension directed edges / largest SCC |
|---|---|---|
| Literal production paths | 888 / 79, 13, 12, 4, 2, 2, 2 | 655 / 101 modules, 124,704 LOC |
| Literal including unit tests | 934 / 91, 13, 12, 4, 2, 2, 2 | 676 / 102 modules |
| Production with known UI facade paths resolved | 864 / 62, 13, 12, 4, 2, 2, 2, 2 | Same as literal production |
| Including tests with facade paths resolved | 914 / 74, 13, 12, 4, 2, 2, 2, 2 | Same as literal test view |

Resolving `crate::ui::strata` to the external toolkit and `crate::ui::screens::X` to root module X removes facade-induced coupling, **not actual source dependencies**. The remaining 62-module UI knot is real at this graph granularity: shared chrome imports feature-owned art/helpers, while features import chrome. Its 58,570 LOC include `inworld_unit_frames_component`, `quest_art`, `bank_art`, menus, quests, guilds and several service screens. No mass public-API conversion is proposed.

Concrete witnesses:

| Cycle / incoming edge | Source | Decision |
|---|---|---|
| `quest_art → launcher`, while launcher consumes chrome | `ui-model/src/ui/screens/quest_art.rs:338` uses `launcher::ACTION_CLOSE` | Do not start by moving all chrome into a supposedly independent primitives crate. |
| `bank_art → inworld_unit_frames_component / merchant_frame_component / static_popup_component` | `ui-model/src/ui/screens/bank_art.rs:9–13` | Directory-level screen grouping hides reverse helper dependencies. |
| `bag_data ↔ professions_reagent_counts`; tooltip → professions | `ui-model/src/game/bag_data.rs:343`, `professions_reagent_counts.rs:7` and graph witnesses | Professions is not a leaf; defer rather than introduce service traits. |
| `assets ↔ animation` | `rust/src/assets/mod.rs:16`, `rust/src/animation/mod.rs:1233` | Animation alone is not an independent crate. |
| material → warmup → material/WMO | `rust/src/assets/material.rs:365`, `rust/src/shader_warmup.rs:14–17` | Preserve shader-cache identity and teardown; do not split this with a directory move. |

## Highest-payoff boundaries

Arrows mean consumer → dependency. Keep direct imports, not a facade that makes ui-model depend back on its extracted consumers:

```text
game-engine-godot → ui-housing / ui-adventure / ui-collections → ui-model → core / shared / ui-toolkit
                 → godot-ui-render → core / ui-toolkit / selected ui-model policies / godot support
                 → godot-render   → core / godot support
```

Names below were proposed package suffixes at baseline; current extractions are documented below. No duplicate model definitions, copied `#[path]` implementations, shadow caches or alternative rendering paths.

| Boundary | Current reachable footprint | Why this cut / prerequisites | Risk |
|---|---:|---|---|
| **1. `game-engine-ui-housing`** | 20 housing-prefixed modules / 6,167 LOC, including private tests | Keep the 12-module screen SCC and four-module model SCC together. No observed non-housing production consumer except lib.rs's `StorageMenuTarget` reexport. New crate depends on existing ui-model for chrome, tooltips and map types; native hosts import housing directly. | **Low–medium:** map/tooltip type identity, crate-private helpers, root reexport migration, explicit data-root setup in moved tests. |
| **2. `game-engine-ui-adventure`** | 15 adventure-prefixed modules / 6,545 LOC | Whole 13-module SCC plus tutorial/tooltip companions. No observed incoming production edge from residual ui-model modules. Depends on base item/catalog/chrome/quest helpers; keep native actor/render behavior in extension. | **Medium:** cross-screen integration suites, crate-private helpers, catalog paths; no change to actor framing or selection. |
| **3. `game-engine-ui-collections`** | 12 named modules / 11,629 LOC | `collections_component`, mount journal/source, pet journal/stable/component, wild pet battle, toybox/component, transmog/frame, trading_post. Include wild pet battle because it consumes pet journal. No observed external production consumer of this combined group. Move one family per gate before consolidating the final group. | **Medium:** test-only incoming edges, shared item catalogs/OnceLocks, process-isolated source-switch tests, runtime preview imports. |
| **4. `game-engine-godot-ui-render`** | `ui` owner is 25,895 LOC; candidate low-level file set is only ~7k, not the whole owner | Projection/layout/parts/assets/font metrics/picking/masks and their internal helpers move together after an import-level subgraph. Keep `RegistryUi`, model/action adapters and feature previews in extension. Existing `profile` and `frame_error` are observed lower dependencies: relocate their unchanged implementations once into native support before projection; retain one TLS diagnostic log. `layout.rs` tests currently reach `RegistryModel`; keep host-level tests outside rather than reversing the dependency. | **Medium–high:** gdext class registration, input queue identity, public/private boundaries, shared resource cleanup and layout policy. |
| **5. `game-engine-godot-render`** | Candidate assets/animation/particle/resource group ~16k LOC, **not independently movable today** | Lower M2/material/animation resources below terrain/world orchestration. Audit `world_models`, lighting, scenery-fade and shader-warmup reverse edges first; retain world/terrain ownership and the single extension entry point. Start with an actual closed subset, then extend it. | **High:** 101-module SCC, Godot class registration, shader and mesh caches, main-thread lifetime, init/deinit order. A separate approved cut packet is required before wiring. |

These are compile-isolation candidates, not linear LOC-to-time savings. Editing an extracted feature still dirties that feature and native consumers. The benefit is retaining the much larger **base UI model** and unrelated feature crates as fresh. Shared base edits still fan out. Public generics/inline bodies may instantiate in consumers; the extension's final link does not disappear.

## Ordered execution: one gate per completed step

### Now

- [x] **0 — Baseline packet.** Source/dependency pins, exact edits, check/real-extension-build/UI-lib-test-binary timings, dirty units and the unchanged 473-name unit census are recorded below. Risk: cold caches or host pressure mistaken for extraction benefit. Housing's paired pilot sample is recorded below.
- [x] **1 — Housing pilot (landed `214a9b854`).** First commit only `git mv` of the housing source/test files into the new crate directory. Next wiring commit adds manifest/workspace/lockfile entries, module declarations, exact imports/visibility, native consumers and migrated integration harness declarations. Push the buildable tip for gate. Risk: `StorageMenuTarget`, private helper access and cfg-test data setup. Do not break unrelated base dependencies merely to shrink a table.
- [x] **2 — Adventure Guide (landed `214a9b854`).** Same rename-only then wiring sequence; move the complete feature SCC and its seven currently named Adventure integration modules. Native model-scene/actor code stays put. Risk: cross-feature menu/catalog test inputs and manifest-relative fixture paths.
- [x] **3 — Collection families (landed `68f93ff62`).** First transmog+Trading Post, then journal/toy/pet families at individually pushed gates. Keep collection helpers once, and keep stable/wild-pet consumers on the same side of the cut. Risk: hidden test dependencies and global catalog initialization. Do not create a second monolithic integration-test crate.
### Later

- [ ] **4 — Native UI projection.** Produce a file-level subgraph for the proposed low-level group before moving files; land the unchanged profile/error support relocation at its own gate first. Extract the closed projection set without `RegistryUi` or `GameClient`; host behavioral tests remain host tests. Risk: Godot registration and resource lifetime. Gate includes unchanged rendered/layout/input behavior, not only `cargo check`.
- [ ] **5 — Native model resources.** Recompute the graph after UI extraction; identify exact reverse edges and smallest no-behavior signature/data ownership changes. Preserve root registration, shader-cache serialization and teardown calls. Risk: high; if no small closed cut exists, present that blocker to lead rather than invent traits/backend abstractions.

A rename-only intermediate commit may not compile; never present it as the gated completed tip. Wiring commits contain no moved file bodies mixed with behavioral changes. No bulk `pub(crate)` → `pub` replacement. Expose only existing externally consumed names. Keep the original Godot class names and registration exactly once; a check alone cannot prove native registration works across new rlibs.

New UI crates retain the existing ui-model dev profile (opt0/debug1); new native crates must explicitly retain extension opt2/debug1. Workspace members do **not** automatically inherit the wildcard dependency opt2 override. Preserve feature unification, target kinds, platform dependencies and build-helper snapshot inclusion; new directories remain under `godot/`. Do not change linker/incremental/profile settings during the split.

### Test conservation per gate

Before each move, save `target → fully-qualified name → ignored/enabled` for affected unit and integration targets. After wiring, compare the mapped name multiset, not just totals; reconcile every relocated, remaining and intentionally process-isolated case. Report pass/fail/ignored totals separately. Keep real assertions and fixtures unchanged, including existing production ordering/failure boundaries.

At baseline ui-model used one `integration` harness plus separate `active_db2` and `item_catalog_pending` process targets. Collections stage1 relocates `active_db2` as its own target; `item_catalog_pending` remains in base. Preserve those isolation boundaries; moving tests must not recombine incompatible process-global catalogs. `paths.rs` supplies a default root only under its own `cfg(test)`; when base ui-model becomes a normal dependency of a feature test, that code is **not** test-configured. Moved fixtures must initialize the same explicit real data root before catalog access. Preserve original fixture paths and include paths; do not create asset copies or touch `data/`.

Each completed wiring tip gets targeted helper `cargo check -p <new-crate> -p game-engine-godot -j4` and affected behavioral tests; save Cargo dirty-unit receipts. Full-suite execution and final acceptance remain the lead's gate. Measure the same one-line edit again with no-op and restore controls. Reject comparisons if sibling revisions, feature graphs, profiles or source baselines drift.

## Housing pilot implementation

`godot/ui-housing` now owns the 20 Housing modules and the three Housing integration suites. The existing `game-engine-ui-model` remains their shared dependency; native Housing/tooltip/preview callers depend directly on `game-engine-ui-housing`, with no reverse dependency or façade. Only the already-used `quest_art::close_button` helper gains public visibility. Workspace dev profiles, wire/map/tooltip type identities and all test module/function names remain unchanged. Rename-only commit: `8d140c66a`.

At `faadc498c`, the admitted `-j4` helper check passes for Housing and GDExtension without diagnostics. Runtime census preserves byte-identical fully qualified names: **473 = 436 residual ui-model + 37 Housing units**, and **33 = 33 Housing integration tests**. All 436/37/33 tests execute successfully, with zero failures/ignored. Before-move integration listing used a temporary same-module-name Housing harness because unrelated `dungeonclient.rs` lacked a worktree fixture link; that harness was removed. Parent-authorized shared-data link setup then supplied the original fixture paths without asset writes. Full-suite/integration acceptance remains the lead's gate.

[Machine-readable pilot receipts](engine-crate-split-housing.json) and raw evidence at `/home/osso/.worktrees/logs/engine-crate-split-housing-faadc498c/` retain commands, exact census, dirty units, sibling pins, log hashes and Cargo HTML. The archived baseline build driver is unchanged except the relocated Housing edit path/output directory and omission of the unrequested native trial. Runtime source changes since the baseline are only this split; all four sibling revisions, profiles, feature selections, target path and linker remain matched.

| Exact warm UI-edit extension loop | Before | Housing pilot |
|---|---:|---:|
| No-op wall | 0.954s | 1.161s |
| `House Settings` → `House Settings!` wall | 58.258s | **34.051s** |
| Edit user / system CPU | 12.968 / 2.858s | 8.890 / 2.993s |
| Restore wall | 27.355s | 17.563s |
| Dirty crates | ui-model, GDExtension | Housing, GDExtension; **ui-model fresh** |
| Cargo feature/native unit duration | 23.55 / 33.00s | 1.97 / 21.77s |

This single descriptive edit sample is 24.207s (41.6%) lower; shared-host scheduling still materially affects wall time, so it is not a statistical speedup guarantee. Warmup418.566s is excluded; no-op and restore exit zero, and the reversible edit is absent from the committed tree. Native final compilation/linking remains. No post-split UI-lib-test-binary timing was requested or measured.

**Adventure Guide approved next:** implementation is in progress, preserving the complete15-module feature group and seven named integration modules. Native model-scene/actor code stays in GDExtension. Source changes from concurrent feature landings must be reconciled before gating.

## Adventure Guide extraction

Rename-only `2ba7f80f5` moves21 source files (15 production root modules, their five nested sources and private section tests) plus seven integration suites into `godot/ui-adventure`. The new crate depends on ui-model; native consumers import it directly. Existing shared helpers remain in ui-model; only `inventory_type_label`, `TabLabel`, `TabGeometry`, `tab_width` and `tab` gain required public visibility. Profile/type identities, actor implementation and assertions stay unchanged. The before census is436 residual ui-model units, including four Adventure section tests, plus110 Adventure integration names. At `31dde0e4d`, admitted helper check passes for Adventure, Housing and GDExtension with no diagnostics. All scoped tests pass: **432 residual ui-model units, four Adventure units, 110 Adventure integrations, 37 Housing units and 33 Housing integrations** (616 total; zero failures/ignored). Exact runtime census preserves **436 = 432 + 4** unit names and **110 = 110** integration names; across both extractions, the original473-unit name multiset is **432 + 37 + 4**, byte-identical. Existing section fixtures already initialize the explicit real data root; no assertions or fixture paths changed.

Parent merged tag-action master `58d10a3c9` at `f9be3a1ef`. The newly landed nested Housing module is relocated unchanged in separate rename-only `c77628cbc`; `31dde0e4d` wires its remaining native-test type import. Core tags stay in core. These tag semantics are the parent's independent master change, not part of the no-behavior split. Housing's earlier paired timing remains a historical `faadc498c` sample; no Adventure-specific performance measurement or post-tag timing is claimed.

[Machine-readable Adventure receipts](engine-crate-split-adventure.json) retain names, commands and results. Raw logs/census live at `/home/osso/.worktrees/logs/engine-crate-split-adventure-31dde0e4d/`. Targeted tests/checks are not full-suite/native runtime acceptance; the lead owns that gate. Native actor/model-scene code was not moved, only its UI imports changed.

## Collections staged packet — before moves

Before-packet source `214a9b854`. Parent subsequently confirmed `es-eng` passed/landed and approved stage1; the census below remains the unchanged source baseline. The current graph retains exactly12 root modules,33 reachable source files and11,629 physical LOC, with no outside ui-model production/unit incoming edges. The group includes the four `pet_journal_*` companions, eight `trading_post_*` companions, nested transmog/frame files and wild-pet effect HUD; filename-prefix-only moves would miss descendants.

Populate one `game-engine-ui-collections` crate incrementally, preserving a separately pushed rename-only/wiring/census gate per family:

| Gate | Existing root modules | Source files / LOC | Conserved affected tests |
|---|---|---:|---|
| 1 | `transmog`, `transmog_frame`, `trading_post` | 19 / 6,781 | 18 unit +138 integration +1 isolated `active_db2` |
| 2 | `mount_journal`, `mount_source` | 2 / 684 | 14 integration |
| 3 | `toybox`, `toybox_component` | 2 / 661 | 14 integration |
| 4 | `pet_journal`, `pet_stable`, `pet_stable_component`, `wild_pet_battle`, then `collections_component` | 10 / 3,503 | 6 unit +31 integration |

**Keep the original `collections_component` implementation in ui-model until Gate4:** mount, pet and toy still consume that shell. Moving it early would create residual base→feature edges while feature→base already exists. Shell-last avoids a façade, duplicated implementation or new service abstraction. Pet journal and wild-pet consumers stay together; native preview/actor and gameplay behavior stay in GDExtension. First-cut known visibility boundary is the existing `merchant_frame_component::money_colored`; expose only actually consumed existing helpers/signature types after import audit.

The source-qualified [before census](engine-crate-split-collections-before.json) reuses the unchanged432-name ui-model unit listing;24 affected names are18 transmog and6 stable. Runtime listing of the six integration suites finds **197 enabled/zero ignored**: Trading Post124, transmog14, mount14, pet19, toy14, wild-pet12. The separately listed **`active_db2` target has one enabled/zero ignored** Trading Post source-switch test. It must move as its own target, never into integration; `item_catalog_pending` remains in base. Temporary same-module listing harness is removed. These tests were listed, not newly executed. Preserve nested `trading_post/monthly_clock`, `slow_purchase` and `time_sort` modules, exact names/assertions and fixture paths.

At every family gate, compare original432 unit names against residual+extracted names and compare exact affected integration/process-target multisets; run only the admitted serialized `-j4` helper checks/tests. Maintain the same base/core/shared/toolkit type identities, existing profiles and explicit real data-root setup in moved fixtures. The lead owns integration acceptance and operational push/merge; later families still require their own gates.

### Collections stage1 implementation

Rename-only `dfca8fe29` moves19 source and6 test files unchanged into `godot/ui-collections`. Wiring creates `game-engine-ui-collections`, leaves the original shared shell in ui-model, and changes native callers to direct feature imports. `active_db2` remains its own test target in the new crate; `item_catalog_pending` stays in base. Only the existing `money_colored` helper gains public visibility. Actor/preview behavior, type identities, profiles, assertions and fixture paths remain unchanged. At wiring `2896b6d3e`, admitted helper check passes for Collections and GDExtension with no diagnostics. Scoped execution passes **414 residual ui-model units,18 Collections units,138 integration and one isolated `active_db2` test** (571 total; zero failures/ignored). Runtime census preserves **432 = 414 + 18**, **138 = 138** integration and **1 = 1** isolated names byte-identically. Across all extractions the original473-unit multiset remains414 base+37 Housing+4 Adventure+18 Collections. Fixtures/assertions/process isolation are unchanged.

[Stage1 receipts](engine-crate-split-collections-stage1.json) retain exact names/commands/results; raw evidence is `/home/osso/.worktrees/logs/engine-crate-split-collections-stage1-2896b6d3e/`. No new timing, full-suite or native runtime acceptance is claimed. The parent owns the nine-package gate and push/merge; remaining mount/toy/pet/shell files stay in base until later approved stages.

### Collections stage2 implementation

Parent accepted/landed stage1 at `ce946a284`. Rename-only `cf31450bf` moves `mount_journal.rs`, `mount_source.rs` and the mount integration suite unchanged into ui-collections. Wiring keeps the shared shell in base and changes native imports directly. Only the existing `inset_border` helper and its existing re-export gain public visibility; actor/render behavior, profiles and fixtures stay unchanged. The14-name before suite is reused from the source-qualified packet: the three mount files have no diff since `214a9b854`. At wiring `9c3035d6a`, admitted helper check passes for Collections/GDExtension without diagnostics. Scoped tests pass **414 base units,18 Collections units,152 integration and one isolated test** (585 total, zero failures/ignored). Runtime census preserves414 and18 unit names, **138 + 14 = 152** integration names and the isolated active_db2 name byte-identically. The shared shell, toy and pet modules remain in base.

[Stage2 receipts](engine-crate-split-collections-stage2.json) and `/home/osso/.worktrees/logs/engine-crate-split-collections-stage2-9c3035d6a/` retain exact names, commands and results. No new timing, native runtime or full-suite acceptance claim; parent owns the unchanged nine-package gate and operational push/merge.

### Collections stage3 implementation

Parent approved toy stage3 while stage2's gate was queued. Rename-only `6a77dd742` moves toy model/component and the14-name integration suite unchanged into ui-collections. Native consumers import Collections directly; shared shell and pet modules remain in base. Stage2 already exposed the existing inset helper, so stage3 adds no visibility/dependency/profile changes. The before packet is source-qualified: the three toy files have no diff since `214a9b854`. At wiring `bfcdbe17c`, admitted helper check Collections/GDExtension passes with no diagnostics. Scoped tests pass **414 base units,18 Collections units,166 integration and one isolated test** (599 total, zero failures/ignored). Runtime census preserves414/18 unit names, **152 + 14 = 166** integration names and one isolated name byte-identically; no assertions or fixtures changed.

[Stage3 receipts](engine-crate-split-collections-stage3.json) and `/home/osso/.worktrees/logs/engine-crate-split-collections-stage3-bfcdbe17c/` retain exact names/commands/results. The parent owns the unchanged nine-package gate, push and merge; no new timing/native runtime/full-suite acceptance claim. Pet/shell remain for stage4.

### Collections stage4 implementation

Parent approved pet/shell stage4 while toy's gate was queued. Rename-only `baaf42472` moves10 sources and two suites unchanged, completing the33-source/12-root Collections group in ui-collections. The stable component keeps its explicit `ui/screens` path; all pet_journal companions and wild-pet effect HUD stay with their owners. Native imports now point directly at Collections. Existing mount/toy shell references become local crate references, not self-external aliases or a base façade. The stable render unit fixture already initializes the explicit real data root; its base paths call is namespace-adjusted without changing assertions/data. Before sources/suites have no diff since `214a9b854`. The moved pet snapshot already derives `serde::Deserialize`; `46ba60eb6` declares that same existing serde version/derive feature on its new owner after the initial missing-direct-dependency check failure. No dependency versions or global feature selection changed. At `46ba60eb6`, admitted helper check Collections/GDExtension passes with no diagnostics. Scoped tests pass **408 base units,24 Collections units,197 integration and one isolated test** (630 total, zero failures/ignored).

Exact runtime census conserves **414 = 408 + 6** base unit names, **18 + 6 = 24** Collections unit names, **166 + 31 = 197** integration names and one isolated name. The final197 integration names equal the complete original six-suite packet. Across all feature extractions, **all original473 unit names = 408 base + 37 Housing + 4 Adventure + 24 Collections**, byte-identical.

[Stage4 receipts](engine-crate-split-collections-stage4.json) and `/home/osso/.worktrees/logs/engine-crate-split-collections-stage4-46ba60eb6/` retain exact names, commands and results. No base→Collections source edges or self-external aliases remain. This completes the planned leaf UI source group, not the overall compile-time goal or native-render cuts. No new timing/native-runtime/full-suite acceptance claim; parent owns the unchanged nine-package gate and push/merge.

## Next approved group — Professions presentation

At `68f93ff62`, remaining ui-model is304 reachable source files/103,366 physical LOC. The62-module shared SCC still owns58,570LOC. The largest high-churn **closed leaf candidate** audited next is Professions presentation: five roots,19 source files/4,204LOC and61 distinct non-merge commits in the pinned14-day interval. This is not a claim that it gives the largest possible saving after arbitrary architectural changes.

| Candidate | Source LOC /14d distinct commits | Boundary |
|---|---:|---|
| Professions presentation | 4,204 /61 | No residual production incoming edges; approved next |
| PvP/battleground | 1,702 /44 | Closed smaller alternative |
| Spellbook/talents/overrides/preview | 5,471 /52 | micro_menu returns feature-owned PlayerSpellsTab; defer coupling fix |
| Achievements | 3,466 /65 | Objective tracker consumes AchievementWindow; adding tracker exposes five more outside consumers |

Approved `game-engine-ui-professions` owns `professions_frame`, `professions_cooldown`, `professions_orders`, `professions_reagent_selection` and `professions_specializations` with all descendants. Keep ProfessionBook/Recipe (`professions`), quality, reagent counts **and RecipeCatalog** in base for tooltip/bag/trainer consumers. Before census:14 units and37 dedicated integration cases, plus the existing Professions cases in `buttonfit` and `panel_lifecycle`. Move only those two mixed-suite cases with exact module::test names; unrelated auction/character/raid siblings stay base. No base reverse-dependency or copied production implementations.

[Before packet](engine-crate-split-professions-before.json) retains exact names and measurement receipts. The reversible real-build edit is `professions_frame_tree.rs`: `No matching recipes` → `No matching recipes!`. Current warm extension wall **32.599s**, user/system17.298/8.503s; restore23.520s. No-op10.300s has zero dirty units and includes helper/slot wait. Warmup403.342s is excluded. Cargo attributes9.70s to base UI and21.89s to native; Housing1.98s, Adventure2.05s and Collections2.34s also rebuild. These overlapping durations are not additive. A cut should keep base/other feature crates fresh for this edit, but saving is **unmeasured until post-cut**; the native tail remains.

Parent approved separate rename-only/wiring/census commits and paired post-cut timing, after ec4 landed `68f93ff62`. Preserve14 unit and39 total moved integration names, source fixture roots, data/type ownership and profiles. Initial before suite listing included83 names across relevant/comparison suites and zero ignored; only39 belong to this cut. Full gate remains parent-owned. Raw before evidence: `/home/osso/.worktrees/logs/engine-crate-split-professions-before-68f93ff62/`. No Professions source move is part of this before packet.

### Professions extraction implementation

Before packet committed `3053ece16`; rename-only `e168fc1a3` moves19 source files/three dedicated suites unchanged. Wiring creates `game-engine-ui-professions` with direct native imports and unchanged base model/catalog ownership. The two existing Professions integration cases in mixed suites relocate, retaining `buttonfit::buttonfit_create_all_keeps_retail_padding_and_right_anchor_for_concrete_counts` and `panel_lifecycle::ui_template_lifecycle_root_professions_unmounts_without_losing_registration_on_reopen`; unrelated cases remain base. Test-local helper copies do not duplicate production code. A test-inclusive falsification audit additionally finds four base `professions::tests::*` cases calling the moved frame/cooldown; `8b773aa0f` relocates those unchanged bodies into a **cfg(test)-only namespace** with the same names, not a production model/facade. Thus the root-owned14-unit before count expands to18 conserved moved units. Base ProfessionBook/Recipe/quality/counts/catalog implementation files remain unchanged.

At `820a57342`, helper feature/native check passes without diagnostics; the subsequent four-case relocation changes only tests and their cfg(test) declaration, not that production proof scope. Scoped execution at `8b773aa0f` passes **390 residual units,18 feature units,39 feature integration and three residual mixed-suite siblings** (450 total, zero failures/ignored). Runtime census conserves **408 = 390 + 18** units, **37 + 2 = 39** feature integration names, and **5 mixed integration cases = 3 residual + 2 moved**. All original473 unit names now equal390base+37Housing+4Adventure+24Collections+18Professions byte-identically. [Professions result receipt](engine-crate-split-professions.json) binds the census/check/test logs and timing pair; raw post-cut evidence is `/home/osso/.worktrees/logs/engine-crate-split-professions-8b773aa0f/`.

| Same reversible Professions edit | Before `68f93ff62` | After `8b773aa0f` |
|---|---:|---:|
| No-op wall (no dirty units) | 10.300s | 9.985s |
| Edit wall | **32.599s** | **38.389s** |
| Edit user + system CPU | 25.801s | 10.119s |
| Restore wall | 23.520s | 13.100s |
| Changed feature/base unit | base9.70s | Professions1.50s |
| Native unit | 21.89s | 35.04s |
| Dirty UI crates | base +Housing/Adventure/Collections | Professions only; all four others fresh |

**No measured edit-wall improvement:** the single post-cut edit sample is5.790s/17.8% slower. Compilation isolation is proven and child CPU is60.8% lower; the native unit grew from21.89s to35.04s. Restore is faster, but is a separate control, not a replacement result. Shared-host scheduling and helper-slot wait make one sample insufficient for a latency guarantee. Common Cargo artifact feature/profile multisets have zero differences, sibling pins match before/after, and the input edit is restored. Warmup197.630s is excluded. No success-biased rerun or timing/native-runtime/full-suite claim; parent owns the ten-package gate.

## Player Spells presentation — approved next slice

At `1123c7606`, residual base owns285 reachable files/98,957LOC. Ranking uses source size,14-day distinct git commits and actual coupling, **not invented per-module compiler times**: Player Spells UI+preview3878LOC/47commits offers the largest audited high-churn presentation-only cut with two tiny shared-data edges; PvP1702LOC/44 is the smaller closed alternative. Achievements3466LOC/65 includes substantial authoritative state/comparison caching and must first separate rendering. Unit frames8260LOC/6 is much larger but lower-churn/shared-art-coupled. Churn interval and exact file inventory are in the [before packet](engine-crate-split-player-spells-before.json).

The approved `game-engine-ui-player-spells` keeps exactly35 existing shared lines in base's real `player_spells_data` module: PlayerSpellsTab enum+impl and SpellbookItemView struct. Micro menu/overrides import that data directly. Existing talents, TalentEditor, loadouts, search and spell_overrides implementations remain base. No façade/re-export aliases, new data definitions or behavior changes. Eleven existing presentation/preview source files contain3878LOC;3843 existing lines move after retaining those35. Native consumers import the feature directly.

Before source `1123c7606`:390 unit names;15 affected integration suites list94 names, **93 enabled/one ignored**. Eleven dedicated suites relocate unchanged; four mixed cases (one micro menu, two PvP, one text-fit) retain original module::test names on the feature side. Their21 unrelated siblings remain base. Expected feature list73 names (72enabled/one ignored), residual21; the ignored data-only classbook audit remains ignored.

The paired real-build baseline changes only `pvp_talent_controls.rs`: `let text = "PvP Talents";` → `let text = "PvP Talents!";`. Warm edit **23.287s**, child CPU17.509s, no-op0.832s/zero dirty units, restore24.318s; all exit0/input restored. Cargo base unit5.91s and native16.06s are package-level evidence, not module attribution; all four extracted UI features also rebuild. The post-cut comparison must preserve pins/profiles/flags, record differing load/controls and avoid a wall-speed guarantee.

Before packet `a154708f7`; rename-only `534ae04b1` moves11 presentation sources+11 dedicated suites unchanged. Wiring retains shared types once in base and migrates four mixed cases without changing assertions. Wiring `d8d935ea2` passes the feature/GDExtension helper check with zero diagnostics. Source/tests compile with original model ownership; `c6dfa999e` deletes an orphaned base test helper after its sole caller moves (no warning suppression), then reruns only the21 affected residual cases. Scoped execution is **390 base units,72 feature integration cases,21 residual mixed cases passed; zero failures and one unchanged ignored audit**. Feature unit count is zero. Exact census preserves390 units and **94 integration names = 73 feature (one ignored) + 21 residual**; original473 UI unit names remain unchanged across all crates.

[Player Spells result receipt](engine-crate-split-player-spells.json) binds commands/names/load controls; raw post-cut evidence is `/home/osso/.worktrees/logs/engine-crate-split-player-spells-d8d935ea2/`.

| Same reversible PvP label edit | Before `1123c7606` | After `d8d935ea2` |
|---|---:|---:|
| No-op wall | 0.832s | 1.045s |
| Edit wall | 23.287s | 22.914s |
| Edit child user+system CPU | 17.509s | 12.139s |
| Restore wall | 24.318s | 16.453s |
| Changed base/feature unit | base5.91s | PlayerSpells0.85s |
| Native unit | 16.06s | 21.54s |
| Dirty UI crates | base +all four extracted features | PlayerSpells only; all five others fresh |

**No wall-speed claim under unequal shared-host load.** The descriptive wall samples differ by0.374s, while child CPU is30.7% lower and dependency isolation is proven. The native unit grows16.06→21.54s; before/after restore/no-op controls and admission waits remain separate. Common Cargo feature/profile multisets have zero differences, sibling pins match and input is restored. Post-cut warmup177.208s is excluded (before warmup1.991s had zero dirty units, already warm). The subsequent helper-only test deletion does not intersect the measured production scope. Parent owns the eleven-package integration gate; no native runtime/full-suite acceptance is claimed. Raw before evidence: `/home/osso/.worktrees/logs/engine-crate-split-player-spells-before-1123c7606/`.

## PvP presentation implementation

Parent accepted Player Spells at `abd6a4ddc`, then authorized PvP or achievement presentation. PvP is selected for the better isolation-to-effort ratio: four closed files/1702LOC, versus achievement rendering mixed with authoritative state/comparison caches. Before packet `5af0fd2ab`; rename-only `b5e1f98e7` moves four source/three dedicated test files unchanged into `game-engine-ui-pvp`. Native match/actor behavior and wire authority remain unchanged.

One mixed `selection_lists` case moves with its exact module::test name; three unrelated siblings stay base. Existing two cross-panel Player Spells cases stay in their peer crate through a **test-only** dependency on ui-pvp. Neither base nor Player Spells production gains a reverse dependency; no façade or new shared definitions. Before census is390 unchanged unit names and36 affected integration names:31 to PvP, three residual base and two peer cases; zero ignored.

The reversible Honor-label edit `"Honor Level —"` → `"Honor Level —!"` takes42.452s before the cut, childCPU27.386s; no-op4.935s, restore53.125s. Controls include existing slot/helper wait, matching sibling pins/profiles and input restoration. [Before receipt](engine-crate-split-pvp-before.json), raw `/home/osso/.worktrees/logs/engine-crate-split-pvp-before-abd6a4ddc/`. Wiring check/tests/census and post-cut timing pending; no wall-speed claim under unequal load.

## Baseline edit measurements

[Machine-readable receipts](engine-crate-split-baseline.json) include exact commands, package/target dirtiness, wall/user/system time, dependency pins, Cargo unit timings and raw-log hashes. Host `agent-server`, Rust/Cargo 1.99.0, four jobs, native helper defaults and unchanged profiles/linker. Raw logs, HTML timings, drivers and before/after name lists are archived outside the worktree at `/home/osso/.worktrees/logs/engine-crate-split-baseline-4645578c7/`. No post-split improvement is claimed.

Two reversible **code** edits, not unchanged-source touches; neither was launched or committed:

- Housing: `housing_settings_component.rs`, title `House Settings` → `House Settings!`.
- Native: `animation/mod.rs`, `MIN_MOVEMENT_BLEND_MS` 150.0 → 151.0.

| Loop | Warm no-op wall | UI edit wall / user / system | Native edit wall / user / system | Dirty targets after edit |
|---|---:|---:|---:|---|
| Real extension build, cdylib+rlib | 0.954s | **58.258 / 12.968 / 2.858s** | **56.077 / 95.944 / 6.099s** | UI edit: ui-model + GDExtension. Native edit: GDExtension only. |
| UI-model library test-binary build (`--lib --no-run`) | 0.995s | **14.680 / 5.369 / 0.850s** | Not requested | Only the ui-model lib-test executable; not integration harnesses. |
| Extension Cargo check | 1.040s | 13.122 / 9.988 / 1.899s | 5.423 / 4.181 / 0.625s | Same package sets as real extension build. |

All calls exited zero. Core, network, session, shared, toolkit and third-party crates stayed fresh in warm edit trials. UI library test inventory before/after is byte-identical: **473 enabled, zero ignored**. These tests were **listed, not executed**; no behavioral/full-suite acceptance is implied. The three Python graph fixtures pass. Rust workspace diff against `4645578c7` is empty after restoration.

Cold/profile warmups are separate and excluded: check **395.471s / 621 nonfresh artifact events**, extension build **812.086s / 307**, test-binary build **530.133s / 116**. The test loop has a different dependency feature graph; never treat its first profile compilation as the steady-state cost of a UI edit.

Cargo attributes the UI-edit real build to ui-model **23.55s** (frontend 16.57s, codegen 6.98s) then GDExtension **33.00s**. The native edit spends **54.51s** in the GDExtension unit. Its frontend/codegen/link partition is unavailable in Cargo HTML: these totals are **not link-only measurements**. UI-only extraction does not remove that native tail. Housing is a safe mechanics pilot, not completion of the compile-time goal; the lead should reassess subsequent priorities against the pilot's paired results.

Restore controls: UI extension build 27.355s, native extension build 43.986s, UI test-binary build 6.535s; check restores 10.993s/4.427s. UI build edit/restore CPU totals are similar (15.827s/16.545s) despite substantially different wall times. Shared-host scheduling/pressure affects latency; this is one descriptive sample per state, not a confidence interval or a post-split speedup forecast.

Reproduction uses archived `measure.py` and `measure-real.py`, copied back under `<checkout>/target/crate-split/`. Run modes sequentially:

```text
scripts/agent/agent-run lead_esplit_15135 env TMPDIR=/var/tmp/osso python3 target/crate-split/measure.py
scripts/agent/agent-run lead_esplit_15135 env TMPDIR=/var/tmp/osso python3 target/crate-split/measure-real.py build
scripts/agent/agent-run lead_esplit_15135 env TMPDIR=/var/tmp/osso python3 target/crate-split/measure-real.py test
```

The build driver calls `depot.build(root, host="native")`, the real dev helper path including its checkout lock/native slot. A measurement-only wrapper adds `-j4 --message-format=json --timings` to the existing `native_cargo` invocation and saves its output; no repository build helper is changed. Test mode calls `depot.run_tests_native` with `-p game-engine-ui-model --lib --no-run` and the same instrumentation. Check mode calls the helper's `native_cargo("check", ...)` under its native slot. Each driver performs ADD/HOLD/SHED admission, records cold/no-op/edit/restore separately, and restores inputs in `finally`. No build jobs overlap. Build/test trials assert unchanged sibling revisions before/after and between samples; all four sibling checkouts were clean at final inspection. Admission wait is outside reported wall time; slot wait/helper/Cargo are inside. CPU totals cover child processes, not host-wide usage.

## Sources

- [Generated module graph](engine-crate-split-graph.md), [baseline receipts](engine-crate-split-baseline.json), `scripts/engine-crate-graph.py`, `scripts/test_engine_crate_graph.py` — exact SCC membership, adjacency/churn, fixture proof and edit timings.
- game-server `0d8542f46:docs/wiki/investigations/server-crate-split-plan.md` and `scripts/server-crate-graph.py` — reference method and staged move/wiring discipline.
- [Workspace manifest](../../../godot/Cargo.toml), [ui-model root](../../../godot/ui-model/src/lib.rs), [extension root](../../../godot/rust/src/lib.rs) and their manifests — actual module/type/profile ownership.
- [Build helper](../../../scripts/depot-build.py), [remote/native build contract](../../remote-builds.md) — commands, snapshot/target/slot behavior.
- [Historical touch timings](native-dev-build-timings.md) — unchanged-file touch evidence, not a true code-edit baseline or current split speedup.

## See Also

- [[godot-conversion]] — native client boundary and retained behavior.
- [[ui-system]] — registry, SharedContext and projection behavior to preserve.
- [[build-hosts]] — build/dependency/cache boundary.
