# Merchant Frame

This page covers the client vendor. The spec is [merchant-frame](../../specs/merchant-frame.md), and the server rules are in game-server `docs/specs/merchant.md`.

## Pieces

- `src/game/networking/merchant.rs` (`MerchantNetworkPlugin`):
  - `VendorInventory` → `MerchantState::apply_inventory`. A new NPC resets the tab and page; the same NPC (a stock refresh) keeps them.
  - `BuybackList` → `MerchantState.buyback`.
  - `MerchantFailed` → `UiErrors`.
  - `NpcFrameEvent::Closed` for the open vendor → close.
  - `MerchantRequest` messages go out on `MerchantChannel` with the open NPC's server bits.
- `src/game/networking/inventory.rs` (`InventoryNetworkPlugin`): `InventorySnapshot` / `InventoryDelta` → `InventoryState::apply_snapshot` / `apply_delta` (bag data in `src/game/bag_data.rs`).
  - Slots keep `item_guid` and `item_id`; the icon comes from `item_icons::item_icon_fdid`.
  - `item_icon_fdid` returns the appearance icon, or `Item.IconFileDataID` for items without an appearance.
  - Bags of size 0 are left out.
- `src/scenes/merchant_frame/`:
  - `build_state` builds the view model.
  - `sync_merchant_window` opens the window (plus `Bag(0)`) when a vendor list arrives. When the window is closed, it sends `NpcInteractionRequest::Close` and closes the bag.
  - `dispatch_action` handles clicks: right-click on an item buys; any click on a buyback cell buys back. Left clicks on vendor items belong to [[cursor-item]] (pickup, drop on a bag slot buys there).
  - Sell All Junk pushes `SELL_ALL_JUNK_ITEMS`; `sell_junk_on_confirm` sends `MerchantRequest::SellAllJunk` on Yes. `has_junk` = a poor bag item with a catalog sell price.
- `src/ui/screens/merchant_frame_component.rs`: the Retail layout, with every offset cited from MerchantFrame.xml/.lua. It reuses `quest_art::window_chrome` (metal_frame).
- Right-click on a bag slot (`bag_slot:<bag>:<slot>`, `scenes/bag_frame::sell_bag_item`) sends `MerchantRequest::Sell` only while the merchant tab is shown.
- `scenes/tooltip_frame::hovered_merchant_tooltip`: name in quality colour, stack count and stock for `MerchantItem<n>`.
- The frame root carries `onclick: merchant_frame` so a cursor item dropped on its background is sold (MF.xml `OnMouseUp` → `PickupMerchantItem(0)`).

## Godot

[Current bounded native Buy + whole/split sale acceptance](godot-conversion.md#native-merchant-split-cursor-sale--main-accepted-bounded-pass) owns acceptance status and exclusions; not full merchant/cursor parity.

- `godot/rust/src/merchant.rs`: right-click interact (targets, then `InteractNpc` within 5 yd), the hover cursor (`wow_cursor_data::npc_cursor`, `Input.set_custom_mouse_cursor` with the `Interface/CURSOR` BLPs), the `MerchantUI` RegistryUi, Escape before the target/game menu. The native `MerchantFrame` alone uses `ui_layout.ron` selected-character placement and 24-logical-unit title drag at effective UI scale; close excludes title capture, release saves, resize clamps, and Options reset clears cached placement. Its backpack and split frame remain separate. Native Panel L/R ordering, coexistence and raise parity remain open.
- `godot/ui-model/src/merchant.rs` (`MerchantSession`): the Bevy scene logic without ECS — frame/bag/split states, `click_frame` / `click_bag` / `split_key` → `MerchantEffect` (a request or `CloseInteraction`).
- The shared data files (`merchant_data`, `bag_data`, `stack_split`, `item_catalog`, `item_icons`, `wow_cursor_data`) compile in `godot/ui-model` with `--cfg godot_host` (its `build.rs`), which drops their Bevy `Resource`/`Message` derives; item tables read from `ui-model::paths::set_data_root`.
- `metal_frame` is composed by `panel_style_data::compose_metal_sheet` into a registry dynamic texture.
- The projection reports right-clicks and Shift-left-clicks on `onclick` frames as `UiInput::AltClick` (`RegistryUi::pop_alt_click`).
- `NpcFlags` and `Gold` are read from the host `Replica` ([[godot-replication]]); interaction/vendor/bag/durability messages arrive as `AccountEvent::Npc`.
- Repair cursor: `MerchantSession.repair_mode` (toggled by `ACTION_REPAIR_ITEM`, cleared by `close_frame`); `repair_click(location)` turns a left click on an item into `RepairItem { Some(guid) }`. The host checks it first in `character_frame_click` (paperdoll) and `bag_cursor_click` (bags), and `update_merchant` shows `ActiveWowCursor::Repair` while it is on.
- Tooltips: `tooltip_sources.rs::merchant_button_tooltip` routes `MerchantSellAllJunkButton`, `MerchantRepairItemButton`, `MerchantRepairAllButton` and `MerchantBuyBackItem` to `game_tooltip/merchant.rs` (`sell_all_junk_tooltip`, `repair_item_tooltip`, `repair_all_tooltip(cost, money)`, `merchant_tooltip` of the last sale).
- Sounds: `godot/core/src/ui_sound_kits.rs` lists the kits (`SoundKit.VolumeFloat`, `SoundKitEntry` FDIDs); `sound_ui_kits.rs` plays one file per call on its own `AudioStreamPlayer` from `data/sounds/ui/{fdid}.ogg` (extracted with `casc-local`; a missing file is an error). `merchant::click_sound(action)` picks the button's kit; `play_merchant_show_sound` plays open/close on the session's `is_open` edge. `merchant_state().ui_sounds` lists the kits played.
- `merchant_window.rs`: placement, title drag and `scroll_merchant` (the wheel over a `MerchantFrame` descendant pages and is always consumed; the backpack in the same RegistryUi is not part of the frame).
- Live-server fixture: `godot/tests/world_merchant_flow.gd` turns with TurnRight (ArrowRight); D strafes and walks the character away. The separate owned `native_input_fixture merchant-click` test replicates a nearby vendor with `NpcFlags::VENDOR`, receives actual `InteractNpc` from a ray-picked right-click, then sends `InteractionOpened`, `InventorySnapshot`, and `VendorInventory`. Authored buyback-tab and close-button left-down reach the owned Effects player; right press, release, Escape, and reopen stay quiet. Its original exit-0 log is `/tmp/claude/merchant-click-f1609b4e-final.log`; verification at `616689b7` independently revalidated that runtime/source identity (`/tmp/claude/verify-merchant-click-final.md`). The extended fixture at script `9f64af5f` and Depot-built native source `9576f32b` passes merchant-only placement, bag separation, scale/clamp, reopen/reset, target-before-menu Escape order and pointer-audio assertions (`data/diagnostics/merchant-placement-green-9f64af5f.log`, exit 0). See [[sound]] for limits.

## Native reply ordering and tooltip content — MAIN-accepted bounded functional PASS

**MAIN accepted independent1529 [bounded functional report](/tmp/claude/verify-native-merchant-tooltips-relay.md): MerchantChannel regression at `55648101b2d15e445d33cfc3e4501a554563ccfa`, literal tooltip port and supplied physical fixture assertions at `33e81860a51fda131b7864e284e5948bdff1dd91`. Not full merchant acceptance or clean-readability clearance.** Original [merchant contract](../../specs/merchant-frame.md) remains authoritative. This section owns the ordering/content evidence; existing cursor and direct-service acceptance remains in [[godot-conversion]].

| Capability | Revision and bounded proof |
| --- | --- |
| Same-channel reply order | `55648101b2d15e445d33cfc3e4501a554563ccfa` replaces independent typed relays with one relay sorting message IDs for exactly `VendorInventory`, `BuybackList`, `MerchantFailed` on OrderedReliable `MerchantChannel`. No ordering merge with other channels. Real loopback UDP regression: behavioral RED at `bbe3619b` (all five replies arrived, first not VendorInventory), GREEN at `55648101` (1 passed); initial Vendor then Buyback preserves the seeded two entries in the integrated runtime. |
| Original merchant tooltip content | `33e81860a51fda131b7864e284e5948bdff1dd91` adds dedicated portable merchant content and native merchant-only routing: quality-coloured name, conditional Stack Count, finite In Stock including zero, then unchanged common Item ID. No generic merchant catalog extras or Sell Price. Five portable tests cover bundle5/stock7, single/unlimited, refresh2/stock0, buyback3 and quality mapping; RED4/5 at `5a9c8de5`, GREEN5/5 at `33e81860`. |
| Physical native presentation | Depot `0xdh43jcwk` build exit0; supplied first runtime exit0. Vendor bundle/single, authoritative stock refresh, two buyback entries, embedded bag, empty vendor/buyback/bag cells, pointer away, service-button exclusion and server close pass original content/placement/hide assertions with 900ms stability. Open1; Buy/Sell/Buyback/Repair/Junk/Inventory transactions all zero. Embedded bag retains catalog Sell Price10, not merchant content. |

GDScript `a4858f7f4ec609d4de9daeb864a2371ceff8c4d2` only exposes observed tooltip state on failure. The existing merged geometry adjustment checks owner-side/above/clamp placement and panel-to-model size agreement using `tooltip_state().rect`; it does not independently predict the original fixed width260 or prove image parity. Content/stability/authority expectations were not weakened by `33e81860`. Source identities above and the supplied build/runtime artifacts delimit this proof, not current whole-HEAD acceptance. Initial setup/resource/order failures and subsequent genuine content RED are historical, not erased by GREEN. The first locked-dependency failure was not behavioral RED. Build retains inherited WMO `NativeWmoGroup.fdid` warning.

Runtime child416080 was intentionally killed/reaped (status9, readers0): forced cleanup, **not normal shutdown**. Full merchant and conversion goals remain open; transferred Tooltip/IPC/other-owner work remains separately owned and unaccepted here. The original five-primitive-argument `merchant_tooltip()` signature remains inherited literal-port parameter-count debt; acceptance does not clear readability or authorize API redesign. Runtime Shift comparisons, later merged CharacterFrame dispatch, repeated-run reliability and current whole-HEAD consumer acceptance remain unproved. Sell All Junk hover tooltip, per-item repair, broader catalog stats, global ownership and full merchant parity are not proven or marked done.

MAIN separately observed fresh direct-services regression after relay/tooltip compilation: Depot `tc318dqwqh` at `68dfe530`, [saved regression](/tmp/claude/native-merchant-services-ordered-relay-regression.log) exit0, Open1/Repair1/Junk1 with two authority barriers. This is MAIN-observed regression only, not new independent current-consumer acceptance.

### Sources

- [MAIN-accepted independent1529 report](/tmp/claude/verify-native-merchant-tooltips-relay.md) — exact functional scope, saved proof ledger, inherited readability debt and exclusions; no test/runtime reruns.
- [Ordering report](/tmp/claude/merchant-channel-order-report.md) — exact real-UDP RED/GREEN revisions, command and immutable logs; fixture enqueue order is not necessarily wire order.
- [Content port report](/tmp/claude/merchant-content-port-report.md) — five portable tests, source boundaries and historical runtime content RED.
- [Native build](/tmp/claude/native-merchant-content-port-build.log) and [first runtime](/tmp/claude/native-merchant-content-port-first-runtime.log) — integrated build0/runtime0, physical observations, zero transactions and forced cleanup.
- [Original specification](../../specs/merchant-frame.md) — required content and still-open capabilities.

## Gotchas

- `find_frame_at` needs `mouse_enabled`: item cells, bag slots and buttons set it, otherwise clicks fall through to the world.
- rsx `name:` takes a `FrameName`-like value; for a `&str` const use `{DynName(CONST.into())}`.
- `ui.rightClick(name)` (JS automation) is what drives buying and selling in headless proofs. The headless cage can't warp the OS cursor, and winit logs "could not set cursor position", but the Bevy window cursor is set, so the clicks land.
