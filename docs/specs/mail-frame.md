# Mail Frame

The Retail `MailFrame` and `OpenMailFrame` at Mailbox game objects, and the minimap mail indicator. Contract: shared-protocol `protocol/mail_messages.rs`. Server rules: game-server `docs/specs/mail.md`. How it works: [trade and mail](../wiki/systems/trade-and-mail.md).

References: MF.xml / MF.lua = `Blizzard_MailFrame/MailFrame.xml` / `.lua`; `Blizzard_Minimap/Mainline/Minimap.xml` under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

## Native Godot client

- Both skins show the masked Mail-Icon portrait (FDID 136382, MF.lua:21). OpenMailFrame shows the normal-mail stationery icon INV_Misc_Note_01 (134327, MF.lua:298); the current wire contract carries normal player/auction mail, not custom stationery. Shared backgrounds exclude the portrait mask.
- MailFrame and OpenMailFrame use the [shared skin-resolved metal chrome](merchant-frame.md#godot-client). The compose body spans the stationery's letter area and uses a multiline scrolling editor. Received letters wrap at the existing 276px width and start at the stationery's top inset (`MF.xml:989-995`); short auction letters must show their complete denomination text, not a one-line ellipsis. This does not implement Retail's received-letter scroll frame for bodies exceeding the available stationery area.
- Auction mail subjects are formatted natively from Retail `data/db2/12.1.0.69933/GlobalStrings.csv`. The server sends `GLOBALSTRINGS_TAG:item name`, not English. Sold/won bodies carry `participant:bid:buyout:deposit:consignment:count`; invoices replace the empty letter (MF.lua:554-630). Seller rows show Sale Price (per item with stack count), Deposit, Auction House Cut and Amount Received; buyer rows show Item Purchased, Sold By and Amount Paid. Outbid/removed/expired bodies are empty. Existing refund/deposit timing and attachment rules are unchanged.
- Replicated type-19 `GameObjectInfo` and `Position` identify real mailboxes. Render/pick the `GameObjectDisplayInfo.FileDataID` model with replicated rotation/scale; unresolved metadata, models or textures must report their precise asset error, never substitute a mailbox.
- Right-click within 5 yards sends `UseGameObject`. Only its matching Mailbox role opens the authored `MailFrame` (Inbox and Send Mail tabs) with the backpack; matching `MailboxContents` may arrive before that role. Closed, unrelated and stale mailbox traffic must not reopen it.
- One mail request is in flight at a time (`C_Mail.IsCommandPending`): the mail buttons wait for matching contents, `MailSent` or `MailFailed`. No client change predicts currency, inventory or mail contents; server `Gold`, inventory and refreshed contents do.
- [x] Inbox: seven-row pages, row selection marks unread mail read, money and attachment buttons send `TakeMoney` / `TakeAttachment`.
- [x] C.O.D. (`OpenMailAttachment_OnClick`): a charge above the player's money shows `COD_ALERT` ("You do not have enough money to pay the C.O.D. charges.", Close); otherwise `COD_CONFIRMATION` ("Accepting this item will cost:" + amount) and Accept takes the item.
- [x] Delete / Return (`OpenMail_Delete`): returnable mail still holding something is returned at once; deleting mail with an item asks `DELETE_MAIL` ("Deleting this mail will also destroy %s"), with money `DELETE_MONEY`; an empty letter is deleted. Delete and Return close the open mail.
- [x] Reply (player mail): Send Mail with To = sender and Subject = "RE: subject"; a sent reply returns to the inbox.
- [x] Open All (`OpenAllMailMixin`): "Opening..." while it runs; newest mail first, its money then its items from the last slot, one request at a time; C.O.D. mail is skipped, a failed item or money is skipped, and it stops with no free bag slot, on `InventoryFull` or when nothing is left.
- [x] Send Mail: To (77 letters), Subject (64), letter (500) and gold / silver / copper boxes (digits). Right-clicking a bag item while Send Mail shows attaches it to the next of 12 buttons (soulbound: "You can't mail soulbound items."; a 13th: "You cannot attach more than 12 items to mail."); attached bag slots are locked; clicking an attachment takes it off; attachments that leave the bags fall off. An empty subject, or the one filled in before, takes the first attachment's name ("name (count)" for a stack). Postage is red when above the player's money.
- [x] Send Money / C.O.D. radio buttons; C.O.D. needs an attachment. Send is enabled with a recipient, a subject and a C.O.D. of at most 10,000 gold (`SendMailFrame_CanSend`), sends `SendMail`, and `MailSent` shows "Mail sent." and clears the form; `MailFailed` shows its Retail text and keeps the form. Cancel clears the form.
- [x] Minimap: `PendingMail` senders show `MiniMapMailFrame` (`ui-hud-minimap-mail-up`, TOPRIGHT at the Tracking button's BOTTOMRIGHT); hovering it shows "Unread mail from:" and a line per sender, `ANCHOR_BOTTOMLEFT`.
- Close/Escape (`CloseAllWindows`: MailFrame and an open letter together), server close, object removal, transfer and session reset close the mailbox and its popups. The mailbox opens all bags (`OpenAllBags`, MF.lua:63) and closes them on hide; attached items are locked in every bag.
- [x] Send Mail texts survive tab switches; Reply's To/Subject fill even though Reply is pressed on the Inbox tab.
- Focused ui-model behaviour tests: `godot/ui-model/tests/native_mailbox.rs`; network relay: `godot/network/src/wire_tests.rs`.
- [x] Live two-client proof (`godot/tests/world_mail_flow.gd`): see [native player mail proof](../wiki/systems/trade-and-mail.md#native-player-mail-proof-2026-10-01).
- [x] Receiving fixture `godot/tests/world_mail_receiving_flow.gd` (auction delivery) and its [saved proof](../wiki/systems/trade-and-mail.md#saved-native-receiving-proof-2026-10-01).
- [x] `godot/tests/world_outbid_body_flow.gd`: [private live denomination-letter RED/GREEN](auction-house-ui.md#private-live-remainder--2026-10-09), authentic received body and exact refund through real mailbox/row input. One-line ellipsis RED becomes two fully visible top-aligned lines after the stationery-height fix; this does not prove long-letter scrolling.

## What it must do (preserved Bevy client)

### Mailboxes
- [x] Replicated type-19 game objects are `WorldObjectInteractionKind::Mailbox`: the mail cursor and minimap mailbox icon, and right-clicking sends `UseGameObject`. Mailbox doodads are scenery; there is no mail keybind.
- [x] `NpcFrameEvent::Opened { role: Mailbox }` opens `MailState`; `MailboxContents` fill it. The mailbox opens the Panel `WindowId::Mail` with the backpack; closing the window sends `CloseInteraction`; `InteractionClosed` closes it.

### Inbox
- [x] Seven `MailItemTemplate` rows (305×45 at 13,−70) per page: MailItemBorder, sender, subject, time left ("N Days" green, hours / minutes red under a day), the package icon (first attachment, its count) or the stationery icon, "COD" on C.O.D. mail. Unread rows are gold with a gold slot; read rows grey.
- [x] Prev / Next (UI-SpellbookIcon page art) act only when there is a page to go to. Open All takes the money and items of every mail without C.O.D.
- [x] Clicking a row opens it in `OpenMailFrame` and marks unread mail read (`MarkRead`).

### Open mail
- [x] `OpenMailFrame` 338×424 at the Inbox frame's TOPRIGHT: From / Subject, the letter on stationery, "Take Attachments:" with the money button (`TakeMoney`) and one button per attachment (`TakeAttachment`) in rows of seven, "No Attachments" otherwise, the C.O.D. amount.
- [x] Taking a C.O.D. item asks `COD_CONFIRMATION` ("Accepting this item will cost:" + amount) first.
- [x] Reply (player mail) switches to Send Mail with To = sender and Subject = "RE: subject".
- [x] Delete / Return follows `InboxItemCanDelete`: returnable mail still holding something is returned; deleting mail with an item asks `DELETE_MAIL` ("Deleting this mail will also destroy %s"), with money `DELETE_MONEY`.

### Send Mail
- [x] To (77 letters), Subject (64) and the letter (500) edit boxes; postage "Postage:" in coins, red when unaffordable.
- [x] Twelve attachment buttons in two rows of seven (15,253 / 15,297, 45 apart). Right-clicking a bag item attaches it to the next free button; clicking an attached item takes it off.
- [x] Money entry with Send Money / C.O.D. radio buttons; C.O.D. needs an attachment.
- [x] Send sends `SendMail` with the typed form (an empty subject takes the first attachment's name; no recipient sends nothing). `MailSent` shows "Mail sent." and clears the form; `MailFailed` shows its Retail text. Cancel clears the form.

### New mail
- [x] `PendingMail` senders show the minimap `MiniMapMailFrame` icon (`ui-hud-minimap-mail-up`); no senders hide it. Hovering it shows `HAVE_MAIL_FROM` "Unread mail from:" with one line per sender (`HAVE_MAIL` without senders).
- [x] IPC: `mail status | send | read | take-item | take-money | return | delete`.

## Private live acceptance — 2026-10-09

Forever skin, pinned Godot 4.7.2, private UDP5500, `fb_mailraid_a`/`fb_mailraid_b` (Mailalpha/Mailbeta), at most two rendered cage/Vulkan clients. Inherited extension built at `5ab533364`; continuation checkout `709a62431` changes no mail/group implementation. Server `5b04aa931`. Evidence: `data/diagnostics/mailraidlive-2026-10-09/`, paired capture JSON and `inputs.jsonl`; [proof ledger](../../data/diagnostics/mailraidlive-2026-10-09/proof-ledger.txt). Full-size PNGs copied to `/syncthing/AgentShared/2026-10-09/mailraidlive/` only after 960×540 inspection. Initial inbox/open captures were accepted by the predecessor; subsequent captures inspected by the continuation.

| Case / Retail rule | Observed vs expected | Result / PNG stems |
|---|---|---|
| Inbox/open letter (`MailItemTemplate`, `OpenMailFrame`) | B sees A's subject/body, Linen20 and 1g, matching the send. | PASS `01-mail-inbox`, `02-mail-open` |
| Take money/item (`TakeMoney`, `TakeAttachment`) | B 200000→210000c; bags gain Linen Cloth20; mail13 has zero money/items. | PASS `03-mail-claimed` |
| Empty-letter delete (`OpenMail_Delete`) | Mail13 disappears; it remains absent after continuation reconnect and real mailbox reopen. | PASS `04-mail-deleted`, `b-mail-resumed.json` |
| Seven-row paging (`MailItemTemplate`) | Eleven mails: first page seven rows, next page four distinct rows; page counter changes. | PASS `06-mail-page-one`, `07-mail-page-two` |
| Return (`OpenMail_Delete`) | B's Wool5 letter15 disappears; A receives returned letter27 with Wool5. | PASS `08-mail-returned-b`, `11-return-and-cod-payment-a` |
| C.O.D. (`COD_CONFIRMATION`, `OpenMailAttachment_OnClick`) | Silk10/1g compose; B confirms 1g, 210000→200000c and gains Silk10; A receives 10000c payment letter28 and claims it, 179540→189540c. | PASS `05-cod-send-form`, `09-cod-confirmation-b`, `10-cod-accepted-b`, `11-return-and-cod-payment-a`, `12-open-all-1` |
| Open All (`OpenAllMailMixin`) | A claims returned Wool5/payment; B 200000→210100c; both finish idle with no remaining money/items. | PASS `12-open-all-1`, `12-open-all-2` |

Input is native mouse/key events, not direct mail requests. Private admin only seeds setup and accelerates cross-account delivery. No product defect or RED/GREEN repair established; stale saved-token/account mapping and missing worktree gametables were harness setup corrections. This run does not prove Open All's C.O.D.-skip/failure/full-bag branches, all send errors, both skins, exact Retail pixels or clean shutdown.

## Gaps
- No stationery choice, attachment tooltips, minimap flipbook animation or auction invoice layout.
- Native: "Mail sent." uses the red UIErrorsFrame line (no yellow info-message variant yet); bag items attach by right-click only, not by dropping a cursor item on an attachment button.
