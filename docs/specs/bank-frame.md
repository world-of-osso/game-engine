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
  - The selected tab draws `Interface/Buttons/CheckButtonHilight` (130724), 32×32 centred, white with ADD blending, on OVERLAY. Side-tab border and icon art remain unchanged.
  - The purchase tab (`bags-icon-addslots`) comes last while a tab can be bought.
- [x] Header: the tab name in a 300×20 box at TOP −36.
- [x] Bottom tabs `Bank` / `Warband Bank` hang at the frame's BOTTOMLEFT 22,2 (PanelTab art).
- [x] Purchase prompt: while the purchase tab is selected, or while the bank has no tabs, the prompt replaces the slots.
  - It shows the title and `CHARACTER_BANK_TAB_PURCHASE_PROMPT` / `ACCOUNT_BANK_TAB_PURCHASE_PROMPT`.
  - Below that, "Cost:" and the Retail `BankTab` price, then a 105×21 Purchase button.
  - Purchase is disabled when the player can't afford the tab. Money digits are Retail red (1,0.1,0.1,1), not grey; coin art stays unchanged.
- [x] Purchase opens `CONFIRM_BUY_*_BANK_TAB` ("Do you want to purchase a Warband Bank tab for:\n1000g"). Accepting it sends `BankPurchaseTab`.
- [x] Warband bank only: the money frame, 394×25 at BOTTOMRIGHT −3,3, with ThinGoldEdge 178×19, the stored money, and Withdraw and Deposit buttons (105×21).
- [x] Deposit All button (256×24): `Deposit All Reagents` in the character bank, `Deposit All Warbound Items` in the Warband bank. The Warband bank also shows the "Include tradeable reagents" checkbox.

### Skin art
- Dedicated bank textures resolve Blizzard atlas element names through `ui_toolkit::atlas::resolve_region`; Modern keeps pre-conversion FDIDs and UVs; portrait binding and background exclusion apply to both skins.
- Forever uses `bank-frame-background`, `bags-item-bankslot64` for character slots, `bank-frame-item-slotframe` over item icons, without a divider. Retail Mainline `BankFrame.xml:600-738` defines no divider in either bank tab; neither skin adds the Camelot-only frame. Warband slot art and purchase-tab art retain their set-0 members. The tinted purchase-prompt background resolves `bags-item-slot64`.
- Camelot art does not change the 98-slot grid, slot actions, tabs or purchase/money state. Its 88-slot page cap and uniform spacing are behavior, excluded from this art conversion.
- Standalone container windows are Retail `ContainerFrameTemplate` (Retail/Forever Mainline ContainerFrame.xml:218-265, ContainerFrame.lua:9-12,779-874): 178 wide, 37×37 slots 5 apart in a `BottomRightToTopLeft` grid from BOTTOMRIGHT (-7, 9), `PortraitFrameFlatTemplate` chrome (flat background, portrait metal border, title at left 35, close button that closes only that bag), `bags-item-slot64` under every slot through the active skin. Not drawn: portrait icon (no texture masks in rsx), backpack search box/sort button/money frame rows.

Source roots: Retail / Forever = `~/.cache/wow-ui-sim/blizzard-ui/{retail,wowforever}/AddOns/Blizzard_UIPanels_Game/`. M / F = `data/db2/{12.1.0.69933,1.60.1.69913}/UiTextureAtlasMember.csv`. Atlas sheet FDIDs come from those builds' `UiTextureAtlas.csv`.

| Name | UI source | Member rows M / F | Modern / Forever FDID |
|---|---|---|---|
| `bank-frame-background` | Retail `Mainline/BankFrame.xml:677` | 11422 / 17833 | 5782252 / 8118796 |
| `bags-item-slot64`; `bags-item-bankslot64` | Retail `Mainline/BankFrame.xml:571`, `.lua:586`; Forever `Camelot/BankFrame.xml:36` | 7767 / 18117; bankslot F:17832 | 4701874 / 8187737; Forever character slots 8118792 |
| `warband-bank-slot` | Retail `Mainline/BankFrame.lua:582` | 11547 / 11565 (set 0) | 5782246 / 5782246 |
| `bags-icon-addslots` | Existing purchase-tab crop, M/F member:2883 (no BankFrame XML/Lua name literal) | 2883 / 2883 | 969828 / 969828 |
| `bank-frame-item-slotframe` | Forever `Camelot/BankFrame.xml:34` (slot art only) | absent / 18121 | absent / 8188339 |

Sheet rows M:84,1346,1772,1774; F:2616,2617,2689,2690. The removed Camelot divider used a 864×32 member at scale 0.48, producing (161.64,224.64,414.72,15.36) across the Retail grid (y=63..382). Retail has no divider anchor, size, draw layer or visibility branch; its money frame is separately anchored BOTTOMRIGHT −3,+3 (`BankFrame.xml:186-188,611-614`).

Local BLPs **8118796, 8118792, 8188339** are present (verified 2026-10-08); inspected `bankloop-2026-10-08/resume-forever-character.png` and `resume-alt-warband.png` render the Forever bank art. Resolution is not guarded or substituted when files are absent.

### Actions
- [x] Character-bank left-click/drag pickup and drop use the shared item cursor. The exact `{ tab, slot }` source survives selecting another bank tab; bags↔bank and same/across-tab destinations send the existing `SwapItem` (server handles swap/merge). Shift-click opens the existing bag StackSplitFrame; accepting places the chosen count on the cursor and dropping sends `SplitItem`. The server owns all stack changes; BankContents refreshes cursor lookup and closing clears bank-origin cursor/split state. Retail Mainline BankFrame.lua: HandleItemPickup, OnDragStart, OnReceiveDrag, OnModifiedClick, SplitStack (416-448, 590-592).
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

Historical bankloop scope only: no server bank defect was established in that run. At its pinned revisions, partial-stack/bank-to-bank operations were protocol limitations: `BankDeposit` named a whole bag stack; `BankWithdraw` named a whole bank slot. The later `bankmoves` change adds character-bank locations to `SwapItem` / `SplitItem`; the N-A item row above remains historical evidence, not the current character-bank contract. Modern/Forever remain skins of the same 98-slot mechanics; Camelot's 88-slot behavior and tooltip record-ID removal are not adopted.

### Final verification and cleanup

On `68d7109df` (production source byte-identical to `2acedbd17`, SHA-256 `70d5bbc5660bda77e1076b86f27c66b3e013350cfe0850fc9d0dee18c2f6bd98`), the required locked/local/pinned five-crate `cargo test --no-fail-fast` exits **0**: CLI **103**, core **781**, Godot **661**, network **66**, UI-model **754** passed; **2365 passed, 0 failed, 6 ignored fixture-generation tests**. Raw log: `whole-crates-68d7109df.log`; counts: `final-whole-crate-counts.json`. Banker/nonbank regressions both pass. `clean-fmt.log` is exit 0; changed routing/readability has no new finding, inherited unrelated merchant function-length/cache-naming debt retained.

Final native extension/CLI build installed in 44.8 seconds. Trace-free ELF SHA-256: `75185d78622d63fd6da3cab0b860ed207cb6bf52317a50e5d95e35e9e35fa9e0`. Seven of nine allowed resumed rendered launches used; two clients only for concurrent account isolation. All resumed clients exited; exact owned server/Weston PIDs terminated; `agents-bankloop.slice` stopped and inactive; UDP 5360 verified free (`cleanup-owned-pids.json`, `cleanup-udp.txt`, `cleanup-slice.txt`). Private redb, evidence and caches retained. No merge/push, no data/UID/PLAN commit, scratch PLAN untouched. Later documentation-only commits do not invalidate this source proof. Rows remain Partial for the explicitly recorded protocol/UI gaps, not an unfinished client fix.

### Character-bank moves — 2026-10-08

`bankmoves` adds character-bank pickup/drop, exact same/across-tab destinations and the shared split picker ([cursor contract](cursor-item.md)). `native_bank_moves` at `178e9dba3` passes 5/5 after RED 1 passed / 4 failed at `d0e42b5ac`; logs: `/home/osso/.worktrees/logs/bankmoves-engine-{red,green}.log`. This proves mounted slot routing and model request/state behavior. Native drag-release and shift-click dispatch are wired in `godot/rust/src/{bank,bag_cursor}.rs`, but no new native event, render or live proof is claimed. Whole-crate/integration gates remain with the lead.

### Live character-bank stack split — 2026-10-09

Matching engine source `43e72682f`, locked extension/CLI build exit0; one rendered native run on private UDP5520. Existing server admin `GrantItem` gave `fb_bankmoves5` / Bankfive real Linen Cloth2589×10 in fresh persistent inventory; no client inventory was fabricated. Evidence: `data/diagnostics/bankmoves5-2026-10-09/` (input/response ledgers and numbered snapshots); inspected PNGs `live5-*.png` under `/syncthing/AgentShared/2026-10-09/bankmoveslive/`, downscaled with ffmpeg.

- PASS bag Shift-click → StackSplitFrame4 → Okay → exact BankFrameItem7: bag6/bank4. Real withdrawal into bag slot3 proves two authoritative Linen stacks6 and4 (`16-authoritative-bag6-plus4.json`).
- PASS bank Shift-click after merging to10 → StackSplitFrame4 → Enter → exact BankFrameItem9: Item7 has6, Item9 has4. Real withdrawals into bag slots1/3 prove both IDs/counts, then redeposit restores6/4 (`24-bank6-bank4.json`, `25-authoritative-bank-withdrawals.json`, `26-final-split.json`). Retail Mainline `BankFrame.lua:439-443,590-592` owns modified-click/picker/SplitContainerItem behavior.
- The bank-origin picker at the leftmost column places Okay outside the viewport (x=-49.33,width42.67); that mouse attempt sent no split. Keyboard Enter proves acceptance without a layout fix. This run does not prove on-screen mouse confirmation for that anchor or full Retail pixel parity.
- PASS live UDP5520 closed-bank rejection: a headless NetworkBridge connection selects the real character without opening any banker, sends `SwapItem` from bank tab0/slot6 to bag0/slot1, and receives `InventoryError { reason: CantDoRightNow, detail: "no character bank open" }`. Exact inventory/equipment snapshots match after reconnect (`live-closed-protocol.txt`). A second rendered login opens the bank, confirms6/4, withdraws both real Linen stacks into bag slots1/3, then restores them (`29-post-protocol-bank.json`, `30-post-protocol-authoritative-withdrawals.json`, `31-post-protocol-final.json`; inspected `live5-09,10` PNGs). Server test `a_move_request_after_closing_the_bank_rejects_without_changing_inventory` at game-server `1360ce6` passes1/1 after a deliberate disabled-guard sensitivity RED; complete persisted inventory/bank equality is asserted. Two rendered runs plus one headless protocol run used. Runtime server binary matches the immutable live4 hash; its source revision is not independently stamped.

### Bounded bankart pass — 2026-10-08

Source root `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`; BF = `Blizzard_UIPanels_Game/Mainline/BankFrame.xml`. Forever shares the Mainline templates and substitutes only skin members already specified above.

| Element | Source citation | Art, rect and flags |
|---|---|---|
| Selected tab | BF:349–376 | `CheckButtonHilight`, FDID130724, (0,0,32,32), no tiling, ADD on OVERLAY; not the old gold Quickslot substitute. |
| Frame background | BF:677–682; Forever `Mainline/BankFrameTemplates.xml:671` | `bank-frame-background` active skin; retained portrait-excluding rect (2,51,734,379); horizontal + vertical tile, normal alpha. Native repetition samples only the cropped member, not its whole atlas sheet. |
| Corner shadows | BF:279–304 | `bank-frame-shadow-corner{topleft,topright,bottomleft,bottomright}`, 46×46 at (2,22)/(689,22)/(2,412)/(689,412); no tile, normal alpha. |
| Edge shadows | BF:306–340 | `!bank-frame-vert-shadow`, 17×344 at (2,68)/(718,68), vertical tile; `_bank-frame-horiz-shadow`, 641×17 at (48,22)/(48,441), horizontal tile; normal alpha. XML local crops select left/right or top/bottom halves of the member, preserving 256px repeat periods. |
| Item hover | `Blizzard_ItemButton/Shared/ItemButtonTemplate.xml:79` (Mainline:138 for giant variant) | `Interface/Buttons/ButtonHilight-Square`, FDID130718, 37×37 icon bounds, white/ADD, no tile. Native bank slot retains press-position, drag and right/Shift click routing; no default button skin. |
| Unaffordable price | `Blizzard_UIPanels_Game/Mainline/BankFrame.lua:1081–1086`; `Blizzard_MoneyFrame/Mainline/MoneyFrame.lua` color lookup | `SetMoneyFrameColorByFrame(..., canAfford and "white" or "red")`; red digits (1,0.1,0.1,1), normal alpha, existing coin members/rects unchanged. |
| Bank-bag chrome (blocked) | Forever `Camelot/BankFrame.xml:4–16,62–112`; `.lua:135–190,398–410` | `bank-frame-bag-slotframe`, `bank-frame-bag-slot-bg`, `bankslot-icon-lock`, no tiling, normal alpha; template scale .75. Bag highlight `CheckButtonHilight` is ADD; BagText/BagCost/MoneyDisplay anchor to the bag row. These are not Retail Mainline side-tab art. Existing layout/authoritative bag state cannot represent this row without the excluded Camelot behavior or a new placement decision. |

RED revision `58fded147`: `/home/osso/.worktrees/logs/bankart-red.log`, **0 passed / 5 failed**, EXIT101. All failures reproduce missing art/flags or grey price, not compilation errors. Stronger model-owned hover RED at `f97c2b1be`: `bankart-hover-red.log`, **0 passed / 1 failed**, EXIT101 (missing additive overlay texture).

Final production revision `808b6eb2e`: `bankart-final-green-restored.log`, **11 passed / 0 failed / 1 ignored**, EXIT0 (art5, skin1, bankmoves5). `bankart-build-final.log` installs the native extension, EXIT0; ELF SHA256 `c1e4a8b4ec0d43cf0507265975178791f2176c618d045ed0d40db21d718f755b`. Earlier native build failed because the helper omits project `.gdshader` files; `d48d0ac25` embeds the shader in shipped Rust, without a fallback. A queued test failed before compilation when the dependency worktree was removed; the required environment path now aliases clean canonical protocol at the identical `c139baa` revision, with no protocol changes.

Evidence: `data/diagnostics/bankart-2026-10-08/`. `capture.py` reuses spellbookshot's cage headless-mode shim; `capture.log` exits0 and asserts actual **1920×1080** window and viewport/image dimensions. All six `{modern,forever}-{tabs,hover,purchase}.png` were read. Grey Modern stone and warm Forever background repeat instead of stretching; each skin retains its slot/chrome/divider art. Selected tabs glow at32×32; the first37×37 slot gains a blue-white additive hover without losing its20 count. Both purchase captures show red500g digits and a disabled grey Purchase button. Native input additionally proves exactly one left/Shift/right bank-slot press, with left press coordinates retained. This is offline projection/input proof, not new server/live acceptance or full Retail pixel equivalence.

| Element | Model proof | Native proof / limit |
|---|---|---|
| Selected marker | PASS FDID130724,32×32,WHITE,ADD,no tile | PASS both skins' purchased and purchase tabs inspected |
| Tiled background | PASS active-skin member, retained rect, both tile flags | PASS both skin patterns visibly repeat at true1920×1080 |
| Edge shadows | PASS all corner/edge atlases, rects and tile axes | PARTIAL corners/top/bottom render; vertical member FDID5779392 absent locally |
| Unaffordable price | PASS all denomination digits red | PASS red500g and disabled button in both skins |
| Bank-bag chrome/lock/cost labels | BLOCKED authoritative bag state and compatible placement absent | NOT IMPLEMENTED / NOT CAPTURED; purchase screenshots are bank-tab prompts, not bag purchases |
| Additive item hover | PASS mounted ADD texture, hover show/hide and parent-closure visibility/alpha | PASS loaded ADD material, actual mouse hover and both-skin inspected captures |

Local-CASC extraction of5779392 failed because `/syncthing/World of Warcraft/.build.info` is absent despite local archives and `.product.db`; no usable `/mnt/c` install or alternative local asset was found. `extract-5779392.log` is EXIT1 and native capture logs the missing texture. No CDN, substitute shadow or fabricated metadata was used. Import exited0 with8 ObjectDB warnings; capture exited0 without that shutdown warning, but platform warnings and the missing-art error remain recorded. Bank-bag art and complete vertical-shadow rendering remain open.

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
- `godot/ui-model/tests/native_bank_moves.rs`: mounted bank-slot routing, cross-tab cursor identity, exact bag/bank move and split requests, authoritative refresh and closed/Warband/purchase rejection.
- `godot/rust/src/merchant.rs`: `distant_banker_attempt_reaches_authoritative_range_validation` and `non_banker_right_click_requires_a_living_npc_in_range`; live event/server/render assertion retained as `bankloop-2026-10-08/assert-bank-range.py` with RED/GREEN capture inputs.
- `godot/ui-model/tests/forever_bank_bags.rs`: concrete Modern/Forever atlas regions, Forever divider/item chrome, 98-slot geometry and unchanged slot actions. The historical full-tree fixture is no longer an invariant: this pass intentionally changes Modern marker, shadows, background flags and price colors.
- `godot/ui-model/tests/native_bank_art.rs`: both-skin marker FDID/rect/ADD, skin background atlas/rect/tiling, shadow atlas/rect/axis tiling, red unaffordable money and item-hover source/size/action.
- `godot/ui-model/tests/bag_window.rs`: under both skins the open backpack has its border, title, a close button that closes it, and one art-backed slot background per slot inside the window; Forever slot art differs from Modern.
- `godot/ui-model/src/game/bank_data_tests.rs`
- `src/game/networking/bank_tests.rs`
- `src/scenes/bank_frame/tests.rs`
- `godot/ui-model/src/ui/screens/bank_frame_component_tests.rs`
- `src/scenes/bag_frame/mod.rs` (`right_clicking_a_bag_item_deposits_into_the_open_bank_tab`)

## Known gaps (current cycle)
- [ ] Camelot bank-bag template art: `bank-frame-bag-slotframe`, `bankslot-icon-lock`, `bank-frame-bag-slot-bg` (`Camelot/BankFrame.xml:5,11,14`; F members:18120,18122,18119) resolves on sheet 8188339 but is not drawn: existing `BankFrameState` has no bank-bag slots, slot-purchase/lock or cost state. BagText/BagCost labels likewise remain unimplemented. Purchased bank-page side tabs are not bank bags; inventing state or behavior is outside this art-only pass.
- Bank-bag art remains blocked by the excluded Camelot bag/page behavior, not missing atlases: Retail Mainline BankFrame has bank tabs but no bank-bag row; Camelot has bag inventory locations and 88 slots/page. Its BagText at BOTTOMLEFT (43,80), scaled bag buttons and cost controls overlap the retained 98-slot grid and Deposit All. BankContents has tab icons, not equipped bank-bag items. No invented bag inventory, purchase request or geometry is introduced by the art fix.

## Out of scope
- Search box, Cleanup/sort, the tab icon picker and the expansion filter (deferred by decision).
- Warband/guild-bank exact-slot drag, split and inter-tab moves. Character-bank support uses ItemLocation::Bank and existing SwapItem/SplitItem. Targeted model/UI tests are in `godot/ui-model/tests/native_bank_moves.rs`; live runs are excluded from this `bankmoves` proof.
