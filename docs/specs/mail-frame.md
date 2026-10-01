# Mail Frame

The Retail `MailFrame` and `OpenMailFrame` at Mailbox game objects, and the minimap mail indicator. Contract: shared-protocol `protocol/mail_messages.rs`. Server rules: game-server `docs/specs/mail.md`. How it works: [trade and mail](../wiki/systems/trade-and-mail.md).

References: MF.xml / MF.lua = `Blizzard_MailFrame/MailFrame.xml` / `.lua`; `Blizzard_Minimap/Mainline/Minimap.xml` under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

## Native Godot receiving slice

Native source implements the auction-delivery dependency; the checklists below describe the preserved Bevy client, not native runtime proof.

- Replicated type-19 `GameObjectInfo` and `Position` identify real mailboxes. Render/pick the `GameObjectDisplayInfo.FileDataID` model with replicated rotation/scale; unresolved metadata, models or textures must report their precise asset error, never substitute a mailbox.
- Right-click within 5 yards sends `UseGameObject`. Only its matching Mailbox role opens the receiving frame; matching `MailboxContents` may arrive before that role. Closed, unrelated and stale mailbox traffic must not reopen it.
- Reuse authored `MailFrame`/`OpenMailFrame`, seven-row inbox paging, sender/subject/body and fixed attachment slots. Native slice excludes sending, reply, delete/return actions, bulk Open All and COD payment; COD attachments are not claimable here.
- Row selection marks unread mail read. `TakeMoney`/`TakeAttachment` address the open object and actual mail/attachment IDs, with one pending request until matching contents or failure. Claims must not predict currency, inventory or attachment removal.
- Server `Gold`, `InventorySnapshot`/`InventoryDelta` and refreshed mailbox contents update native state/backpack; failures show server UI text. Close/Escape, server close, object removal, transfer and session reset close receiving state.
- Focused native metadata, interaction/model/authored-registry and owned UDP tests cover source behavior. Main owns CLI-first AH proof, extension build, actual native receiving fixture and final integration; no source-only test establishes rendered/live acceptance.

Receiving fixture: `godot/tests/world_mail_receiving_flow.gd`, run only after main's CLI proof. It requires an owned loopback endpoint, prepared recipient within range, exact real mailbox entry/display/model FDIDs, one proceeds mail and at least two won/returned item mails. It uses real model triangle picking and authored controls, then asserts received labels, exact inventory/currency changes and quiet reopen. Environment inputs are documented in the fixture; startup selects the prepared character normally with `--server`, `--screen inworld`, `--char` after Godot's `--` separator. No protocol injection or claim shortcuts.

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

## Gaps
- The letter edit box is one line (Enter leaves it); no stationery choice, no attachment tooltips, no minimap flipbook animation, no auction invoice layout.
