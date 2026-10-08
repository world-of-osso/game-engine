# Bank Frame

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

The Retail combined `BankFrame` (character bank + Warband bank) running against the live server banker. Contract: shared-protocol `protocol/bank_messages.rs`; server rules: game-server `docs/specs/banks.md`. How it works: [banks](../wiki/systems/banks.md).

References: BF.xml / BF.lua = `Blizzard_UIPanels_Game/Mainline/BankFrame.xml` / `.lua` under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`; strings from GlobalStrings (build 12.1); atlases from UiTextureAtlas (12.1.0.69933).

## What it must do

### Opening and closing
- [x] The `NpcFrameEvent::Opened { role: Banker }` event, which comes from the gossip option "I would like to check my deposit box.", opens `BankState` on the character bank. The server's two `BankContents` fill it. Contents that arrive while no banker is open are dropped.
- [x] The bank opens as the Wide `WindowId::Bank` window, together with the backpack.
- [x] Closing the window sends `CloseInteraction` and closes the backpack.
- [x] `InteractionClosed` for the banker closes the frame.
- [x] A right-click on a living banker beyond interaction range still sends the interaction attempt; the server refuses it with `You are too far away.` without opening the bank. Other NPCs retain their existing out-of-range target-only behavior.

Range regression (2026-10-08): `data/diagnostics/bankloop-2026-10-08/resume-too-far-response.{json,png}` records real John Burnside input on `fc5156f85` at 5.69 yards: neither gossip nor refusal. Native `merchant.rs` discarded the attempt beyond its approximate five-yard limit. Matched-boundary `assert-banker-attempt.py` is RED on that capture and GREEN on `range-green.json`, where the server opens gossip at the same position. At 11.142 yards, real button-down input in inspected `trace-held.png` shows `You are too far away.` and a closed bank; `assert-bank-range.py` passes (`trace-held-green.log`). Trace confirms living NPC, flags 131073 and `Interact`; temporary product logging was removed in `b57f7e483`. Late snapshots missed this three-second hold/half-second fade: held JSON to released JSON took 7.23 seconds. No release-clearing or server defect was established. Inspected `clean-range-green.png` and `clean-range-green.log` reprove the exact refusal on the final trace-free artifact; `clean-bank-reopened.png`/`clean-bank-closed.png` also reprove normal opening and X closure. Final whole-crate gate follows below.

### Layout
- [x] Frame 738×460 (BF.xml:674) with the `metal_frame` chrome and the active-skin `bank-frame-background` atlas. In both skins the portrait is the open banker (`BF.lua:94`, `SetPortraitToUnit("npc")`), masked at (-3,-7,58,58). Backgrounds exclude that mask; the bank atlas begins at y=51 and retains its bottom inset of 30.
- [x] Slot grid: 98 slots, column-major in 7 rows (BF.lua:941-968).
  - The first slot is at 26,63; the next row is 10 px below; columns are 8 apart within a pair and 19 apart between pairs.
  - Slot backgrounds are `bags-item-slot64` (4701874) in the character bank. The Warband bank uses `warband-bank-slot`, grown −6,5 / 6,−7.
  - Stack counts show above 1.
- [x] Side tabs: 32×32 with the SpellBook-SkillLineTab border (136831). The first is at TOPRIGHT +2,−25, and each next one 17 px below the previous (BF.lua:907-924).
  - The selected tab shows a gold UI-Quickslot2 marker (see Known gaps).
  - The purchase tab (`bags-icon-addslots`) comes last while a tab can be bought.
- [x] Header: the tab name in a 300×20 box at TOP −36.
- [x] Bottom tabs `Bank` / `Warband Bank` hang at the frame's BOTTOMLEFT 22,2 (PanelTab art).
- [x] Purchase prompt: while the purchase tab is selected, or while the bank has no tabs, the prompt replaces the slots.
  - It shows the title and `CHARACTER_BANK_TAB_PURCHASE_PROMPT` / `ACCOUNT_BANK_TAB_PURCHASE_PROMPT`.
  - Below that, "Cost:" and the Retail `BankTab` price, then a 105×21 Purchase button.
  - Purchase is disabled when the player can't afford the tab. Retail reds the price; here the money frame greys it.
- [x] Purchase opens `CONFIRM_BUY_*_BANK_TAB` ("Do you want to purchase a Warband Bank tab for:\n1000g"). Accepting it sends `BankPurchaseTab`.
- [x] Warband bank only: the money frame, 394×25 at BOTTOMRIGHT −3,3, with ThinGoldEdge 178×19, the stored money, and Withdraw and Deposit buttons (105×21).
- [x] Deposit All button (256×24): `Deposit All Reagents` in the character bank, `Deposit All Warbound Items` in the Warband bank. The Warband bank also shows the "Include tradeable reagents" checkbox.

### Skin art
- Dedicated bank textures resolve Blizzard atlas element names through `ui_toolkit::atlas::resolve_region`; Modern keeps pre-conversion FDIDs and UVs; portrait binding and background exclusion apply to both skins.
- Forever uses `bank-frame-background`, `bags-item-bankslot64` for character slots, `bank-frame-item-slotframe` over item icons, and the scaled `bank-divider`. Warband slot art and purchase-tab art retain their set-0 members. The tinted purchase-prompt background resolves `bags-item-slot64`.
- Camelot art does not change the 98-slot grid, slot actions, tabs or purchase/money state. Its 88-slot page cap and uniform spacing are behavior, excluded from this art conversion.
- Standalone container windows are Retail `ContainerFrameTemplate` (Retail/Forever Mainline ContainerFrame.xml:218-265, ContainerFrame.lua:9-12,779-874): 178 wide, 37×37 slots 5 apart in a `BottomRightToTopLeft` grid from BOTTOMRIGHT (-7, 9), `PortraitFrameFlatTemplate` chrome (flat background, portrait metal border, title at left 35, close button that closes only that bag), `bags-item-slot64` under every slot through the active skin. Not drawn: portrait icon (no texture masks in rsx), backpack search box/sort button/money frame rows.

Source roots: Retail / Forever = `~/.cache/wow-ui-sim/blizzard-ui/{retail,wowforever}/AddOns/Blizzard_UIPanels_Game/`. M / F = `data/db2/{12.1.0.69933,1.60.1.69913}/UiTextureAtlasMember.csv`. Atlas sheet FDIDs come from those builds' `UiTextureAtlas.csv`.

| Name | UI source | Member rows M / F | Modern / Forever FDID |
|---|---|---|---|
| `bank-frame-background` | Retail `Mainline/BankFrame.xml:677` | 11422 / 17833 | 5782252 / 8118796 |
| `bags-item-slot64`; `bags-item-bankslot64` | Retail `Mainline/BankFrame.xml:571`, `.lua:586`; Forever `Camelot/BankFrame.xml:36` | 7767 / 18117; bankslot F:17832 | 4701874 / 8187737; Forever character slots 8118792 |
| `warband-bank-slot` | Retail `Mainline/BankFrame.lua:582` | 11547 / 11565 (set 0) | 5782246 / 5782246 |
| `bags-icon-addslots` | Existing purchase-tab crop, M/F member:2883 (no BankFrame XML/Lua name literal) | 2883 / 2883 | 969828 / 969828 |
| `bank-frame-item-slotframe`; `bank-divider` | Forever `Camelot/BankFrame.xml:34`; `:76-79` (scale 0.48, BOTTOM +220) | absent / 18121; absent / 18118 | absent / 8188339 |

Sheet rows M:84,1346,1772,1774; F:2616,2617,2689,2690. Forever divider's 864×32 member becomes 414.72×15.36 at (161.64,224.64) in the existing 738×460 frame.

Local BLPs **8118796, 8118792, 8188339** are present (verified 2026-10-08); inspected `bankloop-2026-10-08/resume-forever-character.png` and `resume-alt-warband.png` render the Forever bank art. Resolution is not guarded or substituted when files are absent.

### Actions
- [x] Right-clicking a filled slot sends `BankWithdraw` for the shown bank and tab.
- [x] Right-clicking a bag item while the bank is open sends `BankDeposit` into the shown bank's selected tab. Nothing is sent while the purchase prompt shows.
- [x] Deposit / Withdraw opens the money entry: a gold/silver/copper `MoneyInputFrame` in a StaticPopup. Accept sends `BankMoneyTransfer` with the typed copper; an empty entry sends nothing.
- [x] Right-clicking a side tab opens the tab settings: the name box (15 letters) and the Equipment / Consumables / Profession Goods / Reagents / Junk assignments. Okay sends `BankUpdateTabSettings`.
- [x] `BankFailed` shows its Retail error text in UIErrors.
- [x] The IPC `warbank` status lists the Warband bank contents.

## Live proof
- `data/diagnostics/banks-20260924/`: isolated server on :5058.
  - Bankone buys a character tab (1g), deposits Linen and withdraws it, then buys Warband tabs 1 and 2 (1000g and 25,000g).
  - Bankone deposits Copper Ore and 100g into the Warband bank.
  - Banktwo, on the same account, sees the ore, withdraws it, and withdraws 10g.

- Native (Godot) client, `data/diagnostics/bank-live/` (2026-10-01, game-engine `bank`, private server UDP 5114, fresh redb, `godot/tests/bank_live.gd`):
  - Bankone right-clicks Olivia Burnside, picks the gossip option, buys a character tab (1g), deposits and withdraws Linen, buys Warband tab 1 (1000g), deposits Linen and 1g there and withdraws the Linen.
  - Escape closes BankFrame and its bags without opening the game menu.

### Resumed private acceptance — 2026-10-08

Evidence root: `data/diagnostics/bankloop-2026-10-08/`; append-only `proof-ledger.txt` distinguishes the earlier warm binary from matching `fc5156f85` extension/CLI. Private server binary remains `game-server.26f3c5c`, UDP 5360, same redb; protocol remains pinned at `7597908`. Only `fb_bank1` (Bankone/Bankalt) and `fb_bank2` (Banktwo) were administered. No protected-instance access, rebase, merge or push.

| Step | PASS / FAIL / N-A | Inspected evidence and scope |
|---|---|---|
| 1 Lifecycle | PASS open/X/Escape/range close/reopen and authoritative distant refusal after fix | Matching-source `resume-open.png`, `resume-x.png`, `resume-escape.png`, `resume-range.png` and IPC/UI dumps. Matched real-input RED/GREEN at 5.69 yards: old silent `resume-too-far-response.png` vs new server gossip `range-green.png`. At actual 11.142-yard separation, inspected `trace-held.png` renders the exact server error and no bank. Fix `00c8615c4`, regression naming `2acedbd17`, temporary trace removed `b57f7e483`; final trace-free `clean-range-green.png`/log (exact refusal) and `clean-bank-reopened.png`/`clean-bank-closed.png` inspected, with explicit-socket IPC/UI dumps. |
| 2 Tabs/purchase | PASS | Matching-source `current-character-cost.png`/`current-character-confirm.png`/`current-character-tab3.png` show 500g purchase; `current-insufficient-inert.png` shows 100,000g disabled. `resume-purchase-poor.png`, `resume-purchase-confirm.png`, `resume-purchase-bought.png`: Banktwo at 999g cannot buy 1000g; seeded 2000g buys, leaves 1000g and 98 slots. Original warm Character 1g/Warband 1000g+25,000g proof retained, not replayed. |
| 3 Tab settings | PASS name/filter close/reopen/relog; N-A icon picker | Matching-source `current-settings-relog.png`/JSON/UI dump show Bank Gear, Equipment/Profession Goods/Reagents retained after relog; earlier close/reopen `baseline-settings-reopened.png`. Icon picker explicitly deferred, not tested or invented. |
| 4 Items | PASS supported moves/full/rejection; N-A bank-origin drag/split/swap/direct inter-tab move | Inspected warm `baseline-drag-deposit.png`, `baseline-soulbound-rejected.png`, `full-tab-rejected.png`: Copper Ore7 whole-stack drag, exact `Soulbound items cannot be stored in the Warband Bank.`, 98 Hearthstones and 99th rejection `Your bank is full` without bag loss. Matching `current-between-tabs.png` plus IPC confirms Linen20 withdrawn/redeposited into Warband tab2; `resume-forever-character.png` confirms persistent full grid. Bank requests have no partial count, swap or bank-origin cursor location; no unsupported request/response/log is claimed. |
| 5 Money/isolation | PASS | Matching `current-money-deposited.png`/`current-money-withdrawn.png`: 75g→175g→150g, wallet 33,240,000→32,240,000→32,490,000 copper. `current-money-overdraw.png` leaves both balances unchanged; warm `baseline-money-zero.png` sends nothing and changes nothing. Bankalt `resume-alt-warband.png` sees shared Linen20/150g but its own 98-Hearthstone Character bank, not Bankone's Bank Gear/Ore7. Concurrent `resume-account1-concurrent.png` vs `resume-account2-warband.png` proves fb_bank2 has no tabs/items and 0g before its own purchase; explicit socket IPC dumps retained. |
| 6 Warband IPC | PASS item contract; N-A money/settings fields | `current-between-tabs-status-warbank.txt`, `resume-alt-warband-status-warbank.txt`: location98/GUID173/item2589×20 matches tab2 window. fb_bank2 status is empty. `status warbank` reports item inventory only; no balance/tab-settings status exists in this contract. |
| 7 Layout/skins | PASS bounded geometry/art inspection; FAIL full Retail equivalence | Modern `resume-open.png`, `resume-purchase-{poor,confirm,bought}.png`; Forever `resume-forever-character.png`, `resume-alt-warband.png`, dialog captures. `layout-comparison.json` checks 396 observable frame/grid values against cached Retail BF.xml:674/BF.lua:941–968: 738×460, 98 slots of 37×37, first26/63, row47, column45 plus11 per pair. Portrait control62×62(-5,-7) has Retail mask58×58(-3,-7), not a sizing defect. Existing non-additive selected-tab marker, stretched/non-tiled backgrounds, omitted edge shadows and grey rather than red unaffordable price remain; no full pixel equivalence claimed. |

No server bank defect established. Missing partial-stack/bank-to-bank operations are protocol limitations, not client workarounds: `BankDeposit` names a whole bag stack; `BankWithdraw` names a whole bank slot. Bank-origin drag, split/swap and direct inter-tab requests cannot be emitted, so there is no request, response or server log for them. Modern/Forever remain skins of the same 98-slot mechanics; Camelot's 88-slot behavior and tooltip record-ID removal are not adopted.

### Final verification and cleanup

On `68d7109df` (production source byte-identical to `2acedbd17`, SHA-256 `70d5bbc5660bda77e1076b86f27c66b3e013350cfe0850fc9d0dee18c2f6bd98`), the required locked/local/pinned five-crate `cargo test --no-fail-fast` exits **0**: CLI **103**, core **781**, Godot **661**, network **66**, UI-model **754** passed; **2365 passed, 0 failed, 6 ignored fixture-generation tests**. Raw log: `whole-crates-68d7109df.log`; counts: `final-whole-crate-counts.json`. Banker/nonbank regressions both pass. `clean-fmt.log` is exit 0; changed routing/readability has no new finding, inherited unrelated merchant function-length/cache-naming debt retained.

Final native extension/CLI build installed in 44.8 seconds. Trace-free ELF SHA-256: `75185d78622d63fd6da3cab0b860ed207cb6bf52317a50e5d95e35e9e35fa9e0`. Seven of nine allowed resumed rendered launches used; two clients only for concurrent account isolation. All resumed clients exited; exact owned server/Weston PIDs terminated; `agents-bankloop.slice` stopped and inactive; UDP 5360 verified free (`cleanup-owned-pids.json`, `cleanup-udp.txt`, `cleanup-slice.txt`). Private redb, evidence and caches retained. No merge/push, no data/UID/PLAN commit, scratch PLAN untouched. Later documentation-only commits do not invalidate this source proof. Rows remain Partial for the explicitly recorded protocol/UI gaps, not an unfinished client fix.

## How it works
- [banks](../wiki/systems/banks.md)

## Implementation inventory
| File | Role |
|---|---|
| `godot/ui-model/src/game/bank_data.rs` | `BankState`, `BankRequest`, `BankPrompt` |
| `src/game/networking/bank.rs` | Receives contents and failures, follows the interaction, sends requests, IPC status |
| `src/scenes/bank_frame/{mod,view,actions}.rs` | Screen, windows, clicks, edit boxes, confirmation popup |
| `godot/ui-model/src/ui/screens/bank_frame_component.rs` | Retail BankFrame layout |
| `godot/ui-model/src/ui/screens/bank_art.rs` | Slots, labels, money entry, checkbox shared with the guild bank |
| `src/scenes/bag_frame/mod.rs` | Right-click deposit (`use_bag_item`) |

## Tests asserting this spec
- `godot/rust/src/merchant.rs`: `distant_banker_attempt_reaches_authoritative_range_validation` and `non_banker_right_click_requires_a_living_npc_in_range`; live event/server/render assertion retained as `bankloop-2026-10-08/assert-bank-range.py` with RED/GREEN capture inputs.
- `godot/ui-model/tests/forever_bank_bags.rs`: concrete Modern/Forever atlas regions, Forever divider/item chrome and unchanged slot actions, 1596-line byte-identical Modern bank fixture.
- `godot/ui-model/tests/bag_window.rs`: under both skins the open backpack has its border, title, a close button that closes it, and one art-backed slot background per slot inside the window; Forever slot art differs from Modern.
- `godot/ui-model/src/game/bank_data_tests.rs`
- `src/game/networking/bank_tests.rs`
- `src/scenes/bank_frame/tests.rs`
- `godot/ui-model/src/ui/screens/bank_frame_component_tests.rs`
- `src/scenes/bag_frame/mod.rs` (`right_clicking_a_bag_item_deposits_into_the_open_bank_tab`)

## Known gaps (current cycle)
- [ ] Camelot bank-bag template art: `bank-frame-bag-slotframe`, `bankslot-icon-lock`, `bank-frame-bag-slot-bg` (`Camelot/BankFrame.xml:5,11,14`; F members:18120,18122,18119) resolves on sheet 8188339 but is not drawn: existing `BankFrameState` has no bank-bag slots, slot-purchase/lock or cost state. BagText/BagCost labels likewise remain unimplemented. Purchased bank-page side tabs are not bank bags; inventing state or behavior is outside this art-only pass.
- [ ] Selected-tab highlight: Retail blends `CheckButtonHilight` additively (ADD). The ui-toolkit texture renderer has no additive blending, so a gold-tinted UI-Quickslot2 marks the selected tab instead.
- [ ] Background and edge shadows are stretched, not tiled (the ui-toolkit has no tiling attribute for FDID textures); the edge shadow atlases are not drawn.

## Out of scope
- Search box, Cleanup/sort, the tab icon picker and the expansion filter (deferred by decision).
- Bank-origin drag, partial-stack transfer, bank-slot swapping and direct inter-tab moves (not represented by the pinned bank protocol). Native whole-bag-stack drag deposit is implemented and has bounded live proof above; it does not add those missing operations.
