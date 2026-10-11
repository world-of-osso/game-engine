# Group Frames

Native party and raid play share `GroupState` in `godot/ui-model/src/game/group_state.rs`. The server fills the roster, live member states, invites, ready checks and pull countdowns; native frames, menus and IPC read that state. [Acceptance and remaining gaps](../../specs/group-frames.md#native-countdown-and-ready-check-acceptance--2026-10-11) live in the contract.

## Content

### Native ready checks and pull countdowns

Verified: 2026-10-11.

- `godot/ui-model/src/countdown_component.rs` projects Retail TimerTracker stages; `group_countdown.rs` owns remaining time, number transitions and sound requests. `godot/rust/src/party_frames.rs` projects the group/manager/ready-check controls; both skins share elements and behavior.
- Countdown updates carry a server UTC deadline. Applying receipt-relative duration revived reliable messages delivered up to18s late in the first three-client run; the authoritative deadline rejects expired starts. Client/server wall clocks must be synchronized.
- Server group authority validates leader/raid-assistant initiation, replaces/cancels timers and clears them on leave. Assistant changes arrive in authoritative roster snapshots.
- Ready-check completion names only unanswered members as AFK and emits the unanswered-recipient message; explicit Not Ready responses are not AFK.
- Countdown sound-kit requests use native playback. Inspect stream file IDs rather than sound-kit IDs: the tick/finish observations used files567474/567438, corresponding to kits158565/158566. Audio driver capability limits are recorded with the [acceptance evidence](../../specs/group-frames.md#native-countdown-and-ready-check-acceptance--2026-10-11).

### Data flow

- **Server** (`game-server crates/server/src/group/`):
  - `GroupRegistry` is pure state keyed by character id and returns `GroupEffect`s. `mod.rs` turns those into messages.
  - `member_state.rs` sends `GroupMemberStates` every 0.2 s: one diff per connection against what that connection was last sent. A new connection (login or relog) starts empty, so it receives everything.
- **Why member states instead of replication:** interest is a flat 100 yd radius (`interest.rs`), and party frames must show members beyond it.
  - Frames therefore never read replicated `Health`/`UnitPowers`.
  - The replicated entity is used only for click-targeting, found by player name.
- **Range fading:** compares the local player's state position with the member's (server coordinates on both sides), 40 yd.
- **Stale invites:** the registry cancels a pending invite at the source whenever the inviter joins another group, leaves, loses leadership, logs out, or the invite times out. Each cancel sends `GroupInviteCancelled` to the invitee.

### Historical Bevy client pieces (retired)

| Piece | File |
|---|---|
| Member frame (Retail CUF Legacy layout, 12.1 atlas rects) | `src/ui/screens/compact_unit_frame_component.rs` |
| Party column, raid grid, menu | `src/ui/screens/group_frames_component.rs` |
| Ready check dialog | `src/ui/screens/ready_check_frame_component.rs` |
| Views, clicks, invite popup | `src/scenes/group_frames/mod.rs` |
| Group entries in the PlayerFrame/TargetFrame menu | `src/rendering/ui/unit_frames.rs` |
| Menu entries per role (`group_menu_entries`) | `GroupState` module; both menus use it |

### Gotchas

- Party mode sorts the player first (`CRFSort_Group`). Raid mode draws group `n` in column `n` and skips empty groups, so column positions stay put when members move.
- Before a member's first state arrives, the frame bars are empty on purpose: no placeholder 100/100.
- Ghosts show "Dead", as Retail does (`UnitIsDeadOrGhost`).

## Sources

- [group-frames spec](../../specs/group-frames.md) — requirements, Retail citations, the not-implemented list

## See Also

- [[portrait-party-frames]] — unwired native portrait family and verified Camelot art relation; compact remains default
- [[ui-system]] — rsx screens, SharedContext
- [[networking]] — message routing through the network worker
