# Mail Frame

The Retail `MailFrame` and `OpenMailFrame` at Mailbox game objects, and the minimap mail indicator. Contract: shared-protocol `protocol/mail_messages.rs`. Server rules: game-server `docs/specs/mail.md`. How it works: [trade and mail](../wiki/systems/trade-and-mail.md).

References: MF.xml / MF.lua = `Blizzard_MailFrame/MailFrame.xml` / `.lua`; `Blizzard_Minimap/Mainline/Minimap.xml` under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

## What it must do

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
- [x] `PendingMail` senders show the minimap `MiniMapMailFrame` icon (`ui-hud-minimap-mail-up`); no senders hide it.
- [x] IPC: `mail status | send | read | take-item | take-money | return | delete`.

## Gaps
- The letter edit box is one line (Enter leaves it); no stationery choice, no attachment tooltips, no minimap tooltip or flipbook animation, no auction invoice layout.
