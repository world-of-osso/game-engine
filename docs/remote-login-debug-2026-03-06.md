# Remote Login Debug Update (2026-03-06)

## Summary

Remote login timeouts are not caused by network reachability.

Packet capture confirmed that:

- the local client sends UDP packets to `104.131.165.138:5000`
- `sakuin` receives those packets
- `sakuin` sends UDP replies
- the replies arrive back on the local machine

DigitalOcean and router checks also did not show a network block:

- DigitalOcean firewall for `sakuin` allows inbound `udp/5000`
- UniFi did not show an active WAN/LAN rule blocking this path

## Current Failure Point

The failure happens after the server accepts the netcode connection.

Observed on `sakuin` via `journalctl`:

- `Received UDP packet from new address ...`
- `New connection on netcode from ...`
- immediate panic in `lightyear_replication`:
  - `src/send/components.rs:1130`
  - `not yet implemented`

The client then times out in `Connecting` because `game-server.service` exits and restarts under systemd.

## Working Theory

This is currently a server-side replication bug, not a firewall or routing problem.

The relevant symptom is that Lightyear reaches a `ReplicationMode::SingleSender` path that is not expected by the current server setup.

## Next Step

Add targeted diagnostics in `game-server` around replication setup on connect:

- log `Replicate` insertions that resolve to `SingleSender`
- log connection entities with `Connected`, `ClientOf`, `LinkOf`, and `ReplicationSender`
- identify which entity is entering the unsupported replication mode
