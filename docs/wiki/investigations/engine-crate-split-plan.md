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

Names below were proposed package suffixes at baseline; current Housing/Adventure extractions are documented below. No duplicate model definitions, copied `#[path]` implementations, shadow caches or alternative rendering paths.

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
- [ ] **1 — Housing pilot.** First commit only `git mv` of the housing source/test files into the new crate directory. Next wiring commit adds manifest/workspace/lockfile entries, module declarations, exact imports/visibility, native consumers and migrated integration harness declarations. Push the buildable tip for gate. Risk: `StorageMenuTarget`, private helper access and cfg-test data setup. Do not break unrelated base dependencies merely to shrink a table.
- [ ] **2 — Adventure Guide.** Same rename-only then wiring sequence; move the complete feature SCC and its seven currently named Adventure integration modules. Native model-scene/actor code stays put. Risk: cross-feature menu/catalog test inputs and manifest-relative fixture paths.
- [ ] **3 — Collection families.** First transmog+Trading Post, then journal/toy/pet families at individually pushed gates. Keep collection helpers once, and keep stable/wild-pet consumers on the same side of the cut. Risk: hidden test dependencies and global catalog initialization. Do not create a second monolithic integration-test crate.
### Later

- [ ] **4 — Native UI projection.** Produce a file-level subgraph for the proposed low-level group before moving files; land the unchanged profile/error support relocation at its own gate first. Extract the closed projection set without `RegistryUi` or `GameClient`; host behavioral tests remain host tests. Risk: Godot registration and resource lifetime. Gate includes unchanged rendered/layout/input behavior, not only `cargo check`.
- [ ] **5 — Native model resources.** Recompute the graph after UI extraction; identify exact reverse edges and smallest no-behavior signature/data ownership changes. Preserve root registration, shader-cache serialization and teardown calls. Risk: high; if no small closed cut exists, present that blocker to lead rather than invent traits/backend abstractions.

A rename-only intermediate commit may not compile; never present it as the gated completed tip. Wiring commits contain no moved file bodies mixed with behavioral changes. No bulk `pub(crate)` → `pub` replacement. Expose only existing externally consumed names. Keep the original Godot class names and registration exactly once; a check alone cannot prove native registration works across new rlibs.

New UI crates retain the existing ui-model dev profile (opt0/debug1); new native crates must explicitly retain extension opt2/debug1. Workspace members do **not** automatically inherit the wildcard dependency opt2 override. Preserve feature unification, target kinds, platform dependencies and build-helper snapshot inclusion; new directories remain under `godot/`. Do not change linker/incremental/profile settings during the split.

### Test conservation per gate

Before each move, save `target → fully-qualified name → ignored/enabled` for affected unit and integration targets. After wiring, compare the mapped name multiset, not just totals; reconcile every relocated, remaining and intentionally process-isolated case. Report pass/fail/ignored totals separately. Keep real assertions and fixtures unchanged, including existing production ordering/failure boundaries.

UI-model currently uses one `integration` harness plus the separate `active_db2` and `item_catalog_pending` process targets. Preserve those isolation boundaries; moving tests must not recombine incompatible process-global catalogs. `paths.rs` supplies a default root only under its own `cfg(test)`; when base ui-model becomes a normal dependency of a feature test, that code is **not** test-configured. Moved fixtures must initialize the same explicit real data root before catalog access. Preserve original fixture paths and include paths; do not create asset copies or touch `data/`.

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

Source `214a9b854`, pending lead `es-eng` landing. **Planning/census only; no Collections source moves yet.** The current graph retains exactly12 root modules,33 reachable source files and11,629 physical LOC, with no outside ui-model production/unit incoming edges. The group includes the four `pet_journal_*` companions, eight `trading_post_*` companions, nested transmog/frame files and wild-pet effect HUD; filename-prefix-only moves would miss descendants.

Populate one `game-engine-ui-collections` crate incrementally, preserving a separately pushed rename-only/wiring/census gate per family:

| Gate | Existing root modules | Source files / LOC | Conserved affected tests |
|---|---|---:|---|
| 1 | `transmog`, `transmog_frame`, `trading_post` | 19 / 6,781 | 18 unit +138 integration +1 isolated `active_db2` |
| 2 | `mount_journal`, `mount_source` | 2 / 684 | 14 integration |
| 3 | `toybox`, `toybox_component` | 2 / 661 | 14 integration |
| 4 | `pet_journal`, `pet_stable`, `pet_stable_component`, `wild_pet_battle`, then `collections_component` | 10 / 3,503 | 6 unit +31 integration |

**Keep the original `collections_component` implementation in ui-model until Gate4:** mount, pet and toy still consume that shell. Moving it early would create residual base→feature edges while feature→base already exists. Shell-last avoids a façade, duplicated implementation or new service abstraction. Pet journal and wild-pet consumers stay together; native preview/actor and gameplay behavior stay in GDExtension. First-cut known visibility boundary is the existing `merchant_frame_component::money_colored`; expose only actually consumed existing helpers/signature types after import audit.

The source-qualified [before census](engine-crate-split-collections-before.json) reuses the unchanged432-name ui-model unit listing;24 affected names are18 transmog and6 stable. Runtime listing of the six integration suites finds **197 enabled/zero ignored**: Trading Post124, transmog14, mount14, pet19, toy14, wild-pet12. The separately listed **`active_db2` target has one enabled/zero ignored** Trading Post source-switch test. It must move as its own target, never into integration; `item_catalog_pending` remains in base. Temporary same-module listing harness is removed. These tests were listed, not newly executed. Preserve nested `trading_post/monthly_clock`, `slow_purchase` and `time_sort` modules, exact names/assertions and fixture paths.

At every family gate, compare original432 unit names against residual+extracted names and compare exact affected integration/process-target multisets; run only the admitted serialized `-j4` helper checks/tests. Maintain the same base/core/shared/toolkit type identities, existing profiles and explicit real data-root setup in moved fixtures. No moves are authorized before `es-eng` lands; lead owns integration acceptance and operational push/merge.

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
