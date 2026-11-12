# Friends Frame

The native `FriendsFrame` is the Retail-style social window hosted by the Godot client. It presents Friends, Who and Raid views; the shared UI model builds the rows, and the native host forwards actions to the account and group requests.

## Content

### State and message flow

- The account stores Friends and Who snapshots. The UI model projects their entries into rows.
- `NetworkBridge::connect` uses explicit receive registrations. It must include `FriendsStateUpdate` and `WhoStateUpdate`; otherwise the native account does not receive those snapshots.
- The host synchronizes FriendsFrame edit-box text before it dispatches Add Friend or Who actions.
- Group members and raid state come from the existing group snapshot. Convert to Raid uses the existing group request.

### Retail geometry and current behavior

- The frame is 385×424. Friends rows are 298×34. Who rows are 298 px inside a 302 px scroll box, with 28 px column headers.
- Who feedback sits after the result list and before the search box. The both-skin component test checks that it does not cover the headers or search box.
- The Raid Info button is disabled when there are no saved instances. The fresh disposable characters had no saved-instance data, so the Raid Info window did not open; this matches Retail's button-enable condition.

VERIFIED: The real-UDP bridge test receives both Friends and Who snapshots. In the private two-client run, the friend row and Who result appeared, a row-selected whisper reached the other client, and both clients reported a two-member raid after conversion.

UNVERIFIED: This live run did not exercise AFK/DND or offline friend rows, or open Raid Info with a saved instance.

## Sources

- [Friends Frame spec](../../specs/friends-frame.md) — feature contract and test mapping.
- [Native bridge](../../../godot/network/src/lib.rs) — explicit protocol receive registrations.
- [Friends/Who bridge test](../../../godot/network/src/friends_who_wire_tests.rs) — real-UDP snapshot delivery.
- [FriendsFrame host](../../../godot/rust/src/friends_frame.rs) — input synchronization and action dispatch.
- [FriendsFrame component](../../../godot/ui-model/src/ui/screens/friends_frame_component.rs) — Retail panel layout.
- Retail FriendsFrame XML (`/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_FriendsFrame/Mainline/FriendsFrame.xml`) — source geometry.

## See Also

- [[friends-who-snapshots]] — root cause and fix for missing snapshots.
- [[ui-system]] — shared screen and frame lifecycle.
