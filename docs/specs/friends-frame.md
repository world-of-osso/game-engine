# Friends Frame

The native `FriendsFrame` is the Retail-style social window. It is rendered by the UI model and hosted by the Godot client. The descriptive implementation notes are in [the Friends Frame system page](../wiki/systems/friends-frame.md).

## What it must do

### Lifecycle
- [ ] `O` (`ToggleSocial`) opens the Friends tab and toggles the frame closed; the Forever chat-frame social icon uses the same action.
- [ ] The close button and Escape close the frame.
- [ ] The panel uses the Retail 385×424 frame, portrait, inset and Friends/Who/Raid tab geometry.

### Friends
- [ ] Show concrete online, AFK/DND and offline friend states with the Retail list-row art.
- [ ] Add Friend sends the entered character name to the server and reports server errors.
- [ ] Send Message opens chat with `/w <name> ` for an online selected friend.

### Who and Raid
- [ ] Who submits the entered query and projects the server's returned character rows; Add Friend and Group Invite use the existing server requests.
- [ ] Raid reflects the current group/raid state; Convert to Raid and Raid Info use existing server actions.

## How it works

- [Friends Frame system](../wiki/systems/friends-frame.md)
- [Window stack and close lifecycle](../wiki/systems/ui-system.md)
- [Chat frame](chat-frame.md)

## Implementation inventory

- `godot/ui-model/src/friends_frame.rs` — tab/open state and friend, Who and raid row projections.
- `godot/ui-model/src/ui/screens/friends_frame_component.rs` — Retail frame, tabs, rows, controls and context menu.
- `godot/ui-model/src/selection_list.rs` — reusable Retail Friends/Who row art.
- `godot/rust/src/friends_frame.rs` — keybind, chat-button, panel and request handlers.
- `godot/rust/src/account.rs` — Friends/Who protocol state and request methods.
- `godot/rust/src/chat.rs` — chat-frame social button dispatch.
- `godot/rust/src/window_stack.rs` and `godot/ui-model/src/panel_lifecycle.rs` — Escape/close registration.
- `godot/rust/src/ipc/world.rs` — existing CLI friend and Who requests.

## Tests asserting this spec

- `godot/ui-model/src/friends_frame.rs` — open/close state and concrete Friends, Who and Raid projections.
- Private-server native proof — O toggle, chat social button, friend add, online/offline rows and whisper prefill.

## Known gaps (current cycle)

- [ ] Run targeted tests, dev/release helper builds and formatting; confirm zero Rust warnings.
- [ ] Verify the frame on a private UDP 5998 server with two throwaway accounts.
- [ ] Complete the Forever visual comparison and publish the required crops.

## Out of scope

- Quick Join, Battle.net friends, Recent Allies and Recruit-a-Friend. The requested tabs and local server APIs cover Friends, Who and Raid only.
- A Classic-only `SocialsMicroButton`. Retail Mainline opens the frame from `ToggleSocial` and the chat-frame `QuickJoinToastButton`.
