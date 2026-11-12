# Friends/Who Snapshot Delivery

The Friends and Who server handlers produced snapshots, but the native client did not receive them. The root cause was the explicit receive list in `NetworkBridge::connect`.

## Content

### Symptom

VERIFIED: On the private UDP 5998 run, the client submitted Who queries but its Friends/Who status snapshots stayed empty. The targeted Who and friend-presence server tests passed.

### Root cause

VERIFIED: `NetworkBridge::connect` registered typed receive relays for protocol messages but omitted `FriendsStateUpdate` and `WhoStateUpdate`.

INFERRED: Without those registrations, the bridge could not relay these snapshots to the account handlers.

### Fix and regression proof

VERIFIED: The bridge now registers both message types. `native_default_bridge_receives_friends_and_who_updates_over_real_udp` passes 1/1 and asserts the exact Friends and Who snapshots arrive as client message events.

VERIFIED: After the fix, the two-client private run showed Friendalpha in Friendbeta's Friends snapshot as Online and in the Who result for `Friendalpha`. Selecting the friend row opened `/w Friendbeta `, and the recipient's chat showed the delivered whisper.

VERIFIED: The group invite popup named Friendalpha; Friendbeta accepted it. The leader then used Convert to Raid. Both clients reported `in_group: true`, `is_raid: true`, and `members: 2`.

### Visual and scope limits

VERIFIED: Who feedback was moved below the results and above the search box. The both-skin layout test passes 1/1. Reviewed Friends, Who and converted-Raid captures and 3× nearest-neighbour crops are in `/syncthing/AgentShared/2026-11-11/friends-panel/`.

VERIFIED: With no saved instances, Raid Info is disabled; Retail `RaidFrame.lua` also disables the button when the saved-instance and world-boss counts are both zero. The Raid Info window was not opened because the disposable characters had no saved-instance data.

UNVERIFIED: AFK/DND and offline friend rows were not exercised. No saved-instance fixture was used to test opening Raid Info.

## Sources

- [Network bridge](../../../godot/network/src/lib.rs) — explicit receive registrations.
- [Real-UDP regression test](../../../godot/network/src/friends_who_wire_tests.rs) — exact message-delivery behavior.
- [Account snapshot handlers](../../../godot/rust/src/account.rs) — Friends/Who state application.
- [FriendsFrame spec](../../specs/friends-frame.md) — remaining feature contract.
- Retail RaidFrame.lua (`/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_RaidFrame/Mainline/RaidFrame.lua`) — saved-instance button condition.

## See Also

- [[friends-frame]] — current UI structure, geometry and behavior.
- [[ui-system]] — shared UI lifecycle.
