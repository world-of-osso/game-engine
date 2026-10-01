# Trade and Mail

How the client does trade and mail. Contracts: [trade frame](../../specs/trade-frame.md), [mail frame](../../specs/mail-frame.md).

## Native Godot mail

The native receiving contract is in [mail frame](../../specs/mail-frame.md#native-godot-receiving-slice). Legacy sending/COD/minimap behavior below is not native proof.

- `godot/network` captures `GameObjectInfo` independently of Player/Npc into `GameObjectSnapshot`; disappearance uses the existing removal event. MailboxContents/MailFailed/PendingMail relay in MailChannel message-ID order.
- `godot/rust/src/game_objects.rs` renders only replicated mailbox objects. Its build-pinned GameObjectDisplayInfo lookup feeds the existing local-CASC M2 companion/texture loader. The M2 gets its real display FDID, replicated position/yaw/scale, native lighting and the existing drawn-triangle picker; asset failures never create placeholder nodes. It shares the picker with units, but targeting excludes objects and mailbox right-click never sends SetTarget/InteractNpc.
- `MailSession.expected` comes from the real UseGameObject input. Matching contents can wait for the interaction role across channels; mismatched contents are ignored. Claims remain pending until matching mailbox contents or failure and never mutate owned money/bags locally.
- `godot/rust/src/mail.rs` projects the authored receiving screen plus the backpack. Inventory messages continue through MerchantSession's shared InventoryState, while money reads replicated local-player Gold. Object removal/interaction close/transfer/reset clear receiving state. PendingMail senders are retained; the native minimap mail indicator is not part of this slice.
- CPU source proof at `9ed690bf`: four portable model/registry/interaction tests, three owned-UDP replication/relay/request tests, one display-parser test passed on Depot. The parser test uses concrete rows, not actual rendered asset proof. This historical CPU proof did not run `world_mail_receiving_flow.gd`. The later saved receiving run below supplies bounded runtime evidence; unrelated Options/loot artifacts do not establish mail acceptance.

### Saved native receiving proof (2026-10-01)

Evidence root and preceding real CLI AH/mail receipts: [auction runtime evidence](auction-house-ui.md#saved-native-runtime-proof-2026-10-01). `native-mail-run6.json` identifies `/tmp/pyrun-tmux/604786dcbe1f4e8ab3501dd965b7f385.log`, exit 0, and `native-mail-shots/`. It uses actual mailbox entry 197134, display 1907 and model FDID 199999, proceeds mail ID 25 and attachment mail IDs 28/24. The saved PASS records actual model triangle picking → mailbox role/contents → authored receiving UI → proceeds/won/returned claims → authoritative Gold/inventory → quiet reopen. The fixture checks exact currency addition, exact attachment removal and inventory counts, unchanged Gold on item claims, and unchanged money/bags after reopening; it does not predict claims locally.

Earlier `/tmp/pyrun-tmux/68515b504e7d48db98707a7e88c8966b.log` showed replicas present but `rendered=false`. Root fix `90dc2ccf` removes object reset from the initial terrain request: replication can precede LoadTerrain, so entity despawns own object lifetime; lighting still resets. `/tmp/pyrun-tmux/120e239e8e8c4b98a01c19db4b7d8c38.log` then showed actual models present/rendered but outside the pickable camera view. The successful fixture moved only the owned player to world coordinates (-8810.13, 642.57, 94.875); no production picking/range bypass was added.

LiquidObject 42 and local-CASC/UI icon errors remain; [auction evidence](auction-house-ui.md#saved-native-runtime-proof-2026-10-01) records partial icon extraction. Exit 0 proves this bounded receiving sequence, not universal rendering, clean resources, performance or shutdown. Sending, reply, delete/return controls, Open All, COD payment and native minimap indication remain outside this receiving slice. Full conversion and broader integration remain owned elsewhere.

### Native player mail (2026-10-01)

- `godot/ui-model/src/mail.rs` `MailSession` owns the native mailbox: inbox paging/selection, the Send Mail draft (attachment guids, C.O.D. mode, the subject it filled in), the one in-flight request (`Pending::Request` until matching `MailboxContents`/`MailFailed`; `Pending::Send` until `MailSent`/`MailFailed`), the request a popup stands for, and Open All's skipped items. `click` / `attach` / `popup_result` / `next_open_all` return `MailEffect`s (outgoing message, popup, edit-box texts, UI error).
- `godot/rust/src/mail.rs` reads the Send Mail edit boxes each frame (cut to their letters, money boxes digits only), routes the mail UI's clicks through the bag-input queue (`mail_cursor_click`: right-click on a bag slot attaches while Send Mail shows; other bag clicks keep bag behaviour), pushes mail popups on the shared `PopupStack` and answers them from `update_group_frames`, and sends one Open All step per frame once the last request is answered.
- `PendingMail` is kept across Loading, map changes and transfers (`Mailbox::close`); only a new connection clears it (`Mailbox::reset`). The server sends it only when the senders change, and the first one usually arrives while the client still shows Loading.
- The minimap icon and its tooltip read `MailSession.pending_senders`; the tooltip reuses the bag tooltip UI.

### Native player mail proof (2026-10-01)

`godot/tests/world_mail_flow.gd` on a private server (UDP 5111, fresh redb, own admin socket), orchestrated by `/home/osso/.worktrees/.playermail-proof/run.sh` (logs, `sync/` flags and `shots/` in the same directory). Accounts `fb_mail_a` / `fb_mail_b` (human, different accounts), both placed 4 yd from Stormwind mailbox entry 197134. All requests came from real clicks, right-clicks and typed keys.

- Sender: unknown recipient and self refused with Retail text, form and money kept; money (5g), Linen (item), Silk C.O.D. 1g, Wool (to be returned) and a letter sent; "Mail sent." each time; gold 100000 → 49850 (5g + 5 × 30c postage).
- Recipient, online while mail arrived: minimap icon and "Unread mail from: Postalpha" tooltip; inbox held only the money mail and the letter until `mail-deliver-now` (item mail to another account waits 1 h); money taken and the emptied letter deleted; Linen taken; Silk: `COD_CONFIRMATION` "Accepting this item will cost: 1g", Accept, gold −1g, Silk in bags; Wool mail Return. Gold 20000 → 60000.
- Sender again: minimap icon for the C.O.D. payment, then after `mail-deliver-now` (returned items to another account wait 1 h) the payment (10000c) and the returned mail; both taken. Gold 59850.
- Server restart on the same redb: recipient still has the unread letter (minimap icon), Linen 20 and Silk 10, gold 60000; sender still has Wool 5, gold 59850.
- Not covered live: Open All, Reply, `COD_ALERT`, `DELETE_MAIL` / `DELETE_MONEY` popups (ui-model tests only). Two harness problems were hit and fixed in the harness: both clients share the checkout's saved-token file, so the first sender run logged in as the recipient's account and displaced it; the recipient's Return step ran in a resumed process after a fixture label lookup was fixed. The run surfaced the `PendingMail`-during-Loading bug above, fixed before the passing run.
- Visible but outside mail: some item icons missing (local asset cache), `LiquidObject 42` errors; the C.O.D. amount label overlaps the attachment count in `OpenMailFrame`.

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
- `godot/network/src/wire_tests.rs`, `godot/ui-model/tests/native_mailbox.rs`, `godot/tests/world_mail_receiving_flow.gd` — focused CPU proof and bounded native receiving assertions.
- `native-mail-run6.json` and the exact saved runtime/diagnostic logs above — receiving proof and replication/terrain/camera failure boundaries.

## See Also

- [[auction-house-ui]] — auction delivery uses existing server mail.
- [[merchant-frame]] — shared native inventory state/backpack.
- [[asset-pipeline]] — actual local-CASC model/texture paths.
