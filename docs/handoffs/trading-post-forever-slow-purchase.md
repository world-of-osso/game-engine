# Trading Post slow-purchase — Forever native proof, 2026-10-20

Existing five-case fixture restored unchanged from `200b29867` onto current master; registered with the existing strict completion runner. No product behavior changes.

## Proof

- Actual engine `46c08f510` (includes `eceb660a0` and CharacterFrame fix `5726a0fd5`), server `b40e810c6`, private protocol `03cebdb`.
- Forever only, native Wayland 1920×1080, UDP5791; private world.db, fresh private.redb, complete 1131-file active DB2 copy. Account-owned preset for `fb_tpfor_oct20`, character30; ten account receipts report `Forever`.
- `ALL_CASES_DONE 5/5`, natural exit0; neither Godot stream contains panic, SCRIPT ERROR or ERROR. Existing Wayland/QuestPOI warnings retained, not claimed warning-free.
- Purchase10s/Return; Checkout10s/Escape with hidden CharacterFrame preserved; early success cancels timer; private kick hides vendor before deadline without late notice, then reconnect/refund; exact conservation and Leave/HUD restoration.
- Tender20000→20000, frozen−1, catalog42, exact eight appearance IDs conserved, empty cart, maximum sampled world delta0.
- All11 PNG originals inspected and published: `/syncthing/AgentShared/2026-10-20/trading-post-slow-purchase/forever-slow-purchase-*.png`.
- Authoritative command/revision/hash/receipt/timing ledger: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/tp-forever-slow-2026-10-20/proof-ledger.json`; explicit PNG paths/hashes in adjacent `published-pngs.json`.
- Client/server/slice stopped; UDP5791 unbound; hold directory empty; tp window cleared. Modern accepted proof not rerun.

## Separate prior-state residue — not cleaned

Historical `fb_tpui_mod` in `trading-post-ui-2026-10-13/private.redb` still owns items1615+1608, costing380 Tender, beyond its refund window. Old database hash verified unchanged.

Proposal only: back up and retire the disposable old fixture account/state after lead approval. If that state must be preserved, separately authorize an atomic offline test-state repair removing exactly those two receipts and corresponding granted ownership and restoring380 Tender. Do not widen global refund policy or alter production state.
