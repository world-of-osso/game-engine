# Authentication

The game server requires username/password authentication. Passwords are hashed with argon2 before storage.

## Register a new account

On the login screen, click "Don't have an account? Register" to switch to register mode. Enter a username and password, then click Register. The server creates the account and logs you in automatically.

## Login

Enter your username and password, then click Login. On success, an auth token is saved to `data/auth_token` on the client. Subsequent logins use the cached token automatically (password not required).

Delete `data/auth_token` to force password re-entry.

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

Client-side:
- `data/auth_token`: cached session token (plaintext file)

## Security notes

- Passwords are transmitted in plaintext over UDP (lightyear netcode has no encryption). Acceptable for LAN/dev use only.
- Argon2 with random salt is used for password hashing server-side.
- Tokens are UUID v4 strings generated per-account.
