# Native trainer frame

Native `ClassTrainerFrame` in `godot/ui-model/src/trainer*.rs` and `godot/rust/src/{trainer,tooltip_sources}.rs`, opened by the existing NPC gossip trainer role. [Professions contract](professions-frame.md) retains historical Retail requirements and recorded user decisions; retired-client checkboxes are not native proof.

## What it must do

- [ ] A concrete `TrainerList` opens the trainer; closing sends `CloseInteraction`, and matching `InteractionClosed` or world reset closes it.
- [x] Available / Unavailable / Used filters project server states without modifying them; selection remains visible.
- [x] Services display catalog names/icons, cost, level, skill rank and prerequisite abilities. Known services show Already known without cost. Learned relevant skill ranks come from `ProfessionSnapshot`.
- [x] Train sends exactly one `TrainerBuySpell` for an available affordable selected service, waits for authority, and refreshes service state from the next list and money from replicated Gold.
- [x] Adding a primary profession requires Accept / Cancel and a free primary slot; changing selection or closing cancels confirmation.
- [x] `TrainerBuyFailed` displays its reason without optimistic spending. Both Modern and Forever use the shared window chrome and same decisions.
- [x] Three-denomination selected and unselected prices keep every coin inside the row/scroll clip and above selection art.
- [x] Hovering any visible service row or descendant shows that service's shared catalog spell/recipe tooltip in both skins, not the selected service; Retail `ANCHOR_RIGHT` +35 and the requested grey Spell ID line remain. Leaving, filtering it away or closing removes the tooltip.

## Retail references

Under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.lua:58-89`: filter toggles and Available / Unavailable / Used checkboxes.
- Same file `:187-307`: service projection, requirement lines, known-state text, cost and enabled Train decision.
- Same file `:11-33`, `:400-406`: profession popup and purchase action.
- `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.xml:28-105`: service icon, name, subtext, cost, selection.
- `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.lua:156-172`: learned rank text.
- Refusals use existing protocol `TrainerFailReason::message` (NotEnoughMoney → "You don't have enough money."); Unavailable has no dedicated Retail string and is explicitly shown as "This service is unavailable."

## How it works

- [Native professions](professions-frame.md#native-godot-coverage-2026-10-07): shared catalogs and owner snapshot.
- [Live execution](../headless-live-run.md): isolated private server and rendered proof.

## Implementation inventory

- `godot/ui-model/src/trainer.rs`: list/filter/selection/request/confirmation/refusal decisions.
- `godot/ui-model/src/trainer_frame.rs`: skinned window, service list, filters, rank, money and confirmation.
- `godot/ui-model/src/professions_catalog.rs`: primary profession classification from SkillLine.
- `godot/rust/src/trainer.rs`: NPC-driven native UI, Gold and snapshot projection, TrainerChannel sends.
- `godot/rust/src/{account,merchant,professions,window_stack,lib}.rs`, `godot/network/src/lib.rs`: existing consumer/lifecycle integration.

## Tests asserting this spec

- `godot/ui-model/tests/trainer.rs`: concrete list, filters, exact once request, authoritative refresh, failure text, confirmation, disabled services and both-skin rendered registry projection.
- `godot/ui-model/tests/trainer_tooltips.rs`: row/descendant and current-visible-list hover decisions, shared spell content/placement/ID and both-skin tooltip rendering.
- `godot/tests/trainer_coin_pixels.gd`, `trainer_followup.gd`: native coin pixel/geometry regressions and offline hovered/leave captures in both skins; current results below.
- Native/private evidence is appended to canonical `data/diagnostics/trainerframe-2026-10-07/proof-ledger.txt` (verified 2026-10-07): nine targeted tests and matching extension/CLI build pass at `57f3e2286`; changed-file formatting passes. No broad-suite claim.
- Inspected Modern `shots/01-open.png`, `03-confirm.png`, `04-trained.png` at `263c9f461`: real Pomeroy gossip, first-profession confirmation, Train purchase, money 10000 → 9990 copper and Herbalism Already known. Model/bridge purchase scope is unchanged by the presentation-only follow-up.
- Inspected Forever `forever-shots/01-open.png`, `02-available-off.png`, `03-used-off.png`, `04-restored.png` at `57f3e2286`: persisted 9990 copper/known state, catalog icons, native pointer-driven Available and Used removal/restoration, root profession flag not displayed as 1/0. `state.txt` and `client4-live.log` retain assertions; client exit 0. Four launches total against an approved `f25258b` server with a fresh offline redb copy and read-only world.db, UDP5304/Weston tf24.
- Earlier filter capture was a genuine native failure: overlapping list controls intercepted popup input. The final screen emits filter controls after the list; final live captures verify the fix. Missing icon 4620675 was extracted from local CASC into canonical textures, never CDN.

## Visual parity audit (2026-10-07)

Cached **Mainline** source governs; historical professions checkboxes are not native acceptance. No trainer-specific recorded user-requested art/layout deviation was found in this spec or the linked professions contract. Preserve authoritative training, confirmation, filters and failure text.

`TUI.xml` / `TUI.lua` below mean cached `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.{xml,lua}`; `Shared.xml` / `Shared.lua` mean `Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.{xml,lua}`.

| Baseline native deviation (at b844d8033) | Cached Retail contract |
| --- | --- |
| `trainer_frame.rs:33-46,67-68`: 420×540 with an invented greeting pane | `Shared.xml:544-572,684-692`: inherited 338×424 portrait window/inset. `TUI.lua:65-66`: NPC portrait and NPC-name title. No greeting/detail pane in `TUI.xml:108-239`; requirements belong to rows. |
| `trainer_frame.rs:115-139,150-174`: 390-wide list at15,112, rows364×66, name with selection prefix and costs below requirements | `TUI.xml:28-60,201-214`, `TUI.lua:47-51,146-152`: 302×330 list at inset TOPLEFT5,-5, 298×47 rows, 1px initial padding, icon36×36 at LEFT6, name iconTOPRIGHT6,-1, subtext240×30, cost TOPRIGHT5,-7. |
| `trainer_frame.rs:143-151`: green/red/grey service names | `TUI.lua:244-266`: unavailable icons desaturated and MOD0.55 background; the temporary grey name is overwritten by unconditional `name:SetText(serviceName)` at265. All names retain inherited GameFontNormal. State colours apply to filter labels (`:86-88`), not service names. |
| `trainer_frame.rs:152-185`: missing normal/selected/hover trainer art and coin icons; zero-cost services show a cost | `TUI.xml:63-95`: TrainerTextures crops for normal/selected/hover. `TUI.lua:259-282`: known and zero cost hide money; otherwise SmallMoneyFrame, white or red affordability. |
| `trainer_frame.rs:156-166`: uniformly white requirements, no colon | `TUI.lua:208-266`: Requires: prefix; independently coloured level number, skill-rank number and ability. `data/GlobalStrings.csv:284-288,2054`: unmet #ff2020, met #ffffff, skill name/parentheses in SystemFont_Shadow_Small. |
| `trainer_frame.rs:71-89`: generic Filter button at300,82 and literal [x]/[ ] rows | `TUI.xml:177-181`, `TUI.lua:54,84-89`: width100, top-right−13,−35, WowStyle1FilterDropdownTemplate and three coloured checkboxes. `Blizzard_Menu/Mainline/MenuTemplates.xml:66-105`: height18, common-dropdown-b-button. |
| `trainer_frame.rs:92-115`: rank text at64,82 without a bar | `TUI.xml:127-176`, `TUI.lua:156-172`: 136×18 at64,36, GuildFrame border, blue background/fill, centred learned rank/max. |
| `trainer_frame.rs:48-65`: Money: text and generic100×24 Train at300,450 | `TUI.xml:112-118,182-194`: UI-MoneyFrame-Border148×34 at bottom-left5,−9; SmallMoneyFrame anchored to its RIGHT8,6; MagicButton80×22. `Shared.lua:35-38`: bottom-right adjusted−6,+4. |
| No separate SkillStepButton in native inventory | `TUI.xml:195-200,215-223`, `TUI.lua:131-144`: optional316×40 step row and shorter bottom-inset list. Protocol provides no Retail GetTrainerServiceStepIndex; do not infer step identity from primary-profession acquisition. Remains a gap. |

Additional cached Retail details: `Blizzard_Menu/Mainline/MenuVariants.lua:9-22` uses the 12×12 `common-dropdown-ticksquare` and 15×14 yellow check, offset +2,+1, with text 7px to its right. `Blizzard_Menu/Mainline/MenuTemplates.lua:53-85` supplies `common-dropdown-bg` at alpha0.925, 8/8/8/15 insets and 20px child width padding. The popup anchor's +2 WoW y is upward (`DropdownButton.lua:90-95`). `Blizzard_MoneyFrame/Shared/MoneyFrame.lua:33-72` distinguishes PLAYER (lower coins shown) from STATIC (collapsed); `Mainline/MoneyFrame.lua:302-368` includes zero lower denominations for PLAYER. Native player money now uses that distinction without altering other windows' existing collapsed money helpers.

State colour values come from `data/db2/12.1.0.69933/GlobalColor.csv:5,9-10`: RED_FONT_COLOR=#ff2020, GREEN_FONT_COLOR=#19ff19, GRAY_FONT_COLOR=#808080. No invented state palette.

## Native visual implementation

Presentation uses 338×424 shared portrait chrome, the trainer/inset/row crops, 298×47 rows with top/right money, independent #ff2020 unmet requirement numbers/abilities, desaturated unavailable icons, selected/additive hover art, blue learned-rank fill, a 100×18 skinned filter and 80×22 Train. NPC portrait rendering reuses the existing masked unit-portrait host; title remains the NPC name. Training/filter/confirmation authority is unchanged. Exact UI acceptance is recorded below only after tests/captures pass.

Offline secondary preview API: `godot/rust/src/trainer_preview.rs`; an eight-service snapshot plus a representative human portrait, never a GameClient. `GODOT_TRAINER_FILTER_MENU=1` opens the popup for a separate visual capture; main captures leave it closed so row costs remain visible. Baseline captures used the original open-popup fixture. This preview does not prove the real NPC appearance or live lifecycle.

## Offline visual acceptance (2026-10-07)

Production Rust at `ec76c939b`; capture assertions at `7a17cf3e6` (no Rust change). Targeted helper: **16/16** pass (nine original trainer behavior cases, seven content/art cases), `/tmp/claude/trainerart-final-green.out`. Whole Godot workspace `cargo fmt --all -- --check` passes. Locked local `--cli` build/install passes; extension SHA-256 `6ef65c399981c5b41a25b20d787a75bb20bae3fa40473a88c1ef989e9c1a1881`, `/tmp/claude/trainerart-final-build.out`. Full-crate integration results follow below; targeted counts alone are not broad-suite proof.

Inspected canonical `data/diagnostics/trainerart-2026-10-07/{before-modern,before-forever,after-modern,after-forever}.png` and cropped window views. Before: oversized 420×540 window, empty portrait, greeting, tinted/prefixed names, text-only money and untextured rows. After: inherited 338×424 chrome, real masked representative-human portrait, NPC-name title, trainer background/rows, 36px icons, neutral #ffd200 names, selected/additive art, independently coloured unmet level/rank numbers, known text without cost, coin prices/wallet, blue35/300 bar, 80×22 Train and 100×18 Filter. Both final main captures exit0 and pass native geometry/content/colour and settled portrait-model/mask assertions. Authored half-pixel icon/name y5.5/6.5 rasterize to5/7; tests retain exact dimensions/x anchors and allow at most0.5px per half-pixel y anchor.

Popup captures use the same fixture with `GODOT_TRAINER_FILTER_MENU=1`: Retail dropdown/background/ticksquare/checkmark members and #19ff19/#ff2020/#808080 labels; main captures leave the popup closed so prices remain visible. The eight-row fixture overflows the330px list, with available/unavailable/used, selected2963, profession3908, costs10/12550/500/0 and wallet12345. Purchase/confirmation authority is unchanged; these offline images do not re-prove live purchasing or the real NPC's appearance. RID/ObjectDB/font shutdown diagnostics and test-only Dozen/Wayland warnings remain, not clean-resource/general-renderer proof.

### Full affected-crate integration

The locked local helper ran `--test -p game-engine-ui-model -p game-engine-godot -p game-engine-core --no-fail-fast` **once** after final relevant Rust changes; exit0. Deduplicated detailed log counts: **core781 passed**, **Godot651 passed**, **UI-model751 passed /6 ignored**, **0 failures** (2183 passed total). No ignored tests were introduced here. Evidence: canonical `data/diagnostics/trainerart-2026-10-07/{full-details.log,full-counts.json,proof-ledger.txt}`; command and wrapper output `/tmp/claude/trainerart-full.out`. Subsequent capture-assertion/docs changes do not invalidate this CPU scope; no suite rerun.

## Trainer follow-up (2026-10-07)

The selected three-denomination copper icon crossed the row border because the native flattened money renderer treated the SmallMoneyFrame's right edge as the coin edge. Cached `Blizzard_MoneyFrame/Mainline/MoneyFrame.xml` anchors CopperButton RIGHT at -13; `MoneyFrame.lua:378-380` reapplies that inset. The trainer money frame itself remains TOPRIGHT +5 (`TUI.xml:28-39`). Restore the internal 13px inset and include it in the name's width budget, rather than inventing a row offset. Money is a child frame above parent selection/highlight layers, as in Retail. Native pixel RED at baseline changes 20 opaque copper pixels when the selected layer is hidden (`trainer_coin_pixels.gd`); Native pixel GREEN in both skins moves the copper rect from `(324,215,13,13)` to `(311,215,13,13)` and reduces selection-induced opaque-pixel changes from **20 to 0**. Fix `7c8381e07`; test `trainer_coin_pixels.gd`.

Service hover now registers in the existing native `tooltip_sources::frame_tooltip` dispatcher and reuses `spell_game_tooltip`, `GameTooltipUI`, skin styling, placement and ID appending. `TrainerBook` resolves the row's nearest action ancestor against the current visible services, independent of selection/availability/affordability; the shared owner anchor retains Retail's +35px offset. Existing per-frame source resolution refreshes while hovered and removes stale tooltips on leave/filter/close. The offline preview uses the same source decisions and renderer, with local catalog content and no GameClient/server.

No new recipe-content renderer is introduced: the existing spell source supplies title, subtext, cost/range/cast/cooldown and rendered description where present. Pinned Spell2963/2964 descriptions are empty; crafted-output item properties and reagent sections are not supplied by this shared spell renderer. Cached Lua calls C++ `SetTrainerService` but does not specify its complete line content, so full C++ recipe-tooltip parity is not claimed. The user-requested Spell ID line is preserved ([tooltip contract](unit-tooltip.md)).

SkillStep and rank modifiers are unchanged. The authoritative missing values are `GetTrainerServiceStepIndex()`'s optional service index (not the primary-profession acquisition flag) and `GetTrainerTradeskillRankValues()`'s `rankModifier` (`TUI.lua:180-190`). Existing `TrainerService` requirements and `ProfessionSkillLine` base rank/max/step do not supply those values.

### Follow-up acceptance

Tooltip source `2e8e71564`; preview lifecycle correction `2bd450ae4` initializes the shared host once then updates state, as production `sync_game_tooltip` does. Targeted **18/18** pass: original trainer9, art7, hover/content2; UI-model source is unchanged by preview-only fixes. Whole-workspace format proof at `8eb890930` plus changed-preview-file check at `2bd450ae4` pass; matching locked local extension/CLI builds pass. Evidence: canonical `data/diagnostics/trainertips-2026-10-07/proof-ledger.txt`, `green-details.log`, `/tmp/claude/trainertips-{green,build-refresh}.out`.

At `2bd450ae4`, `trainer_followup.gd` captures and asserts **six** offline snapshots in one renderer: `modern-{default,hovered,selected}.png`, `forever-{default,hovered,selected}.png` under that evidence directory. Every gold/silver/copper icon is 13×13, inside the scroll clip and at least8px inside the row's right edge, for selected Linen and unselected Wool prices. Both hovered shots show the shared Bolt of Linen Cloth tooltip to the right, **1.5 sec cast** from the local catalog and the grey **Spell ID: 2963** line; pointer leave hides it. Default shots select Tailoring, with both Linen/Wool three-coin prices unselected; selected shots show Bolt's complete red1g25s50c and disabled Train. Modern dark stone and Forever bronze chrome, representative portrait,35/300 rank and known-service text remain. All six PNGs were inspected; `capture-observations.json` and `captures-refresh.log` retain details. Existing RID/ObjectDB/font shutdown warnings remain; this is not clean-resource or real-NPC/live lifecycle proof.

Full required crate integration is recorded below after completion; targeted/runtime results are not a substitute for its counts.

## Known gaps (current cycle)

- [ ] Native matching `InteractionClosed`, world-reset and Escape network-close receipt lack dedicated behavioral integration assertions; close/request code is wired, but do not infer full lifecycle acceptance from purchase proof.
- [ ] Client exit logs contain texture/RID/ObjectDB leak warnings; exit 0 is not clean-resource shutdown proof.
- [ ] Conditional SkillStepButton/split-inset layout lacks a supplied Retail step index; rank modifiers remain unsupported/unverified. Complete C++ recipe-tooltip output/reagent line parity remains unverified; shared service-hover proof is tracked above. Do not infer the step from `TrainerService.profession` (primary-slot acquisition). Default-window geometry/art, portrait slot/mask, base-rank fill and per-requirement colours have bounded offline acceptance above, not pixel-identical or live-NPC proof. Existing native scroll-input decisions are unchanged; only list/scrollbar geometry is matched. No user-requested deviation is removed or invented.

## Out of scope

- Server/protocol changes, reputation discounts, additional trainer mechanics, merge and push are excluded by this task.
