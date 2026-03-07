# WoW CLI Feature Expansion Implementation Plan

**Goal:** Expand the CLI beyond auction house, mail, guild, and chat with high-value WoW systems, including a `cast spell on target` command that reuses in-engine targeting and networking paths.

**Architecture:** Keep the CLI as a thin front end over the existing IPC bridge. Each new feature should follow the current pattern: add request/response types in `src/ipc/mod.rs`, route them in `src/ipc/plugin.rs`, store or derive state from focused resources/snapshots, and format human-readable output in `src/bin/game-engine-cli.rs`. For features that require server authority, send intents through the existing networking layer rather than implementing client-only behavior.

**Tech Stack:** Rust 2024, Bevy 0.18, `clap`, `serde`, `peercred-ipc`, Lightyear client networking, shared protocol types from `/syncthing/Sync/Projects/wow/game-server/crates/shared`

---

## Implementation Status (2026-03-06)

This plan has largely been implemented. Current repo state already includes the full Tier 1 surface and most Tier 2 read-only reports.

Completed from this plan:
- Task 1: shared CLI command-family scaffolding (`inventory`, `quest`, `group`, `spell`, `combat`, `reputation`, `collection`, `profession`, `map`)
- Task 2: inventory and storage inspection (`inventory list`, `search`, `whereis`)
- Task 3: quest snapshots and views (`quest list`, `watch`, `show`)
- Task 4: group roster and invite/uninvite (`group roster`, `status`, `invite`, `uninvite`)
- Task 5: spell cast intent path (`spell cast`, `spell stop`) with current-target resolution and network intent submission
- Task 6: combat log and recap text views (`combat log`, `combat recap`)
- Task 7: progression reports (`reputation list`, `collection mounts/pets`, `profession recipes`)
- Task 8: map and waypoint utilities (`map position`, `target`, `waypoint add`, `waypoint clear`)

Already present in code:
- IPC request/dispatch support in `src/ipc/mod.rs` and `src/ipc/plugin.rs`
- CLI request mapping and tests in `src/bin/game-engine-cli.rs`
- Snapshot formatting for the above features in `src/ipc/plugin.rs`

Remaining optional follow-ups (not blockers):
- add `--json` output mode once text output contracts are finalized
- deepen server-backed semantics where snapshots are still minimal
- expand per-feature integration tests beyond parser/request-mapping coverage

Quick regression commands:

```bash
cargo test spell_cast
cargo test inventory_search
cargo test quest_list
cargo test group_roster
cargo test combat_log
cargo test reputation_list
cargo test map_target
./run-tests.sh
```

---

## Feature Ideas and Priority

**Tier 1: implement next**
- `inventory` and storage inspection: bags, bank, reagent bank, guild vault, warbank, equipped gear search
- `quests`: active quest list, objectives, turn-in status, daily/weekly grouping
- `group`: party or raid roster, role summary, ready-check style status, invite or kick intents
- `spell cast`: cast by spell ID or name onto the current target or an explicit target identifier
- `combat log`: recent damage, heals, interrupts, aura events, death recap text output

**Tier 2: add after the core loop exists**
- `professions`: recipe lookup, craftability from known inventory, cooldown status
- `reputation` and `achievements`: progress reports and watchlists
- `collections`: mounts, pets, toys, appearances, missing-state summaries
- `social`: friends, ignore, who, alt notes, presence summaries
- `map`: coordinates, waypoint creation, target distance, nearby POI inspection

**Non-goals for v1**
- full macro language execution
- unattended combat rotations or bot behavior
- spell queueing beyond a single cast intent
- retail-complete quest log semantics
- account-wide persistence for every report before the underlying systems exist in engine/server

## Command Surface

Start with a consistent top-level shape:

```text
game-engine-cli status character-stats
game-engine-cli status bags
game-engine-cli inventory search --text linen
game-engine-cli quest list
game-engine-cli quest watch
game-engine-cli group roster
game-engine-cli spell cast --spell 133 --target current
game-engine-cli combat log --lines 30
game-engine-cli reputation list
game-engine-cli collection mounts --missing
game-engine-cli map target
```

Rules:
- default output is human-readable text
- add `--json` only after the text output stabilizes
- prefer subcommands that read like game nouns, not transport verbs
- use `status` for passive snapshots and feature nouns for richer workflows

### Task 1: Establish shared CLI expansion conventions

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Test: `src/bin/game-engine-cli.rs`

**Step 1: Write the failing test**

Add parser and request-mapping tests that define the target command families without implementation details:

```rust
#[test]
fn spell_cast_command_maps_to_ipc_request() {
    let request = spell_request(SpellCmd::Cast {
        spell: "133".into(),
        target: Some("current".into()),
    })
    .expect("valid spell cast command");

    assert!(matches!(request, Request::SpellCast { .. }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test spell_cast_command_maps_to_ipc_request`
Expected: FAIL because `SpellCmd`, `spell_request`, or `Request::SpellCast` do not exist yet.

**Step 3: Write minimal implementation**

Add empty or placeholder command families:
- `inventory`
- `quest`
- `group`
- `spell`
- `combat`
- `reputation`
- `collection`
- `map`

Keep handlers thin and initially return placeholder `Response::Text` until each subsystem is implemented.

**Step 4: Run test to verify it passes**

Run: `cargo test spell_cast_command_maps_to_ipc_request`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs
git commit -m "feat: scaffold expanded CLI command families"
```

### Task 2: Inventory and storage inspection

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Modify: `src/status.rs`
- Modify: `src/auction_house.rs`
- Test: `src/status.rs`
- Test: `src/bin/game-engine-cli.rs`

**Step 1: Write the failing test**

Add tests for:
- `inventory search --text torch` mapping to `Request::InventorySearch`
- formatter behavior when no items match
- formatter behavior for grouped matches by storage source

Use concrete test data:

```rust
let snapshot = InventorySearchSnapshot {
    entries: vec![InventoryItemEntry {
        storage: "bags".into(),
        slot: 4,
        item_guid: 101,
        item_id: 25,
        name: "Worn Shortsword".into(),
        stack_count: 1,
    }],
};
```

**Step 2: Run test to verify it fails**

Run: `cargo test inventory_search`
Expected: FAIL because `InventorySearchSnapshot` and `Request::InventorySearch` are missing.

**Step 3: Write minimal implementation**

Add a read-only inventory search path that aggregates from:
- bag snapshot state already exposed through `Request::BagsStatus`
- existing storage snapshots in `src/status.rs`

Initial commands:
- `inventory list`
- `inventory search --text <term>`
- `inventory whereis --item-id <id>`

Do not add destructive inventory actions yet.

**Step 4: Run test to verify it passes**

Run: `cargo test inventory_search`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs src/status.rs src/auction_house.rs
git commit -m "feat: add inventory and storage CLI inspection"
```

### Task 3: Quest log snapshots and watch views

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Modify: `src/status.rs`
- Modify: `src/networking.rs`
- Modify: `/syncthing/Sync/Projects/wow/game-server/crates/shared/src/protocol/mod.rs`
- Test: `src/status.rs`
- Test: `src/bin/game-engine-cli.rs`

**Step 1: Write the failing test**

Add tests for:
- `quest list` mapping to `Request::QuestList`
- formatting an active quest with objective counters
- formatting a daily quest marker separately from a normal quest

**Step 2: Run test to verify it fails**

Run: `cargo test quest_list`
Expected: FAIL because quest request and snapshot types do not exist.

**Step 3: Write minimal implementation**

Define a small replicated quest snapshot in shared protocol and consume it in the engine:
- active quests
- title
- zone
- objective text
- completed flag
- repeatability flag (`daily`, `weekly`, `normal`)

Expose commands:
- `quest list`
- `quest watch`
- `quest show --id <quest_id>`

Avoid quest abandon, accept, or turn-in commands until NPC interaction exists.

**Step 4: Run test to verify it passes**

Run: `cargo test quest_list`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs src/status.rs src/networking.rs /syncthing/Sync/Projects/wow/game-server/crates/shared/src/protocol/mod.rs
git commit -m "feat: add quest log CLI views"
```

### Task 4: Group and social roster commands

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Modify: `src/status.rs`
- Modify: `src/networking.rs`
- Modify: `/syncthing/Sync/Projects/wow/game-server/crates/shared/src/protocol/mod.rs`
- Test: `src/status.rs`
- Test: `src/bin/game-engine-cli.rs`

**Step 1: Write the failing test**

Add tests for:
- `group roster` request mapping
- roster formatter showing leader, role, online state, and subgroup
- invite command mapping without attempting live network delivery in the unit test

**Step 2: Run test to verify it fails**

Run: `cargo test group_roster`
Expected: FAIL because group request and snapshot types do not exist.

**Step 3: Write minimal implementation**

Expose:
- `group roster`
- `group status`
- `group invite --name <player>`
- `group uninvite --name <player>`

Keep the read path richer than the write path. For v1, the CLI should surface current roster state clearly even if invite results are only server-acknowledged text.

**Step 4: Run test to verify it passes**

Run: `cargo test group_roster`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs src/status.rs src/networking.rs /syncthing/Sync/Projects/wow/game-server/crates/shared/src/protocol/mod.rs
git commit -m "feat: add group roster and invite CLI commands"
```

### Task 5: Cast spell on target

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Modify: `src/target.rs`
- Modify: `src/networking.rs`
- Modify: `src/status.rs`
- Modify: `/syncthing/Sync/Projects/wow/game-server/crates/shared/src/protocol/mod.rs`
- Test: `src/bin/game-engine-cli.rs`
- Test: `src/target.rs`
- Test: `src/networking.rs`

**Step 1: Write the failing test**

Add tests for:
- `spell cast --spell 133 --target current` mapping to `Request::SpellCast`
- target resolution when `current` is selected but no target exists
- network intent building uses the currently selected target entity bits when present

Use explicit cases:
- spell by numeric ID: `133`
- spell by exact token name: `Fireball`
- target modes: `current`, explicit entity bits, explicit unit GUID if the protocol supports it

**Step 2: Run test to verify it fails**

Run: `cargo test spell_cast`
Expected: FAIL because spell cast request types and handlers do not exist.

**Step 3: Write minimal implementation**

Create a thin cast-intent path:
- CLI parses `spell cast --spell <id-or-name> --target <current|entity|guid>`
- IPC sends `Request::SpellCast`
- Bevy main thread resolves `current` through `CurrentTarget` in `src/target.rs`
- networking sends a client intent to the server using the existing target sync pattern in `src/networking.rs`

Minimal command set:
- `spell cast --spell 133 --target current`
- `spell cast --spell Fireball --target 123456789`
- `spell stop`

Validation rules:
- fail fast if no current target exists
- fail fast if spell identifier cannot be resolved
- do not bypass server validation for range, line of sight, mana, cooldown, or hostile/friendly checks

**Step 4: Run test to verify it passes**

Run: `cargo test spell_cast`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs src/target.rs src/networking.rs src/status.rs /syncthing/Sync/Projects/wow/game-server/crates/shared/src/protocol/mod.rs
git commit -m "feat: add spell cast CLI targeting"
```

### Task 6: Combat log and encounter text views

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Modify: `src/status.rs`
- Modify: `src/networking.rs`
- Test: `src/status.rs`
- Test: `src/bin/game-engine-cli.rs`

**Step 1: Write the failing test**

Add tests for:
- `combat log --lines 10` mapping
- output formatting for damage, heal, interrupt, aura apply, and death lines
- death recap ordering from newest to oldest

**Step 2: Run test to verify it fails**

Run: `cargo test combat_log`
Expected: FAIL because combat log snapshot and request types do not exist.

**Step 3: Write minimal implementation**

Store a bounded recent event ring buffer in a resource. Expose:
- `combat log --lines <n>`
- `combat recap --target current`

Keep storage bounded and text-first. Do not build a meter parser in v1.

**Step 4: Run test to verify it passes**

Run: `cargo test combat_log`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs src/status.rs src/networking.rs
git commit -m "feat: add combat log CLI snapshots"
```

### Task 7: Reputation, collections, and profession reports

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Modify: `src/status.rs`
- Modify: `src/item_info.rs`
- Test: `src/status.rs`
- Test: `src/bin/game-engine-cli.rs`

**Step 1: Write the failing test**

Add tests for:
- `reputation list`
- `collection mounts --missing`
- `profession recipes --text potion`

Each test should assert specific rendered rows, not only field presence.

**Step 2: Run test to verify it fails**

Run: `cargo test reputation_list`
Expected: FAIL because the richer report requests do not exist.

**Step 3: Write minimal implementation**

Start read-only:
- `reputation list`
- `collection mounts`
- `collection pets`
- `profession recipes --text <term>`

Reuse existing item naming and lookup helpers where possible. Do not add recipe crafting or summon/mount actions yet.

**Step 4: Run test to verify it passes**

Run: `cargo test reputation_list`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs src/status.rs src/item_info.rs
git commit -m "feat: add progression and profession CLI reports"
```

### Task 8: Map and waypoint utilities

**Files:**
- Modify: `src/bin/game-engine-cli.rs`
- Modify: `src/ipc/mod.rs`
- Modify: `src/ipc/plugin.rs`
- Modify: `src/minimap.rs`
- Modify: `src/target.rs`
- Modify: `src/status.rs`
- Test: `src/bin/game-engine-cli.rs`
- Test: `src/status.rs`

**Step 1: Write the failing test**

Add tests for:
- `map target`
- `map waypoint add --x 42.1 --y 65.7`
- target distance formatter when no target exists

**Step 2: Run test to verify it fails**

Run: `cargo test map_target`
Expected: FAIL because map request and waypoint snapshot types do not exist.

**Step 3: Write minimal implementation**

Expose:
- `map position`
- `map target`
- `map waypoint add --x <x> --y <y>`
- `map waypoint clear`

Only include local-zone coordinates in v1.

**Step 4: Run test to verify it passes**

Run: `cargo test map_target`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/game-engine-cli.rs src/ipc/mod.rs src/ipc/plugin.rs src/minimap.rs src/target.rs src/status.rs
git commit -m "feat: add map and waypoint CLI utilities"
```

## Verification Checklist

Run these after each task, not only at the end:

```bash
cargo fmt
cargo test <targeted-test-name>
```

Run this before declaring the whole feature set complete:

```bash
./run-tests.sh
```

Manual checks:
- start the engine and verify `game-engine-cli ping` still works
- verify a placeholder command returns a useful error instead of panicking
- verify `spell cast --spell 133 --target current` errors clearly when no target is selected
- verify `spell cast` uses the same target as the on-screen selection circle
- verify read-only reports remain usable with no server connection when backed by local snapshots

## Recommended Delivery Order

1. Scaffold command families and placeholder IPC types.
2. Ship inventory inspection because bag and storage data already exist locally.
3. Ship `cast spell on target` early because it establishes the write-intent pattern beyond AH and mail.
4. Add quests and group once replicated snapshots exist in shared protocol.
5. Add combat log, then progression reports, then map helpers.

## Notes for the implementer

- Prefer new focused snapshot structs in `src/status.rs` over overloading `NetworkStatusSnapshot`.
- Keep `src/ipc/plugin.rs` as a dispatcher and formatter layer, not a state owner.
- If spell name resolution is not available locally, accept numeric spell IDs first and add name resolution in a follow-up.
- Use the same fail-fast style already present in `src/bin/game-engine-cli.rs` and `src/ipc/plugin.rs`.
- Do not invent client-authoritative spell success. The CLI should report submission or server rejection, never pretend the cast landed locally.
