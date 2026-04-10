# Lightyear Replication Timeout

Remote login appeared to time out during the `Connecting` phase, suggesting a network or firewall problem. Packet capture disproved that — the real failure was a server-side panic in `lightyear_replication` triggered immediately after accepting the netcode connection.

## Finding

- UDP packets reached `sakuin` on port 5000 and replies arrived back at the client. DigitalOcean firewall and UniFi rules were not blocking the path.
- `journalctl` on `sakuin` showed: connection accepted → immediate `not yet implemented` panic at `lightyear_replication/src/send/components.rs:1130`.
- `game-server.service` exited under systemd and the client timed out in `Connecting`.

## Root Cause

The server reached a `ReplicationMode::SingleSender` code path that is not yet implemented in the version of lightyear in use. This is a bug in the lightyear library, not in game-server configuration or network infrastructure.

## Resolution / Workaround

No fix landed at the time of investigation. Workaround is blocked on either upgrading lightyear past the unimplemented path or patching the replication setup to avoid `SingleSender`.

**Proposed diagnostics** (not yet applied):
- Log `Replicate` insertions that resolve to `SingleSender` on connect.
- Log connection entities with `Connected`, `ClientOf`, `LinkOf`, and `ReplicationSender` components to identify which entity triggers the unsupported mode.

## Sources

- [remote-login-debug-2026-03-06.md](../../remote-login-debug-2026-03-06.md) — packet capture results and journalctl trace

## See Also

- [[lightyear]] — networking system overview (if page exists)
