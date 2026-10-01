# Chat Frame

This page covers the in-world chat frame `ChatFrame1`. It has the Chattynator Dark look, with General / Combat Log / Whispers tabs, sits at BOTTOMLEFT (0, 40) and is 500x280. The contract is [chat-frame](../../specs/chat-frame.md). The Bevy client and the Godot client use the same screen, the same line formatting and the same slash parser.

## Shared pieces (root `src/`, also compiled in `godot/ui-model` under `godot_host`)

- `src/game/chat_data.rs`:
  - `ChatState`, `WhisperState`, `MAX_CHAT_MESSAGES` (100) and `MAX_RECENT_WHISPER_TARGETS` (10).
  - `runtime_chat_channel` maps a server `ChatType` to a runtime channel. An outgoing whisper keeps its recipient, for `To [Bob]:`.
  - `WhisperState::record_message`: only incoming whispers set the `/r` target.
- `src/ui/chat_frame.rs` holds the rest of the chat logic:
  - tabs and their colours, `chat_message_line` wording, `parse_chat_input` and combat log lines;
  - flash rule, scroll and wrap;
  - `tab_entries`, `tab_len`, `new_chat_messages`, `hold_scroll_position`, `add_system_line` and `local_copy_chat_text`.
- `src/ui/screens/chat_frame_component.rs`:
  - the `rsx!` screen, with Chattynator geometry cited in `file:line`;
  - `chat_frame_view`, which builds `ChatFrameView` from state and log;
  - the retail edit box header `Say: ` (ChatFrameEditBox.xml:48-53; ChatFrameEditBox.lua:674-692).
- `src/game/group_state.rs` is shared only because `ChatCommand::Group` carries a `GroupCommand`.

## Bevy

`src/scenes/chat_frame/mod.rs` runs the keyboard, clicks, wheel, flash pulse and clipboard (arboard). `src/game/networking/messages.rs` receives and sends `ChatMessage` and `EmoteIntent`.

## Godot

- **Network.** `godot/network` subscribes to `ChatMessage`. `Account` turns it into `AccountEvent::Chat`, and `send_chat` / `send_emote` send on `ChatChannel`.
- **Host** (`godot/rust/src/chat.rs`). `ChatModel` is the Bevy-free controller: receive, submit to `ChatRequest`, click, scroll, flash and scroll-hold, and `leave_world`. `Chat` owns the `ChatFrameUI` RegistryUi.
- **Keyboard:**
  - Unhandled Enter, `/` or R (R only with a reply target) opens the edit box. This runs in `unhandled_key_input` after the merchant split keys and before Escape target-clear and the game menu. It needs `InWorld`, input allowed and no game menu.
  - While the box has focus, `GameClient::input` runs before the GUI. There Enter sends, Escape closes (so the game menu does not open) and Up/Down recall history.
  - Movement and action keys stop because gameplay input is gated on a focused `LineEdit`.
  - Any other focus loss closes the box on the next frame.
- **Wheel.** The wheel over `ChatFrame1Messages` scrolls the chat and is consumed, so the camera does not zoom.
- **Copy Chat** goes to `DisplayServer.clipboard_set`.
- **Combat log.** Lines come from `Account::combat_log` by sequence number. Names come from the replicated NPC or player name in the host `Replica`, else `Unknown`. Spell names come from the native spell catalog.
- **RegistryUi additions:** `show_chat_frame`, `set_editbox_text`, `set_frame_alpha` (flash pulse without a rebuild), `frame_viewport_rect` and `release_focus_named`.
- Not ported: `/who` prints `Who is unavailable.` and group commands print `Group commands are unavailable.`, because Godot has no who or group networking yet.

## Proof

- **Live fixture.** `godot/tests/world_chat_flow.gd` runs against a private game-server on UDP 5089 with `fb_chat`/`Fbchat`. It checks:
  - geometry, the MOTD line and the edit box header;
  - that W types into the box and does not move the character;
  - the server echoes of `/say`, `/y` and `/e`, the whisper-to-offline error, `/join`, `/help`, wheel scroll and Scroll to bottom;
  - `/` prefill, Up history, Escape (the game menu stays closed) and the Combat Log tab.
- **Screenshots** are in game-engine `data/diagnostics/chat-2026-09-29/`.
- **Unit tests.** Model tests are in `godot/rust/src/chat_tests.rs`. Shared-logic tests are in `src/ui/chat_frame_tests.rs` and `src/game/chat_data.rs`.

## Gotchas

- The server sends the MOTD once per world entry, and only when one is set. A fresh redb has none: set it with the admin socket's `SetMotd` (msgpack, 4-byte LE length prefix).
- The 1-unit strip between the background (to 247) and the edit box (from 248) is in Chattynator's geometry, so the world shows through it.
- A client built after a shared-protocol change needs a server built from the same protocol. If they differ, the client fails with "the message protocol doesn't match".
