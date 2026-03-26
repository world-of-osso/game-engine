# Inworld Loading Screen Implementation Plan

**Goal:** Add a WoW-style loading screen for `cargo run --bin game-engine -- --server dev --screen inworld` that stays visible until the client has enough replicated world state and terrain data to enter the world cleanly.

**Architecture:** Keep the existing `GameState::Loading` transition, but stop treating it as a pass-through state. Add a dedicated loading-screen UI plugin and component, start the minimum network and terrain preparation work while in `Loading`, and only transition to `InWorld` once the local player and initial terrain tile are ready. The screen should reuse the existing UI toolkit `Screen` pattern so it fits the current login and char-select implementations.

**Tech Stack:** Rust, Bevy 0.18, existing `ui_toolkit::screen::Screen` UI system, Lightyear networking, local WoW BLP loading via existing UI texture support.

---

### Task 1: Define the loading completion contract

**Files:**
- Modify: `src/game_state.rs`
- Test: `src/game_state.rs`

**Step 1: Write failing unit tests for loading readiness**

Add tests that cover:

```rust
#[test]
fn loading_waits_for_local_player_before_progressing() {}

#[test]
fn loading_waits_for_initial_terrain_tile_before_progressing() {}

#[test]
fn loading_completes_when_local_player_and_initial_tile_are_ready() {}
```

The tests should exercise a new pure helper that accepts:
- whether the local player is ready
- whether terrain streaming has a map name
- whether the initial tile is pending
- whether the initial tile is loaded

Expected behavior:
- no local player => incomplete
- local player but no terrain seed => incomplete
- local player and seeded terrain but tile not loaded => incomplete
- local player and initial tile loaded => complete

**Step 2: Run the focused test target to verify failure**

Run:

```bash
cargo test game_state::tests::loading_
```

Expected: FAIL because the new helper/tests do not exist yet.

**Step 3: Implement the minimal readiness helper**

In `src/game_state.rs`:
- add a small `LoadingReadiness` struct or equivalent pure helper return value
- add `evaluate_world_loading(...)`
- make it return:
  - progress text for UI
  - progress percentage for UI
  - `complete` bool for the state transition

Keep the logic narrow:
- do not try to estimate every asset
- gate on the local player existing
- gate on terrain being seeded
- gate on the initial tile being loaded

**Step 4: Replace the placeholder loading transition**

Change `check_loading_complete` so it:
- queries the local player readiness
- reads `AdtManager`
- transitions to `GameState::InWorld` only when `evaluate_world_loading(...).complete` is true

**Step 5: Run the focused test target again**

Run:

```bash
cargo test game_state::tests::loading_
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/game_state.rs
git commit -m "feat: gate loading state on world readiness"
```

### Task 2: Build the WoW-style loading screen UI

**Files:**
- Create: `src/loading_screen.rs`
- Create: `src/ui/screens/loading_component.rs`
- Modify: `src/ui/screens/mod.rs`
- Modify: `src/main.rs`
- Test: `src/loading_screen.rs`

**Step 1: Write a failing UI-oriented test**

Add a small test that builds the loading screen into a `FrameRegistry` and asserts the key frames exist:

```rust
#[test]
fn loading_screen_builds_expected_frames() {
    assert!(registry.get_by_name("LoadingRoot").is_some());
    assert!(registry.get_by_name("LoadingBarFill").is_some());
    assert!(registry.get_by_name("LoadingStatusText").is_some());
}
```

**Step 2: Run the focused test target to verify failure**

Run:

```bash
cargo test loading_screen::tests::loading_screen_builds_expected_frames
```

Expected: FAIL because the plugin/component do not exist yet.

**Step 3: Implement the loading screen component**

Create `src/ui/screens/loading_component.rs` with:
- a `LoadingScreenState` context struct
- a `loading_screen(ctx: &SharedContext) -> Element`
- fixed frame names for:
  - `LoadingRoot`
  - `LoadingArtwork`
  - `LoadingBarBackground`
  - `LoadingBarFill`
  - `LoadingBarFrame`
  - `LoadingStatusText`
  - `LoadingProgressText`
  - `LoadingTipText`

Visual direction:
- full black background
- centered loading artwork using a real WoW loading screen BLP from the synced WoW install
- world-of-osso logo at the top
- actual WoW loading bar frame/fill assets from `Interface/GLUES/LoadingBar`
- gold Friz-style text
- a short tip under the bar

**Step 4: Implement the loading screen plugin**

Create `src/loading_screen.rs` modeled after `src/login_screen.rs` / `src/char_select.rs`:
- `OnEnter(GameState::Loading)` builds the screen
- `Update` syncs root size and loading progress visuals
- `OnExit(GameState::Loading)` tears it down

The plugin should:
- derive its display state from `evaluate_world_loading(...)`
- resize the fill frame width from progress percent
- update screen text through `SharedContext`

**Step 5: Register the plugin**

Modify:
- `src/ui/screens/mod.rs` to export `loading_component`
- `src/main.rs` to include `mod loading_screen;`
- `src/main.rs` to add `loading_screen::LoadingScreenPlugin`

**Step 6: Run the focused UI test again**

Run:

```bash
cargo test loading_screen::tests::loading_screen_builds_expected_frames
```

Expected: PASS

**Step 7: Commit**

```bash
git add src/loading_screen.rs src/ui/screens/loading_component.rs src/ui/screens/mod.rs src/main.rs
git commit -m "feat: add wow-style loading screen UI"
```

### Task 3: Start world preparation work during `Loading`

**Files:**
- Modify: `src/terrain.rs`
- Modify: `src/networking.rs`
- Test: `src/game_state.rs`

**Step 1: Write a failing behavior test around readiness inputs**

Add or extend a test that proves loading cannot complete if terrain streaming work has not started yet. The helper-level version is enough:

```rust
#[test]
fn loading_stays_incomplete_while_initial_tile_is_only_pending() {}
```

Expected logic:
- local player ready
- terrain seeded
- initial tile pending but not loaded
- result is incomplete

**Step 2: Run the focused test target to verify failure**

Run:

```bash
cargo test game_state::tests::loading_stays_incomplete_while_initial_tile_is_only_pending
```

Expected: FAIL until the helper and surrounding logic are aligned.

**Step 3: Allow terrain streaming to run in `Loading`**

Modify `src/terrain.rs` so the ADT streaming system set runs in:
- `GameState::Loading`
- `GameState::InWorld`

This should include:
- bootstrap terrain streaming
- tile load dispatch
- loaded tile receive/spawn
- doodad LOD swapping if already grouped in the same system chain

**Step 4: Allow local-player tagging to run in `Loading`**

Modify `src/networking.rs` so the systems that tag the local player and derive local-world readiness also run in:
- `GameState::Loading`
- `GameState::InWorld`

Keep chat/input/combat systems restricted to `InWorld`.

**Step 5: Run the focused test target again**

Run:

```bash
cargo test game_state::tests::loading_stays_incomplete_while_initial_tile_is_only_pending
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/terrain.rs src/networking.rs src/game_state.rs
git commit -m "feat: prepare world data during loading state"
```

### Task 4: Verify the full inworld flow manually

**Files:**
- No new code expected unless bugs are found

**Step 1: Run formatting**

Run:

```bash
cargo fmt
```

Expected: no errors

**Step 2: Run targeted tests**

Run:

```bash
cargo test game_state::tests::loading_
cargo test loading_screen::tests::loading_screen_builds_expected_frames
```

Expected: PASS

**Step 3: Run the real client flow**

Run:

```bash
RUST_BACKTRACE=1 cargo run --bin game-engine -- --server dev --screen inworld
```

Expected:
- app enters `Loading`
- a full-screen WoW-style loading screen is visible
- the bar/status advances while the world prepares
- the app transitions to `InWorld` only after the initial world is actually usable

**Step 4: Sanity-check failure cases**

While running the client, verify:
- disconnect during loading still returns to login cleanly
- loading does not flash away instantly before terrain arrives
- loading does not remain visible after the initial tile is present and the local player is tagged

**Step 5: Optional cleanup commit if needed**

```bash
git add .
git commit -m "test: verify inworld loading screen flow"
```
