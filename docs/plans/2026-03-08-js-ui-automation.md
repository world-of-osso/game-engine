# JavaScript UI Automation Implementation Plan

**Goal:** Add a scriptable in-engine UI automation system that drives the same login/edit-box/button paths as a user, so login and other UI flows can be debugged and verified by actually performing the actions.

**Architecture:** Build a small Rust automation core first, centered on a queue of high-level UI actions that resolve frame names, drive pointer/keyboard helpers, and wait on observable conditions such as game state changes or frame text updates. Then embed a JavaScript runtime that exposes that automation core as a minimal host API, and add a startup script runner so ad-hoc debug scripts can log in, wait for transitions, and trigger dumps without adding one-off Rust code paths.

**Tech Stack:** Rust, Bevy 0.18 ECS/state, existing `login_screen.rs` and UI frame registry/input helpers, embedded JavaScript runtime (recommended: QuickJS via `rquickjs` or `quick-js`), existing dump hooks in `src/main.rs`

---

### Task 1: Define the Rust automation core and prove it can drive the login UI without scripting

**Files:**
- Create: `src/ui/automation.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/main.rs`
- Modify: `src/login_screen.rs`
- Test: `src/ui/automation.rs` unit tests
- Test: `src/login_screen.rs` unit tests

**Step 1: Write the failing tests**

Add focused tests in `src/ui/automation.rs` and `src/login_screen.rs` that prove:

```rust
#[test]
fn automation_click_focuses_username_editbox() {
    let mut world = setup_login_world();
    queue_action(&mut world, UiAutomationAction::ClickFrame("UsernameInput".into()));
    run_automation_tick(&mut world);
    assert_eq!(world.resource::<LoginFocus>().0, Some(username_id(&world)));
}

#[test]
fn automation_type_uses_login_editbox_code_path() {
    let mut world = setup_login_world();
    queue_actions(
        &mut world,
        vec![
            UiAutomationAction::ClickFrame("UsernameInput".into()),
            UiAutomationAction::TypeText("alice".into()),
        ],
    );
    run_automation_until_idle(&mut world);
    assert_eq!(get_username_text(&world), "alice");
}

#[test]
fn automation_login_reaches_connecting_state() {
    let mut world = setup_login_world();
    queue_actions(
        &mut world,
        vec![
            UiAutomationAction::ClickFrame("UsernameInput".into()),
            UiAutomationAction::TypeText("alice".into()),
            UiAutomationAction::ClickFrame("PasswordInput".into()),
            UiAutomationAction::TypeText("secret".into()),
            UiAutomationAction::ClickFrame("ConnectButton".into()),
        ],
    );
    run_automation_until_idle(&mut world);
    assert!(matches!(
        *world.resource::<NextState<GameState>>(),
        NextState::Pending(GameState::Connecting)
    ));
}
```

**Step 2: Run the tests to verify they fail**

Run: `cargo test automation_login_reaches_connecting_state automation_type_uses_login_editbox_code_path automation_click_focuses_username_editbox --bin game-engine`

Expected: FAIL because the automation resource/plugin/action executor does not exist yet.

**Step 3: Write the minimal automation implementation**

Create `src/ui/automation.rs` with:
- `UiAutomationPlugin`
- `UiAutomationQueue(VecDeque<UiAutomationAction>)`
- `UiAutomationAction`
  - `ClickFrame(String)`
  - `TypeText(String)`
  - `PressKey(KeyCode)`
  - `WaitForState(GameState, f32)`
  - `DumpTree`
  - `DumpUiTree`
- systems to process one action at a time
- frame lookup by frame name using `FrameRegistry`

Important implementation rule:
- for login interactions, call the same internal helpers already used by real input handling
- extract shared helpers from `src/login_screen.rs` if needed, but do not set edit box text directly in the automation path

Example shape:

```rust
pub enum UiAutomationAction {
    ClickFrame(String),
    TypeText(String),
    PressKey(KeyCode),
    WaitForState(GameState, f32),
    DumpTree,
    DumpUiTree,
}

fn process_click_frame(
    ui: &mut UiState,
    login_ui: Option<&LoginUi>,
    action: &str,
) -> Result<(), String> {
    let frame_id = find_frame_by_name(&ui.registry, action)
        .ok_or_else(|| format!("frame not found: {action}"))?;
    let center = frame_center(&ui.registry, frame_id)?;
    handle_mouse_click(...same login helper path..., center, ...);
    Ok(())
}
```

**Step 4: Run the tests to verify they pass**

Run: `cargo test automation_login_reaches_connecting_state automation_type_uses_login_editbox_code_path automation_click_focuses_username_editbox --bin game-engine`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/automation.rs src/ui/mod.rs src/main.rs src/login_screen.rs
git commit -m "Add UI automation core for login flow"
```

### Task 2: Add observable waits and deterministic dump actions

**Files:**
- Modify: `src/ui/automation.rs`
- Modify: `src/main.rs`
- Test: `src/ui/automation.rs`

**Step 1: Write the failing tests**

Add tests for waiting and dump action completion:

```rust
#[test]
fn wait_for_state_blocks_until_target_state() {
    let mut world = setup_login_world();
    queue_actions(
        &mut world,
        vec![
            UiAutomationAction::WaitForState(GameState::CharSelect, 1.0),
        ],
    );
    tick_automation(&mut world, 0.1);
    assert!(automation_is_waiting(&world));
    world.resource_mut::<NextState<GameState>>().set(GameState::CharSelect);
    apply_state_transition_for_test(&mut world);
    tick_automation(&mut world, 0.1);
    assert!(automation_is_idle(&world));
}

#[test]
fn dump_tree_action_sets_existing_dump_flag() {
    let mut world = setup_login_world();
    queue_action(&mut world, UiAutomationAction::DumpTree);
    run_automation_tick(&mut world);
    assert!(world.contains_resource::<DumpTreeFlag>());
}
```

**Step 2: Run the tests to verify they fail**

Run: `cargo test wait_for_state_blocks_until_target_state dump_tree_action_sets_existing_dump_flag --bin game-engine`

Expected: FAIL because waits and dump actions are not implemented.

**Step 3: Write the minimal implementation**

Extend automation execution with:
- wait state stored in an `AutomationRunner` resource
- per-frame timeout countdown
- dump actions that reuse existing debug flags/resources from `src/main.rs`

Add minimal diagnostics:

```rust
#[derive(Resource, Default)]
pub struct AutomationRunner {
    pub waiting: Option<AutomationWait>,
    pub last_error: Option<String>,
    pub completed: bool,
}
```

**Step 4: Run the tests to verify they pass**

Run: `cargo test wait_for_state_blocks_until_target_state dump_tree_action_sets_existing_dump_flag --bin game-engine`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/automation.rs src/main.rs
git commit -m "Add automation waits and dump actions"
```

### Task 3: Add a startup automation runner from a structured script file

**Files:**
- Create: `src/ui/automation_script.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/main.rs`
- Test: `src/ui/automation_script.rs`
- Test: `src/main.rs`

**Step 1: Write the failing tests**

Add tests for parsing and scheduling a startup script:

```rust
#[test]
fn parse_json_script_into_actions() {
    let script = r#"
    [
      {"click":"UsernameInput"},
      {"type":"alice"},
      {"click":"PasswordInput"},
      {"type":"secret"},
      {"click":"ConnectButton"},
      {"wait_for_state":"Connecting","timeout_secs":1.0},
      {"dump_tree":true}
    ]
    "#;
    let actions = parse_automation_script(script).unwrap();
    assert_eq!(actions.len(), 7);
}

#[test]
fn startup_flag_loads_script_path_into_queue() {
    let args = vec!["--run-ui-script".to_string(), "debug/login.json".to_string()];
    let config = parse_automation_script_arg(&args).unwrap();
    assert_eq!(config.path, PathBuf::from("debug/login.json"));
}
```

**Step 2: Run the tests to verify they fail**

Run: `cargo test parse_json_script_into_actions startup_flag_loads_script_path_into_queue --bin game-engine`

Expected: FAIL because the parser and CLI flag do not exist.

**Step 3: Write the minimal implementation**

Before JavaScript, add a structured startup path so the automation core can already be used:
- `--run-ui-script <path>`
- load a simple JSON or RON file into `Vec<UiAutomationAction>`
- enqueue actions at startup

Example supported format:

```json
[
  {"click":"UsernameInput"},
  {"type":"alice"},
  {"click":"PasswordInput"},
  {"type":"secret"},
  {"click":"ConnectButton"},
  {"wait_for_state":"CharSelect","timeout_secs":5.0},
  {"dump_tree":true}
]
```

This phase is intentionally not JavaScript yet; it de-risks the automation core and gives immediate end-to-end proof.

**Step 4: Run the tests to verify they pass**

Run: `cargo test parse_json_script_into_actions startup_flag_loads_script_path_into_queue --bin game-engine`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/automation_script.rs src/ui/mod.rs src/main.rs
git commit -m "Add startup UI automation script runner"
```

### Task 4: Embed JavaScript and expose the automation API

**Files:**
- Create: `src/ui/js_automation.rs`
- Modify: `Cargo.toml`
- Modify: `src/ui/mod.rs`
- Modify: `src/main.rs`
- Test: `src/ui/js_automation.rs`

**Step 1: Write the failing tests**

Add tests that execute a tiny JS script and verify actions are emitted:

```rust
#[test]
fn js_click_and_type_emit_automation_actions() {
    let script = r#"
        ui.click("UsernameInput");
        ui.type("alice");
        ui.click("PasswordInput");
        ui.type("secret");
    "#;
    let actions = run_js_to_actions(script).unwrap();
    assert_eq!(
        actions,
        vec![
            UiAutomationAction::ClickFrame("UsernameInput".into()),
            UiAutomationAction::TypeText("alice".into()),
            UiAutomationAction::ClickFrame("PasswordInput".into()),
            UiAutomationAction::TypeText("secret".into()),
        ]
    );
}

#[test]
fn js_wait_for_state_and_dump_emit_actions() {
    let script = r#"
        ui.waitForState("CharSelect", 5.0);
        ui.dumpTree();
    "#;
    let actions = run_js_to_actions(script).unwrap();
    assert_eq!(actions.len(), 2);
}
```

**Step 2: Run the tests to verify they fail**

Run: `cargo test js_click_and_type_emit_automation_actions js_wait_for_state_and_dump_emit_actions --bin game-engine`

Expected: FAIL because the JS runtime is not integrated.

**Step 3: Write the minimal implementation**

Add one JS runtime dependency. Recommended:
- `rquickjs` for a lightweight embedded engine with explicit host bindings

Expose a minimal global API:

```javascript
ui.click("UsernameInput");
ui.type("alice");
ui.key("Enter");
ui.waitForState("CharSelect", 5.0);
ui.dumpTree();
ui.dumpUiTree();
```

Implementation rule:
- JS should enqueue actions only
- actual execution stays in the Rust automation core
- keep the API small until real use cases require more

**Step 4: Run the tests to verify they pass**

Run: `cargo test js_click_and_type_emit_automation_actions js_wait_for_state_and_dump_emit_actions --bin game-engine`

Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml src/ui/js_automation.rs src/ui/mod.rs src/main.rs
git commit -m "Add JavaScript UI automation runtime"
```

### Task 5: Add a real login debug script and an end-to-end verification path

**Files:**
- Create: `debug/login.js`
- Modify: `src/main.rs`
- Modify: `src/login_screen.rs`
- Test: `src/main.rs` integration-style tests where practical
- Verify: manual scripted run against a real server

**Step 1: Write the failing test**

Add a test for CLI parsing of the JS automation entrypoint:

```rust
#[test]
fn parse_js_automation_flag() {
    let args = vec![
        "--state".to_string(),
        "login".to_string(),
        "--run-js-ui-script".to_string(),
        "debug/login.js".to_string(),
    ];
    let parsed = parse_js_automation_arg(&args).unwrap();
    assert_eq!(parsed, PathBuf::from("debug/login.js"));
}
```

**Step 2: Run the test to verify it fails**

Run: `cargo test parse_js_automation_flag --bin game-engine`

Expected: FAIL because the JS automation startup flag does not exist.

**Step 3: Write the minimal implementation**

Add:
- `--run-js-ui-script <path>`
- load and execute `debug/login.js`
- enqueue actions before the first login tick

Create `debug/login.js`:

```javascript
ui.waitForFrame("UsernameInput", 5.0);
ui.click("UsernameInput");
ui.type(env.LOGIN_USER);
ui.click("PasswordInput");
ui.type(env.LOGIN_PASS);
ui.click("ConnectButton");
ui.waitForState("CharSelect", 10.0);
ui.dumpTree();
```

Support minimal environment variables:
- `LOGIN_USER`
- `LOGIN_PASS`

**Step 4: Run automated and manual verification**

Run:
- `cargo test parse_js_automation_flag --bin game-engine`
- `cargo test login_screen --bin game-engine`
- `cargo test networking_auth --bin game-engine`

Manual verification command:

```bash
LOGIN_USER=alice LOGIN_PASS=secret cargo run --bin game-engine -- --server 127.0.0.1:5000 --state login --run-js-ui-script debug/login.js
```

Expected:
- login UI visibly receives typed text
- login button is activated through the normal UI path
- app reaches `CharSelect`
- tree dump output is produced

**Step 5: Commit**

```bash
git add debug/login.js src/main.rs src/login_screen.rs
git commit -m "Add scripted login verification via JavaScript"
```

### Task 6: Harden the debugging workflow and document it

**Files:**
- Create: `docs/ui-automation-debugging.md`
- Modify: `AGENTS.md`
- Test: manual workflow notes

**Step 1: Write the failing verification checklist**

Create a checklist in the doc for:
- login script against local server
- failed login script
- reconnect script
- dump-tree after `CharSelect`
- dump-ui-tree while still on login screen

**Step 2: Verify the documentation points to missing commands**

Run the documented commands manually and correct any mismatch.

Expected: At least one command or path will need adjustment while the feature settles.

**Step 3: Write the minimal documentation**

Document:
- how to run a JS automation script
- available `ui.*` APIs
- how scripts are resolved
- which actions use the real UI path
- known limitations

Include example scripts:

```javascript
ui.click("MenuButton");
ui.waitForText("LoginStatus", "Menu is not implemented yet", 1.0);
ui.dumpUiTree();
```

**Step 4: Verify the documented workflow**

Run:

```bash
cargo run --bin game-engine -- --state login --run-js-ui-script debug/login.js
```

Expected: workflow matches the documentation exactly.

**Step 5: Commit**

```bash
git add docs/ui-automation-debugging.md AGENTS.md
git commit -m "Document JavaScript UI automation debugging"
```
