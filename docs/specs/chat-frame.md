# In-world chat frame

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Phase 2 of the [in-game UI plan](../plans/2026-09-23-ingame-ui.md): a Retail-style tabbed chat at the
bottom left, with the combat log as a tab, drawn with the default look of the Chattynator addon
(its Dark skin). Uses the existing `ChatMessage`, `EmoteIntent`, `QueryWho`, `GroupInviteIntent`
and `CombatLogEvent` protocol messages; server-side scoping of Say/Yell/Party/Whisper is
game-server work.

Chattynator references are `file:line` in its source
(`_retail_/Interface/AddOns/Chattynator`). Its textures are converted into
`data/textures/ui/chattynator/` (`ChatTabLeft/Middle/Right`, `ChatBackground` flipped vertically,
from TGA to PNG; `ChatButton`, `Copy`, `ScrollToBottom` and `Fade` PNGs as shipped).

Modern chrome follows the contracts below. Forever overrides only panel/header/tab art; [Forever reference chrome and number provenance](forever-chat-meter-chrome.md).

## What it must do

### Frame

- [x] Root frame `ChatFrame1`, 500×280 at BOTTOMLEFT (0, 40) (Core/Config.lua:28-29).
- [x] Background `ChatFrame1Background`: ChatBackground texture tinted with the tab's background colour (`1a1a1a`, Combat Log `262626`) at alpha 0.8, from the frame's left edge to 5 beyond the message area above and below (Skins/Dark.lua:151-176, 474).
- [x] Tabs General / Combat Log / Whispers (`ChatFrame1TabsTab0..2`) from (32, 0), 10 apart, each the label width (min 20) plus 30, rounded up to whole units, wide and 22 high; left/middle/right tab textures tinted General `06a1ff`, Combat Log `c97c48`, Whispers the whisper colour; label GameFontNormalSmall gold, 5 from the top; the selected tab at alpha 1, others 0.5 (Display/Main.lua:47-52, Display/Tabs.lua:47-55, Constants.lua:14-16, Skins/Dark.lua:196-310, Core/Config.lua:43, Core/Initialize.lua:48-50).
- [x] Clicking a tab selects it, stops every tab flashing and returns to the newest message (Display/Tabs.lua:170-173, 294-323).
- [x] General lists every received message, Whispers only whispers, Combat Log only combat lines.
- [x] New-message flash: an incoming message flashes the unselected tabs that list it, unless the selected tab lists it too; outgoing whispers never flash; the Combat Log tab never flashes. The flash (`ChatFrame1TabsTab{n}Flash`) is the tab art 1 wider each side and 2 taller in the tab colour, pulsing 0→1→0 every second at full alpha (Core/Config.lua:107, Display/Tabs.lua:188-230, Skins/Dark.lua:219-242, 318-363).
- [x] Messages (`ChatFrame1Messages`, 34 from the left, 27 from the top, 5 short of the right, 38 above the bottom) stack up from 2 above the bottom, newest last, 5 apart; a message that does not fit whole is not shown (Display/Main.lua:22-31, Display/ScrollingMessages.lua:178-274, Core/Config.lua:96).
- [x] Each chat message shows its local arrival time `HH:MM:SS` in grey at the left, a 2 wide Fade separator 4 left of the text, and the text from the timestamp width + 11 (Core/Config.lua:97-99, Core/Messages.lua:364-385, Display/ScrollingMessages.lua:222-266). Combat Log lines have no timestamp, separator or spacing and stop 15 short of the right, as Blizzard's ChatFrame2 that Chattynator embeds (API/CustomTab.lua:6-54).
- [x] Font: Arial Narrow 14 (ChatFontNormal, `message_font_size` 14, `line_spacing` 0), 14 per line (Core/Config.lua:95-103).
- [x] The wheel over the messages scrolls one message per notch, 5 with Ctrl, to the oldest with Shift; new lines do not move a scrolled-up view (Display/ScrollingMessages.lua:25-57).
- [x] Scroll to bottom (`ChatFrame1ScrollToBottomButton`, 26×28 at the messages' bottom right, (-2, 5)) shows while scrolled up and returns to the newest message (Display/Buttons.lua:14-18, 199-204, 307).
- [x] Copy Chat (`ChatFrame1CopyButton`, 26×28, left of the messages at the top of the `outside_left` button column) puts the selected tab's newest 200 lines, oldest first, each as `[HH:MM:SS] text`, on the system clipboard; a failure prints `Copy Chat failed: <reason>` (Display/Buttons.lua:182-191, 298-319, Display/CopyChat.lua:87-131, Core/Config.lua:116,122).
- [x] Buttons: ChatButton texture tinted 0.15 with a 15×15 icon tinted 0.8 (Skins/Dark.lua:27-150).
- [x] Lines word-wrap to the text width; links never split.
- [x] Retail wording and colours: `[Bob] says:` white, `[Bob] yells:` red, `[Party] [Bob]:` blue, `[Guild] [Bob]:` green, `[Bob] whispers:` / `To [Bob]:` pink, system yellow.

### Input

- [x] Enter opens and focuses the edit box (`ChatFrame1EditBox`, 32 high across the frame bottom on a 0.1 grey fill at alpha 0.8; Display/Main.lua:194-220, Skins/Dark.lua:177-195), putting input in Text mode, so movement and keybinds stop.
- [x] The open edit box shows Blizzard's `Say: ` header (`CHAT_SAY_SEND`) in ChatFontNormal at LEFT (15, 0), with text in the SAY colour inset 15 + header width on the left and 13 on the right (Blizzard_ChatFrameBase ChatFrameEditBox.xml:48-53, 111; ChatFrameEditBox.lua:674-692).
- [x] Enter sends and closes; Escape (in-world Escape chain) or any focus loss closes without sending.
- [x] `/` opens prefilled with `/`; R opens `/w <last whisperer> ` when someone has whispered.
- [x] Up/Down cycle the last 32 sent lines.
- [x] Enter while a popup is shown accepts the popup and does not open chat; one action per press.

### Slash commands

- [x] `/s /say`, `/y /yell`, `/p /party`, `/g /guild`, `/e /em /me /emote`, `/w /whisper /t /tell <name>`, `/r /reply`; plain text is Say.
- [x] `/dance /wave /sit /sleep /kneel` send the server emote.
- [x] `/who <query>` uses the who query path; `/invite <name>` sends a group invite.
- [x] `/help` prints a command list. Unknown commands, including `/join` and `/leave` (no server channels), print `Type '/help' for a listing of a few commands.`

### Combat log

- [x] Each `CombatLogEvent` becomes a line with source/target names resolved from the replicated `Npc` or `Player` name, else `Unknown`.
- [x] Spell ids render as `[Spell Name]` links from the spell catalog; hovering a link shows the spell tooltip.

## Not yet

- The edit box header following the chat type as you type (`/y ` switching to `Yell:`) and sticky chat types; the header always reads `Say: `.
- Godot: `/who` and group commands (no native who/group networking; they print a local line), spell link tooltips on hover, and live proof of tab flashing, Copy Chat and incoming-whisper R reply.
- Links typed in chat, shift-click link insertion, sticky chat type, `/who` result lines, group invite feedback lines, custom channels.
- Chattynator parts not taken: the Copy Chat dialog (the text goes straight to the clipboard instead), message fading after 25 s, button and tab hover tints and tooltips, the social/channel/voice/menu/search/settings buttons, the new-tab and tab-overflow buttons, tab drag, rename and colour menus, the resize grip, moving the window, the Combat Log filter bar, and message storage across sessions.

## Implementation inventory

- `godot/ui-model/src/ui/chat_frame.rs` — tabs and their colours, line formatting, slash parser, combat log lines, timestamps, flash rule, scrolling, Copy Chat text, wrapping, sent history.
- `godot/ui-model/src/ui/screens/chat_frame_component.rs` — `ChatFrame1` screen (Chattynator geometry and art) and link frame lookup.
- `src/scenes/chat_frame/mod.rs` — keyboard, tab/button clicks, wheel scrolling, flash pulse, clipboard, screen sync.
- `src/game/networking/messages_combat.rs` — combat log lines with resolved names.
- `src/scenes/static_popup/mod.rs` — popup Enter consumes the press.
- `src/scenes/tooltip_frame/mod.rs` — chat links feed the shared spell tooltip.
- `godot/ui-model/src/lib.rs` — compiles `chat_data`, `chat_frame`, `chat_frame_component` and `group_state` for the Godot host.
- `godot/rust/src/chat.rs` — Godot host: `ChatModel` (receive, submit, click, scroll, flash, leave world), `ChatFrameUI` RegistryUi, keyboard focus, wheel, Copy Chat, combat log names.
- `godot/rust/src/account.rs`, `godot/network/src/lib.rs` — `ChatMessage` in, `ChatMessage`/`EmoteIntent` out.

## Tests asserting this spec

- `src/scenes/chat_frame/tests.rs` — Bevy App tests: Enter/Text mode, whisper send, W while typing, focus loss, `/` and history, R reply, unknown command, tab routing and colours, combat log names and link, popup Enter, tab click selection and alpha, flash on an unselected tab and its clearing, no flash for the selected tab, scroll-to-bottom visibility and held view, timestamps, Copy Chat and its failure line.
- `godot/ui-model/src/ui/chat_frame_tests.rs` — parser, wording, combat lines, wrapping, history, timestamp format, flash rule, flash pulse, scroll bounds, message fit, Copy Chat text.
- `src/rendering/hud_layout_tests.rs` — chat frame on screen, clear of the HUD, tabs 10 apart.
- `godot/ui-model/src/ui/screens/chat_frame_component.rs` — link frame lookup.
- `godot/ui-model/src/game/chat_data.rs` — server channel mapping and whisper reply target.
- `godot/rust/src/chat_tests.rs` — Godot model: wording/colours/tabs, say/yell/whisper/emote requests, history, local command lines, R reply, flash, scroll hold, combat log tab.
- `godot/tests/world_chat_flow.gd` — live Godot client on a private server: geometry, MOTD, edit box header, W types without moving, server echoes of `/say` `/y` `/e`, offline-whisper error, `/join` and `/help` lines, wheel scroll and Scroll to bottom, `/` prefill, Up history, Escape without the game menu, Combat Log tab.
