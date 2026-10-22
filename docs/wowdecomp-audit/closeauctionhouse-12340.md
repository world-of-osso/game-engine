# `CloseAuctionHouse()` — build 12340

Source inspection: 2026-10-22. Reference W: `wowdecomp` revision `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` (3.3.5a/12340 reconstruction). Findings describe reconstructed source, not observed native-client behavior or Retail parity. No game client, simulator, server, build, or tests were run. Behavioral prose was relayed to the requesting wow-ui-sim session.

## Results

- **Arguments and returns:** the registered Lua wrapper does not inspect its Lua stack, calls the close routine, and returns zero Lua values. No argument validation or binding-level error path appears; extra arguments are ignored by this wrapper. No explicit protected/insecure check appears at this binding. This does not rule out an external script-execution security layer.
- **State:** when an auctioneer is active, this is local interaction/UI-state teardown, not a server-close request in the inspected path. It clears the browse, owner, and bidder result arrays, auctioneer and house ID, owner/bidder-list permissions, and sell-item state. It clears the interaction target. It does **not** explicitly reset selected browse/owned/bid IDs, sort criteria, total-result counters, or query-pending fields in this function. Sell state is cleared; if a multisell is pending, its items are unlocked and `AUCTION_MULTISELL_FAILURE` is also signaled.
- **Events and order:** on an active auctioneer, `AUCTION_HOUSE_CLOSED` is signaled at the start of the close routine. If a multisell is pending, `OnMultisellFailure(false)` follows: it resets pending-sale state, unlocks sell items, calls `ClearSellItem()`, then signals `AUCTION_MULTISELL_FAILURE`. `ClearSellItem()` conditionally signals `NEW_AUCTION_UPDATE` when a sell item was set. After that, `ClearInteractTarget()` can re-enter `CloseAuctionHouse()` when the auctioneer is also the interaction target and the auctioneer branch is reached in `CloseInteraction()`'s `else if` chain. `CloseInteraction()` clears the target before calling the close routine, while `m_auctioneer` is still nonzero. This reentrant call signals a second `AUCTION_HOUSE_CLOSED` before either invocation clears the auctioneer; the cleared target prevents further recursion through this path. Its teardown can conditionally signal `NEW_AUCTION_UPDATE` while clearing any remaining sell item. Thus, with a matching interaction target, the closed event can fire twice; exact additional events depend on pending multisell and whether a sell item remains. A call made after the auctioneer has been cleared returns without event or cleanup. The inspected wrapper and close routine contain no server acknowledgment dependency.
- **Security boundary:** the binding exposes the global and contains no protected/insecure gating. That establishes only the absence of a check in this wrapper, not the complete native client's security policy.

## Evidence

- W `WoW/Source/Ui/AuctionHouse_Scripts.cpp:1021–1023,1237–1239`: wrapper and global registration.
- W `WoW/Source/Ui/AuctionHouse.cpp:1610–1630`: active-session guard, event and teardown order.
- W `WoW/Source/Ui/AuctionHouse.cpp:361–369,619–627`: conditional `NEW_AUCTION_UPDATE` from sell-item clearing; pending multisell failure cleanup/event.
- W `WoW/Source/Ui/AuctionHouse.cpp:71–149`: selected-result IDs, browse/bid/owner totals, query-pending fields and sort-criteria state declarations; these are not reset in the close function.
- W `WoW/Source/Ui/GameUI.cpp:4298–4340,4351–4366`: interaction close clears target then calls auction-house close through the matching `else if` branch; target reset prevents another nested close through `ClearInteractTarget`.
- W `WoW/Source/Ui/Types.h:1084–1090,1341`: close, new-auction-update and multisell-failure event identifiers.

## Limits

Reconstructed 12340 source is not native execution evidence; annotation/address tags do not independently certify every reconstructed behavior. Direct `FrameScript_SignalEvent` calls establish call order in this code, not any broader scheduling guarantees of addon handlers. A matching interaction target adds the reentrant close path described above; duplicate close events are a reconstructed call-graph consequence, not a native-client runtime observation. No conclusion is made about server state beyond the absence of a close packet/ack in this inspected call path.
