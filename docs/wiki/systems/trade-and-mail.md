# Trade and Mail

How the client does trade and mail. Contracts: [trade frame](../../specs/trade-frame.md), [mail frame](../../specs/mail-frame.md).

## Native Godot receiving mail

The native receiving contract is in [mail frame](../../specs/mail-frame.md#native-godot-receiving-slice). Legacy sending/COD/minimap behavior below is not native proof.

- `godot/network` captures `GameObjectInfo` independently of Player/Npc into `GameObjectSnapshot`; disappearance uses the existing removal event. MailboxContents/MailFailed/PendingMail relay in MailChannel message-ID order.
- `godot/rust/src/game_objects.rs` renders only replicated mailbox objects. Its build-pinned GameObjectDisplayInfo lookup feeds the existing local-CASC M2 companion/texture loader. The M2 gets its real display FDID, replicated position/yaw/scale, native lighting and the existing drawn-triangle picker; asset failures never create placeholder nodes. It shares the picker with units, but targeting excludes objects and mailbox right-click never sends SetTarget/InteractNpc.
- `MailSession.expected` comes from the real UseGameObject input. Matching contents can wait for the interaction role across channels; mismatched contents are ignored. Claims remain pending until matching mailbox contents or failure and never mutate owned money/bags locally.
- `godot/rust/src/mail.rs` projects the authored receiving screen plus the backpack. Inventory messages continue through MerchantSession's shared InventoryState, while money reads replicated local-player Gold. Object removal/interaction close/transfer/reset clear receiving state. PendingMail senders are retained; the native minimap mail indicator is not part of this slice.
- CPU source proof at `9ed690bf`: four portable model/registry/interaction tests, three owned-UDP replication/relay/request tests, one display-parser test passed on Depot. The parser test uses concrete rows, not actual rendered asset proof. GDScript `world_mail_receiving_flow.gd` is committed but unrun; main must first prove CLI AH/mail, then build this source and run the fixture on its prepared recipient. Existing installed libraries and Options/loot runtime artifacts do not establish mail acceptance.

## Trade (preserved Bevy client)
- `game_engine::trade::TradeClientState` keeps the last `TradeSnapshot` (`player` = our side). `TradeAction`s queued by the frame, the unit / group menus, the bags and IPC go out in `send_pending_actions`; IPC actions get the next update as their reply.
- `scenes/trade_frame`: `trade_frame_state` maps the snapshot (icons via `stack_slot`, names in `merchant_data::quality_color`); `step_trade_window` opens the window on an open trade and cancels once when the player closes it (`TradeWindow::Cancelling` until the server ends the trade); `ask_trade_requests` / `answer_trade_popup` drive the `TRADE` popup.
- `scenes/bag_frame::trade_offer` offers a right-clicked stack in `first_free_slot`.

## Mail (preserved Bevy client)
- `game_engine::mail_data::MailState`: the open mailbox, its `MailboxContents`, the tab, page and open mail, the Send Mail attachments (bag guids) and money mode, and `pending_senders`. `MailRequest` is the frame / IPC action.
- `game/networking/mail.rs` follows `NpcFrameEvent` for `NpcRole::Mailbox`, fills the state from the four server messages and sends requests to the open mailbox.
- `scenes/mail_frame`: `mail_click` turns a click into state changes, requests, confirmation popups (`PendingConfirm` holds the request until the popup answers) and edit box texts; `view::mail_frame_state` builds the frame model.
- `scenes/frame_input` holds the edit box, focus and click helpers the trade and mail frames share.
- The minimap `MiniMapMailFrame` follows `MailState.pending_senders` (`rendering/ui/minimap.rs::sync_mail_indicator`).
- Mailboxes are server game objects: `game/networking/game_objects.rs` gives type 19 `WorldObjectInteractionKind::Mailbox`, which `rendering/ui/target.rs` uses like `ServerObject`.

## Sources

- [Mail frame contract](../../specs/mail-frame.md) — native receiving boundary versus preserved Bevy behavior.
- `godot/network/src/lib.rs`, `godot/rust/src/game_objects.rs`, `godot/rust/src/mail.rs`, `godot/ui-model/src/mail.rs` — native replication, assets, interaction/session and UI.
- `godot/network/src/wire_tests.rs`, `godot/ui-model/tests/native_mailbox.rs`, `godot/tests/world_mail_receiving_flow.gd` — focused CPU proof and deferred native receiving fixture.

## See Also

- [[auction-house-ui]] — auction delivery uses existing server mail.
- [[merchant-frame]] — shared native inventory state/backpack.
- [[asset-pipeline]] — actual local-CASC model/texture paths.
