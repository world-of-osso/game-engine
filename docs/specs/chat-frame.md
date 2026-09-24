# In-world chat frame

Phase 2 of the [in-game UI plan](../plans/2026-09-23-ingame-ui.md): a Retail-style tabbed chat at the
bottom left, with the combat log as a tab. Uses the existing `ChatMessage`, `EmoteIntent`,
`QueryWho`, `GroupInviteIntent` and `CombatLogEvent` protocol messages; server-side scoping of
Say/Yell/Party/Whisper is game-server work.

## What it must do

### Frame

- [x] Root frame `ChatFrame1`, 430×200 reference units, 16 from the left and bottom edges.
- [x] Tabs General / Combat Log / Whispers (`ChatFrame1Tabs`); clicking a tab switches the list.
- [x] General lists every received message, Whispers only whispers, Combat Log only combat lines.
- [x] Scrolling message list (`ChatFrame1Messages`, virtualized); it follows new lines while scrolled to the bottom.
- [x] Lines word-wrap to the list width; links never split.
- [x] Retail wording and colours: `[Bob] says:` white, `[Bob] yells:` red, `[Party] [Bob]:` blue, `[Guild] [Bob]:` green, `[Bob] whispers:` / `To [Bob]:` pink, system yellow.
- [x] The background shows while hovered or typing and fades out after 3 s idle.

### Input

- [x] Enter opens and focuses the edit box (`ChatFrame1EditBox`), putting input in Text mode, so movement and keybinds stop.
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

- Links typed in chat, shift-click link insertion, sticky chat type, `/who` result lines, group invite feedback lines, custom channels.

## Implementation inventory

- `src/ui/chat_frame.rs` — tabs, line formatting, slash parser, combat log lines, wrapping, sent history.
- `src/ui/screens/chat_frame_component.rs` — `ChatFrame1` screen and link frame lookup.
- `src/scenes/chat_frame/mod.rs` — keyboard, tab clicks, fade, screen sync.
- `src/game/networking/messages_combat.rs` — combat log lines with resolved names.
- `src/scenes/static_popup/mod.rs` — popup Enter consumes the press.
- `src/scenes/tooltip_frame/mod.rs` — chat links feed the shared spell tooltip.

## Tests asserting this spec

- `src/scenes/chat_frame/tests.rs` — Bevy App tests: Enter/Text mode, whisper send, W while typing, focus loss, `/` and history, R reply, unknown command, tab routing and colours, combat log names and link, popup Enter.
- `src/ui/chat_frame_tests.rs` — parser, wording, combat lines, wrapping, history.
- `src/ui/screens/chat_frame_component.rs` — link frame lookup.
