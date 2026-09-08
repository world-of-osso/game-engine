# Authentication

The game server requires username/password authentication. Passwords are hashed with argon2 before storage.

## Register a new account

On the login screen, click "Don't have an account? Register" to switch to register mode. Enter a username and password, then click Register. The server creates the account and logs you in automatically.

## Login

Enter your username and password, then click Login. On success, the client saves a cached token using the [server-specific path](#storage). Subsequent token logins do not require the password.

Delete the relevant server's token file to force password re-entry.

## How it works

1. **Register**: Client sends `RegisterRequest { username, password }`. Server hashes the password with argon2, stores the hash in the `PASSWORDS` table, creates an account, and returns a session token.
2. **Login (password)**: Client sends `LoginRequest { token: None, username, password }`. Server looks up the account by username, verifies the password against the stored argon2 hash, and returns a session token.
3. **Login (token)**: Client sends `LoginRequest { token: Some(cached), username, password: "" }`. Server validates the token directly — no password check.

## Storage

Server-side (redb tables):
- `ACCOUNTS`: token (string) -> account_id (u64)
- `USERNAME_ACCOUNTS`: username (string) -> account_id (u64)
- `PASSWORDS`: account_id (u64) -> argon2 hash (bytes)
- `ACCOUNT_CHARACTERS`: account_id (u64) -> character_id list

Client-side engine tokens are plaintext files under the build's `CARGO_MANIFEST_DIR/data`:
- `worldofosso.com` and its subdomains share `auth_token`.
- Other server strings use `auth_token.<server>`, replacing `:` with `_` and `/` with `_`; for example, `auth_token.127.0.0.1_5000`.
- `XDG_CONFIG_HOME` does not relocate engine tokens. The headless `game-cli` uses a separate config-directory token cache.

Initial login/registration is queued when the network worker is created, rather than waiting for main-thread connection handling. Lightyear buffers it until connected. Responses, token persistence, UI transitions, and world mutation remain main-thread work; sender readiness is prepared before dispatching responses so immediate character selection is not dropped.

## Security notes

- Passwords are transmitted in plaintext over UDP (lightyear netcode has no encryption). Acceptable for LAN/dev use only.
- Argon2 with random salt is used for password hashing server-side.
- Tokens are UUID v4 strings generated per-account.
