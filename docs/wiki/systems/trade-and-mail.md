# Trade and Mail

How the client does trade and mail. Contracts: [trade frame](../../specs/trade-frame.md), [mail frame](../../specs/mail-frame.md).

## Trade
- `game_engine::trade::TradeClientState` keeps the last `TradeSnapshot` (`player` = our side). `TradeAction`s queued by the frame, the unit / group menus, the bags and IPC go out in `send_pending_actions`; IPC actions get the next update as their reply.
- `scenes/trade_frame`: `trade_frame_state` maps the snapshot (icons via `stack_slot`, names in `merchant_data::quality_color`); `step_trade_window` opens the window on an open trade and cancels once when the player closes it (`TradeWindow::Cancelling` until the server ends the trade); `ask_trade_requests` / `answer_trade_popup` drive the `TRADE` popup.
- `scenes/bag_frame::trade_offer` offers a right-clicked stack in `first_free_slot`.

## Mail
- `game_engine::mail_data::MailState`: the open mailbox, its `MailboxContents`, the tab, page and open mail, the Send Mail attachments (bag guids) and money mode, and `pending_senders`. `MailRequest` is the frame / IPC action.
- `game/networking/mail.rs` follows `NpcFrameEvent` for `NpcRole::Mailbox`, fills the state from the four server messages and sends requests to the open mailbox.
- `scenes/mail_frame`: `mail_click` turns a click into state changes, requests, confirmation popups (`PendingConfirm` holds the request until the popup answers) and edit box texts; `view::mail_frame_state` builds the frame model.
- `scenes/frame_input` holds the edit box, focus and click helpers the trade and mail frames share.
- The minimap `MiniMapMailFrame` follows `MailState.pending_senders` (`rendering/ui/minimap.rs::sync_mail_indicator`).
- Mailboxes are server game objects: `game/networking/game_objects.rs` gives type 19 `WorldObjectInteractionKind::Mailbox`, which `rendering/ui/target.rs` uses like `ServerObject`.
