# UI Automation Debugging

Use the UI automation runner when you need proof that a UI flow actually works through the normal in-engine path.

## Entry Points

- Structured action file: `--run-ui-script <path>`
- JavaScript file: `--run-js-ui-script <path>`

Example:

```bash
LOGIN_USER=alice LOGIN_PASS=secret cargo run --bin game-engine -- --server 127.0.0.1:5000 --state login --run-js-ui-script debug/login.js
```

## JavaScript API

Available globals:

- `ui.click(name)`
- `ui.type(text)`
- `ui.key(name)`
- `ui.waitForState(name, timeoutSecs)`
- `ui.waitForFrame(name, timeoutSecs)`
- `ui.dumpTree()`
- `ui.dumpUiTree()`
- `env.NAME`

Example:

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

## What Uses The Real UI Path

The automation runner is only valid for debugging because it does not write login resources directly for text entry.

It uses the same internal login helpers for:

- focusing edit boxes by clicking them
- typing characters into edit boxes
- handling login button clicks
- triggering the connect flow

Current scope:

- login screen interactions are routed through the existing login UI code
- waits and dump actions are generic runner actions

## Debug Checklist

- Successful login script:
  `LOGIN_USER=alice LOGIN_PASS=secret cargo run --bin game-engine -- --server 127.0.0.1:5000 --state login --run-js-ui-script debug/login.js`
- Failed login script:
  Run the same command with an invalid password and confirm the state does not reach `CharSelect`
- Reconnect script:
  Add a script that clicks `ReconnectButton` after a saved `data/auth_token` exists
- Login UI tree dump:
  Use a script that stops after `ui.dumpUiTree()`
- Post-login entity tree dump:
  Use a script that waits for `CharSelect` and then calls `ui.dumpTree()`

## Known Limitations

- The current scripted input path is implemented for the login screen first
- No general pointer movement model is exposed yet
- The JavaScript layer queues actions; Rust executes them
- Unsupported key names fail during script loading
