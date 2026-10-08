# Native trainer frame

Native `ClassTrainerFrame` in `godot/ui-model/src/trainer*.rs` and `godot/rust/src/trainer.rs`, opened by the existing NPC gossip trainer role. [Professions contract](professions-frame.md) retains historical Retail requirements and recorded user decisions; retired-client checkboxes are not native proof.

## What it must do

- [ ] A concrete `TrainerList` opens the trainer; closing sends `CloseInteraction`, and matching `InteractionClosed` or world reset closes it.
- [x] Available / Unavailable / Used filters project server states without modifying them; selection remains visible.
- [x] Services display catalog names/icons, cost, level, skill rank and prerequisite abilities. Known services show Already known without cost. Learned relevant skill ranks come from `ProfessionSnapshot`.
- [x] Train sends exactly one `TrainerBuySpell` for an available affordable selected service, waits for authority, and refreshes service state from the next list and money from replicated Gold.
- [x] Adding a primary profession requires Accept / Cancel and a free primary slot; changing selection or closing cancels confirmation.
- [x] `TrainerBuyFailed` displays its reason without optimistic spending. Both Modern and Forever use the shared window chrome and same decisions.

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

State colour values come from `data/db2/12.1.0.69933/GlobalColor.csv:5,9-10`: RED_FONT_COLOR=#ff2020, GREEN_FONT_COLOR=#19ff19, GRAY_FONT_COLOR=#808080. No invented state palette.

## Known gaps (current cycle)

- [ ] Native matching `InteractionClosed`, world-reset and Escape network-close receipt lack dedicated behavioral integration assertions; close/request code is wired, but do not infer full lifecycle acceptance from purchase proof.
- [ ] Client exit logs contain texture/RID/ObjectDB leak warnings; exit 0 is not clean-resource shutdown proof.
- [ ] Exact Retail 338×424 geometry, trainer-specific background/row art, portrait, rank fill, per-requirement colouring, tooltip and SkillStepButton parity remain unverified. No user-requested deviation is removed or invented.

## Out of scope

- Server/protocol changes, reputation discounts, additional trainer mechanics, merge and push are excluded by this task.
