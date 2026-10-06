# Trade Frame

The Retail `TradeFrame` for player-to-player trade. Contract: shared-protocol `protocol/gameplay_messages.rs` (`TradeStateUpdate`, `TradeSnapshot`, the trade requests, `CancelTradeAccept`). Server rules: game-server `docs/specs/trade.md`. How it works: [trade and mail](../wiki/systems/trade-and-mail.md).

References: TF.xml / TF.lua = `Blizzard_UIPanels_Game/Mainline/TradeFrame.xml` / `.lua`; GameDialogDefs.lua (`TRADE`) under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

## What it must do

### Starting a trade
- [x] Right-clicking another player's unit frame (target / focus) or group frame offers "Trade" (`UnitPopupTradeButtonMixin`), which sends `InitiateTrade` with the player's name. There is no Trade on yourself.
- [x] An incoming request shows the `TRADE` popup "Trade with %s?" (Yes / No, 60 s timeout). Yes sends `AcceptTrade`; No and the timeout send `DeclineTrade`. The request ending hides the popup.
- [x] Server refusals and messages (`ERR_TRADE_*`, "Trade complete.", "Trade canceled.") show in the error frame.

### The window
- [x] An open trade opens the Panel `WindowId::Trade`; Retail TradeFrame opens no bags (no `OpenAllBags`). Closing the window (close button, Escape's `CloseAllWindows`) sends `CancelTrade` once; the trade ending closes the window.
- [x] Native: TradeFrame is `toplevel` (TF.xml:143), raised on press like the other panels.
- [x] Both skins bind the local player's masked portrait in the main ring (`SetPortraitToUnit("player")`, TF.lua:69); backgrounds exclude its mask.
- [x] 344×446 `ButtonFrameTemplate` chrome; the player's name at 65,−5 and the partner's at 230,−5; the partner half tinted white .15 from TOPRIGHT −172,−20.
- [x] Seven `TradeItemTemplate` slots per side (player at 14,−89, partner at 182,−89, 7 px apart, the seventh 28 px lower) with UI-EmptySlot, UI-QuestItemNameFrame, the item icon, count and name in its quality colour. The empty seventh slot shows UI-TradeFrame-EnchantIcon; both seventh slots are labelled "Will not be traded".
- [x] `InsetFrameTemplate` borders under the item columns, the seventh slots and both money rows.
- [x] Each accepted side shows the UI-TradeFrame-Highlight strips over its items and seventh slot (`TRADE_ACCEPT_UPDATE`).
- [x] Trade (85×22 at BOTTOMRIGHT −85,5) sends `ConfirmTrade` and is disabled once the player has accepted. Cancel (77×22) sends `CancelTradeAccept` when the player has accepted, otherwise `CancelTrade`.

### Offers
- [x] Right-clicking a bag item while the trade is open offers the whole stack in the first free traded slot (`SetTradeItem`); an offered item is not offered twice.
- [x] Native: an item on the cursor clicked or dropped on any player slot, the "Will not be traded" slot included, is offered there (`ClickTradeButton`, TF.xml:108-116) with the cursor's count; the cursor empties.
- [x] Clicking one of the player's offered items takes it back (`ClearTradeItem`); clicking an empty slot without a cursor item sends nothing.
- [x] The money entry (`TradePlayerInputMoneyFrame` at 11,−61) sends `SetTradeMoney` when it loses focus or on Enter / Tab, only when the amount changed; it shows the offered money while not being typed in. The partner's money shows in coins at TOPRIGHT −5,−64.
- [x] IPC: `trade status | initiate | accept | decline | cancel | set-item | clear-item | set-money | confirm | cancel-accept`.

## Gaps
- No item tooltips on trade slots, no enchanting through the seventh slot; the Bevy client has no cursor drops on trade slots; a click on an offered item clears it instead of picking it up onto the cursor; no TradeFrame `OnMouseUp` drop on the frame body.
