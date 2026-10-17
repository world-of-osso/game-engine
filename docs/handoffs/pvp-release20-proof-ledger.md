# Native release20 Arathi proof — 2026-10-17

**PASS, bounded lifecycle.** One rendered native client, both art skins, plus19 real UDP support accounts. Not20 rendered windows. All player processes and the private realm were release-built through the bookworm helpers. Lead accepted the result; the first cold-transfer stall remains open with server-tick-perf.

## Frozen code and artifacts

| Input | Revision |
|---|---|
| Engine | `bd3f266a1aa0be2f8d9e6c3ff3bd8b6429e917d7` |
| Server | `c7c05f628af08a97b9f8f32f17de93fe3c3fd9e6` |
| Protocol | `e9b80b8b51bb926886e6eed6099464e7cd2e9203` |

| Release artifact | SHA-256 |
|---|---|
| Native extension | `0c02a2e32aee0bb111851bd48be8db9a45bddf7865a80208421351cc7896e9e9` |
| Support peers | `a2964669e976a4e9dbae4cef39ef6798f06ba9f7476a72c03658a9d0cbaae4c3` |
| Server | `fc7172e3ed1ec29b29cb7fba2bcbe8b1ed1a704433570e366cedda9c038d544d` |
| Admin | `3b3364b1745bb6130bf5aafe0c7bc6c7b552ef38e02b438d185dcd505bb71638` |

All four builds passed without compiler warnings. `native-loaded-release.json` records the actual Godot process mappings and matching release-library hash; merely having a release file was not counted. Later documentation commits do not change these build revisions.

## Observed flow

1. Twenty actual level39 accounts formed four five-player parties through ordinary invitations and accepts. Native leader UI and three support-leader queue requests produced one Arathi1018/map2107 match,10v10, without splitting parties.
2. Normal accepts/WorldPortAck reached InProgress with20 received scoreboard rows. Five normal native object uses at authenticated banner locations incremented objective926 from1 through5. Private-admin positioning accelerated travel only.
3. Natural1500–0 victory paid each of10 winners wallet1792/lifetime XP671 and each of10 losers61. No queue, roster, resource score, timer, outcome or Honor was injected.
4. All20 received normal origin-map returns and cleared PvP match state. Native entry coordinates matched exactly after return; wallet/XP671 remained.
5. Four original1920×1080 scoreboard/end PNGs were personally inspected and placed in `/syncthing/AgentShared/2026-10-17/pvp-release20/`. Owned processes stopped, UDP5941 freed and the pvp window cleared at13:44:45 local time.

## Both attempts and event attribution

Evidence root: `data/diagnostics/bg-release-20-2026-10-17/`.

- **Attempt1 failed:** `attempt1/server-live.log` records the10.5686-second cold-transfer frame; `attempt1/peers-live.log` records peer12 ConnectionTimedOut and the support-process exit. The match remained Waiting. This failure was not relabeled success.
- **Attempt2 passed:** identical code and private database, fresh processes, normal four-party queue and full lifecycle. Raw successful logs are `server-live.log` and `peers-live.log`.
- `raw-event-order-index.json` preserves original per-file line order, logged UTC timestamps and full-log hashes. It does not fabricate cross-log ordering or absent per-peer accept/WorldPortAck send timestamps. No system-timings feature was enabled; measured frame_stats are in the server logs.

Full argv/revision/results reside in `*-release-master.json`; authoritative states in `normal20-invitation.json`, `ended-pvp.json`, `ended-peers.json`, `returned-peers.json` and `final-returned-currency.json`; objective events in `real-source-objective-events.json`; cleanup in `cleanup.json`. The machine-readable `proof-ledger.json` binds these receipts. Diagnostic data is intentionally not committed.

## Limits

Scoreboards are scrollable: captures show a visible subset of20 received rows. Existing Honor columns remain dashed; grant amounts are established by actual wallet/XP receipts and visible61-Honor gain lines. Additional wallet images remain diagnostic-only because671 is pixel-ellipsized as6...; they are not readable671 pixel proof. The nearby flag model partly obstructs the world view; no general rendering/Retail parity claim.

`client-release-import.json` preserves two ShaderFile thread-access errors, one editor popup-parent error and an8ObjectDB exit-leak warning. Do not call the import or runtime clean. The passing warm retry does not fix or certify cold-start reliability. Broader [PvP authority gaps](../specs/pvp-progression.md) remain parked.
