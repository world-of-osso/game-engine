# Professions Frame

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

The Retail profession trainer, ProfessionsBook and ProfessionsFrame running against the live server. The contract is shared-protocol `profession.rs`, `protocol/trainer_messages.rs` (`TrainerList`, `TrainerBuySpell`, `TrainerBuyFailed`), `ProfessionSnapshot` and `CraftRecipe`; server rules are in game-server `docs/specs/professions.md`.

References (all under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`):
- TUI.xml / TUI.lua = `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.xml` / `.lua`
- PB.xml / PB.lua = `Blizzard_ProfessionsBook/Blizzard_ProfessionsBook.xml` / `.lua`
- PF.xml = `Blizzard_Professions/Blizzard_ProfessionsFrame.xml`; PC.xml / PC.lua = `Blizzard_ProfessionsCrafting.xml` / `.lua`
- RL.xml / RL.lua = `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeList.xml` / `.lua`; SF.xml = `Blizzard_ProfessionsRecipeSchematicForm.xml`
- strings: wow-ui-sim `data/global_strings_wowforever.rs` (build 12.1); colours: GlobalColor DB2 (`data/db2/12.1.0.69933/GlobalColor.csv`)
- atlas crops: `data/UiTextureAtlasMember.csv` (member id cited next to each constant)

## Data

- [x] Recipe data comes from the pinned DB2 CSVs (`src/game/professions_data.rs`): profession `SkillLine` rows (CategoryID 9/11) with `SpellBookSpellID`, `SkillLineAbility` rows with a `SkillupSkillLineID`, `SpellReagents`, the `SpellEffect` CREATE_ITEM (24) output, `TradeSkillCategory`, `SpellName`/`SpellMisc` names and icons, and `ItemSparse` names/qualities of the involved items. Cached in `data/cache/profession_catalog-<build>.bin`.
- [x] `ProfessionSnapshot` (learned lines and spells) is stored in `ProfessionStatusSnapshot` by its single reader (`networking::messages::receive_profession_snapshot`). Changes after the first snapshot post Retail chat lines: `ERR_SKILL_GAINED_S` "You have gained the %s skill.", `ERR_SKILL_UP_SI` "Your skill in %s has increased to %d." (tier lines) and `ERR_LEARN_RECIPE_S` "You have learned how to create a new item: %s."
- [x] Bag counts come from `InventoryState` (all bag stacks of the item).

## Trainer frame (`ClassTrainerFrame`)

- [x] `TrainerList` opens it as an NPC-driven Panel (slot L); closing it sends `CloseInteraction`; `InteractionClosed` for the trainer closes it. `TrainerList` again after a purchase refreshes it and keeps the selection and scroll position.
- [x] 338×424 `ButtonFrameTemplate` (TUI.xml:110, SharedUIPanelTemplates.xml:548) with the `metal_frame` chrome; the title is the NPC name (TUI.lua:69).
- [x] `TrainerTextures` (404984) background fitted to the ScrollBox −3,+4 / +3,−4 (TUI.lua:44-45); ScrollBox 302×330 at the Inset TOPRIGHT −5,+5 (TUI.xml:201-205), rows 298×47 with 1 px padding (TUI.lua:47), 7 rows (`CLASS_TRAINER_SKILLS_DISPLAYED`), mouse wheel scrolls one row.
- [x] Row (TUI.xml:28-105): row texture, icon 36×36 at LEFT 6 (desaturated and the 0.55 grey `disabledBG` when unavailable), name `GameFontNormal` at icon TOPRIGHT +6,−1, sub text 240×30 19 below it, `SmallMoneyFrame` at TOPRIGHT 5,−7 (red when unaffordable, TUI.lua:273-280), selected texture.
- [x] Sub text (TUI.lua:208-266): `REQUIRES_LABEL` "Requires:" and the level (`TRAINER_REQ_LEVEL` "Level %d"), skill (`TRAINER_REQ_SKILL_RANK` "%s (%d)") and abilities joined by ", "; "Already known" (`ITEM_SPELL_KNOWN`) without money for known services. Service names and icons come from the spell catalog; a missing spell shows "Unknown".
- [x] Rank bar `ClassTrainerStatusBar` 136×18 at 64,36 (TUI.xml:127-176) with `TRADESKILL_RANK` "%d/%d" for the tier line most services require, hidden until that line is learned.
- [x] Train (`MagicButtonTemplate` 80×22 at BOTTOMRIGHT) is enabled for an available, affordable service; a profession also needs a free primary slot (TUI.lua:273-307). Money frame over `UI-MoneyFrame-Border` (237619) at BOTTOMLEFT 5,−9.
- [x] Training a service that adds a primary profession (`TrainerService.profession`) first shows `CONFIRM_PROFESSION` (TUI.lua:11-33): "You may only know two professions at any one time.  Would you like to learn %s as your first/second one?" with Accept / Cancel; Accept sends `TrainerBuySpell`.
- [x] `TrainerBuyFailed { NotEnoughMoney }` shows "You don't have enough money." in `UIErrorsFrame`.
- [ ] Filter dropdown (available / unavailable / used), NPC portrait, `SkillStepButton`, service tooltips, hover highlight.

## ProfessionsBook (`ProfessionsBookFrame`, K)

- [x] K (`TOGGLEPROFESSIONBOOK`, Bindings_Standard.xml:1238; our `toggle_professions`) toggles it as a Panel. 550×525 `ButtonFrameTemplate` titled "Professions" (`TRADE_SKILLS`) with `Professions-Book-Left/-Right` (383588/383589) at 7,25.
- [x] PrimaryProfession1/2 (437×81) at 80,67 and 12 below; SecondaryProfession1-3 (437×46) 40 and then 30 apart (PB.xml:359-398).
- [x] Learned primary: icon in the 72×72 `ProfessionsBook` (383591) border at 7,7, name at 100,2, rank title = the tier line name (`skillLineName`, PB.lua:405-413), `ProfessionStatusBarTemplate` 95×16 with `Professions-Progress-Fill` (383590) and `TRADESKILL_RANK`, right cap at max rank (PB.lua:420-447).
- [x] Missing primary: "First Profession" / "Second Profession" (0.85,0.7,0.6) and `PROFESSIONS_MISSING_PROFESSION` (0.1,0.05,0.05).
- [x] The profession spell button (`SpellBookSpellID`, e.g. 3908 Tailoring) at TOPRIGHT −109,−3 opens the ProfessionsFrame on that profession.
- [ ] Unlearn button (unlearning is not supported), the second spell button, flyouts.

## ProfessionsFrame (Recipes page)

- [x] Wide window (942×658, `GetDesiredPageWidth`, PC.lua:344-351; PF.xml:7-8), title = profession name (`TRADE_SKILL_TITLE`), `metal_frame` chrome; tab 1 "Recipes" (`PROFESSIONS_RECIPES_TAB_NAME`) at the frame BOTTOMLEFT 22,2 (PF.xml:16-25).
- [x] The tier line shown is the learned child of the profession with the highest `ParentTierIndex`. Rank bar 453×18 at 280,40 (PC.xml:199-202): `Professions-skillbar-bg`/`-frame` (15729/15730), fill 441×18 at 5,3 from the first 856×34 frame of `Skillbar_Fill_Flipbook_<kit>` (Tailoring 4693230), `Skillbar_Fill_Flipbook_DefaultBlue` for a profession without kit art (PRB.lua:109-116), `TRADESKILL_NAME_RANK` "%s %d/%d" (`Number12FontOutline`).
- [x] RecipeList 274 wide at 5,72 (PC.xml:136-142) on `Professions-background-summarylist` (21219). SearchBox at 13,8 (RL.xml:40-45) with Common-Input-Border caps, the magnifying glass and "Search" while empty; clicking focuses it (keyboard goes to it), typing filters recipe names, Enter/Escape or clicking elsewhere ends focus.
- [x] ScrollBox at the SearchBox BOTTOMLEFT −5,−7 to BOTTOMRIGHT −20,5 (RL.xml:48-53); tree indent 10, 5 px padding, 1 px spacing (RL.lua:16-20); category rows 25, recipe rows 20 (RL.lua:96-109); mouse wheel scrolls one row.
- [x] Category row: `Professions-recipe-header-left/-middle/-right` (16623-16625), label `GameFontNormal_NoShadow` at LEFT 10,+2, collapse/expand icon (19542/19541) at RIGHT −10,+2 (RL.xml:94-146); a click collapses it. Recipes are grouped under their own `TradeSkillCategory`, categories by `OrderIndex`, recipes by name.
- [x] Recipe row (RL.xml:149-217, RL.lua:236-301): skill-up icon `Professions-Icon-Skill-High/-Medium/-Low` for orange/yellow/green (none when grey or at max rank), label in `PROFESSION_RECIPE_COLOR` (0xffe2dcd6), " [%d] " craftable count when above 0, `Professions_Recipe_Active` selected overlay. Difficulty uses shared `recipe_difficulty` (TrinityCore `SkillGainChance` thresholds).
- [x] SchematicForm at the RecipeList TOPRIGHT +2 (655×553, PC.lua:912-923) on `Professions-Recipe-Background-<profession>` (21205-21218, fallback 21206): output icon 47×47 at 28,33 with the item ring, `OutputText` at its RIGHT +14,+17 (SF.lua:443); "Reagents:" label and slots 180×50 (4 per column, 5 apart) below the icon +75,−65 (SF.xml:63-69, SchematicForm.lua:1286-1290).
- [x] Reagent slot (ReagentSlotBase.xml:6-27, ReagentSlot.lua:238-252): `Professions-Slot-bg` (15182) button with the item icon, name "have/need Name" (`TRADESKILL_REAGENT_COUNT` "%s/%d") in white, or `DISABLED_REAGENT_COLOR` (0xffa0a0a0) while short.
- [x] Create (80×22 at BOTTOMRIGHT −9,7), the `NumericInputSpinner` 30 left of it and Create All ("Create All [%d]", `PROFESSIONS_CREATE_ALL_FORMAT`) 30 left of that (PC.xml:205-224). Enabled while the bags hold the reagents for one craft. Create sends `CraftRecipe { casts: spinner }`, Create All `CraftRecipe { casts: craftable count }`; the spinner stays within 1..craftable.
- [x] The cast shows on the player cast bar (the server's replicated `CastState`); reagents, the created item and the skill-up arrive as `InventoryDelta` and `ProfessionSnapshot`.
- [ ] Filter dropdown, expansion dropdown on the rank bar, favourites, unlearned recipes, tooltips, recipe description, Specializations and Crafting Orders tabs, minimized mode, the cast bar moved onto the page (PC.lua:1282-1296), quality/concentration (Dragonflight systems are out of scope).
- [ ] Rank-bar `BarAnimation` flipbook playback and the `Skillbar_Flare_<kit>` fade (PRB.xml:94-99); the fill shows the first frame. The Tailoring kit fill is unit-tested only; the live proof showed DefaultBlue before this change.

## Tests asserting this spec

- `src/game/professions_data_tests.rs`: Bolt of Linen Cloth reagents/output/trivial ranks/category, Tailoring and its Classic tier from the pinned CSVs.
- `src/profession_tests.rs`: snapshot storage and the gained / skill-up / learned-recipe chat lines, IPC status and recipes text.
- `src/game/trainer_data.rs` tests: selection of the first available service, refresh keeps selection and scroll, scroll limits.
- `src/ui/screens/trainer_frame_component_tests.rs`: row texts, red cost, disabled background, row geometry, Train action, rank bar.
- `src/scenes/trainer_frame/tests.rs`: Georgio's rows and requirements, Train enabling with money and the primary limit, known services, the profession confirmation.
- `src/ui/screens/professions_book_component_tests.rs`: learned and missing entries, spell button action, entry layout.
- `src/ui/screens/professions_frame_component_tests.rs`: list rows, counts and geometry, schematic, reagent colours, Create buttons, search box.
- `src/scenes/professions_frame/view_tests.rs`: grouping, craftable counts, selection, difficulty, search, collapse, the book entries; `tests.rs`: bag counts.

## Verified scope

- [x] Live headless proof on an isolated server (:5059) at Georgio Bolero (verified: 2026-09-25): gossip "Train me." → trainer list with Tailoring selected → CONFIRM_PROFESSION → Tailoring learned for 10c (Classic Tailoring 1/300, 5 recipes, Retail chat lines) → K → book → ProfessionsFrame → Create ×2 and ×1 with the cast bar: 12 → 6 Linen Cloth, 3 Bolt of Linen Cloth in the backpack, Classic Tailoring 4/300, state kept across relogs. Evidence: game-engine `data/diagnostics/crafting-20260925/` (`proof.txt`).


## Native Godot coverage (2026-10-07)

Native profession recipe book and crafting window in `godot/ui-model` and `godot/rust`, consuming the existing owner `ProfessionSnapshot` and sending `CraftRecipe`. [Native UI host](../wiki/systems/godot-conversion.md) owns rendering architecture.

### What it must do

- [x] Consume the owner's profession lines and learned spells; list known recipes by DB2 category. Required rank remains crafting metadata, not a bracketed recipe-row suffix.
- [x] Filter recipe names with a case-insensitive search; retain a selectable schematic.
- [x] Show crafted item and reagents using DB2 names/icons and owned/needed counts summed across bags, not equipped items.
- [x] Create sends the selected recipe's spell ID and positive u16 cast quantity on `ProfessionChannel`; Create All sends the available reagent-limited quantity. Missing reagents disable Create.
- [x] Refresh bags and skill bar from authoritative inventory and profession updates; render Modern and Forever using native chrome and scroll lists. Real K and profession spellbook entry opening are live-proved.

### Native visual contract

- Recipe tree: 25-pixel collapsible category headers, 20-pixel plain recipe rows, 10-pixel indent, selection overlay and reagent-limited craftable count. No spell/category blue-swirl icons or red recipe buttons. Recipe names and craftable counts use neutral `PROFESSION_RECIPE_COLOR` (#e2dcd6), including selected rows. Difficulty comes from each recipe's existing trivial thresholds and learned line rank, conveyed only by high/medium/low skill-up icons; trivial or maximum-rank recipes have no icon.
- Schematic: output item icon/ring and item name; 180×50 reagent cells, four per column with 5-pixel spacing. Each cell contains an item slot, item-quality border, adjacent "owned/needed Name" text, with no count on the icon. Missing quantities dim the entire count/name text to `DISABLED_REAGENT_COLOR` (#a0a0a0); sufficient quantities use white, without changing crafting eligibility.
- Header/chrome: profession title, 453×18 textured rank bar at (280,40), rank/max text, 942×658 portrait metal frame, list at (5,72), schematic at (281,72). Search belongs inside the list. Create/Create All and quantity spinner belong at bottom right. Classic tier shows Recipes only; unsupported specialization/order systems remain absent.
- Forever uses its own atlas members/chrome where present, otherwise shared Retail profession art. Its c60 profession sheet `8164391` is referenced by the export but absent from local CASC (missing resolution/listfile entry, rendered preview RED on 2026-10-07). Those members use shared Retail art until `textures/8164391.blp` is supplied; available Forever chrome remains unchanged. No alternate crafting mechanics.
- Offline previews must use a concrete mixed-difficulty snapshot with several categories, a selected recipe and partly owned reagents, through the production screen in both skins. Preview APIs live in `*_preview.rs` secondary Godot API blocks.

Retail source root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- **(a)/(b)** `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeList.xml:94-217` (headers, skill-up indicator, plain row and selection); `.lua:232-242,267-301,346-376` (`GetLabelColor` returns neutral `PROFESSION_RECIPE_COLOR` for learned recipes, `DISABLED_FONT_COLOR` for unlearned; difficulty is carried by skill-up art). Selection changes overlays, not label color. Hover temporarily uses `HIGHLIGHT_FONT_COLOR`, restored on leave. Native lists currently contain learned recipes only; unlearned/hover presentation remains unsupported.
- **(c)** `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeReagentSlotBase.xml:6-27,41-62`, `Blizzard_ProfessionsTemplates.xml:43-85`, `Blizzard_ProfessionsRecipeReagentSlot.lua:99-129,238-252,331-333`, `Blizzard_ProfessionsRecipeSchematicForm.xml:31-38,63-69`, `.lua:1286-1290`; output ring: `Blizzard_ItemButton/Mainline/ItemButtonTemplate.xml:23-73` `CircularGiantItemButtonTemplate`. Retail formats `TRADESKILL_REAGENT_COUNT` ("%s/%d"), then `("%s %s"):format(quantityText, reagentName)` into the adjacent Name. `Update` colors the whole text via `GetNameColor`; required slots without an allocation use `DISABLED_REAGENT_COLOR`, allocated slots use `HIGHLIGHT_FONT_COLOR`. Native fixed-reagent crafting dims the text when the available quantity cannot meet the required allocation.
- **(d)** `Blizzard_Professions/Blizzard_ProfessionsRankBar.xml:5-61`, `.lua:103-136`, `Blizzard_ProfessionsCrafting.xml:199-202`.
- **(e)** `Blizzard_Professions/Blizzard_ProfessionsFrame.xml:7-25`, `.lua:269-292` (tab gates), `Blizzard_ProfessionsCrafting.xml:136-150,205-224`, `.lua:344-351,912-941`; search: `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeList.xml:40-53`.

### Original native visual proof (2026-10-07; superseded presentation)

The captures below preserve the original non-Retail difficulty-tinted labels and on-icon counts, not the corrected visual contract above.

Source `d1494131` on `professionsart`, after `a5b0c122`, `1d0d5de7` and `58b76b02`; no merge/push. Locked local helper `--test -p game-engine-godot -p game-engine-ui-model --no-fail-fast professions`: **12 passed** (eight existing UI-model cases, Account dispatch, two color/content cases and one native Taffy geometry case). Log `/tmp/claude/professionsart-final-tests.out`. Matching extension/CLI installed with the locked local helper `--cli`; `/tmp/claude/professionsart-final-build.out`. Both changed crates pass `cargo fmt --check`. No broad-suite/CI or separate `cargo check` claim; actual extension/CLI compilation and linking passed.

Inspected canonical `data/professions-art-20261007/modern.png` and `forever.png`: three categories, all four difficulty colors, Brown Linen Robe selected, Bolt of Linen Cloth 2/3 and Coarse Thread 4/1, Classic Tailoring 35/300, textured output/reagent slots and bottom-right controls. `godot/tests/capture_ui_screen.gd` passed native content, colors and geometry assertions in both skins; `modern-final.log` / `forever-final.log` retain results. Preview data lives in `godot/rust/src/ui/professions_preview.rs`, a secondary Godot API block; resolved geometry in `professions_art_tests.rs`, colors/content in `godot/ui-model/tests/professions_art.rs`.

Private Weston/Dozen rendering, no server. Local CASC supplied Tailoring portrait/fill; unavailable c60 sheet uses the explicitly allowed shared art. Wayland/Dozen capability warnings and RID/ObjectDB shutdown leak diagnostics remain in capture logs; not a leak-free teardown or pixel-identical Retail claim. Owned capture/compositor processes exited and `agents-professionsart.slice` is inactive. Original live craft ledger below remains historical behavior proof.

### How it works

- [Native profession data flow](../wiki/systems/professions-ui.md#native-godot-implementation).
- [Godot native UI host](../wiki/systems/godot-conversion.md).
- [Private live proof procedure](../headless-live-run.md).

### Implementation inventory

- `godot/ui-model/src/professions.rs` — recipe book state and crafting decisions.
- `godot/ui-model/src/professions_tests.rs` — concrete snapshot, filtering, bags and request behavior.
- `godot/ui-model/src/professions_catalog.rs` — pinned DB2 recipe metadata.
- `godot/ui-model/src/professions_frame.rs` — skin-aware native scroll list and schematic.
- `godot/network/src/lib.rs` — owner snapshot relay.
- `godot/network/src/professions_wire_tests.rs` — real UDP snapshot and craft request boundary.
- `godot/rust/src/professions.rs` — native catalog/window/input host; inventory and skill refresh.
- `godot/rust/src/account.rs`, `spells/casting.rs`, `spells/spellbook.rs`, `ui/mod.rs`, `window_stack.rs`, `lib.rs` — existing native host integration.

### Tests asserting native coverage

- `godot/ui-model/src/professions_tests.rs` — eight passing cases: requested five behaviors, authoritative refresh, DB2 joins (including signed sentinel count) and both-skin controls.
- `godot/network/src/professions_wire_tests.rs` — one passing real UDP snapshot/CraftRecipe request case.
- `godot/rust/src/professions_account_tests.rs` — one passing Loading snapshot/skill refresh dispatch case.
- Live scripts/captures: canonical `data/professions-live-20261007/`, excluded from Git.

### Native proof ledger

Runtime source `cc1c224f`; default debug extension and matching CLI installed through the locked local helper. UI-model eight tests pass on that revision; Account one and UDP one proofs remain valid because subsequent changes touch only recipe CSV parsing. Targeted logs: `/tmp/claude/professions-signed-green.out`, `professions-green-final.out` (Account pass; obsolete fixture failure retained), `professions-wire-green.out`; build: `professions-build-final.out`. Scoped Rust formatting passes; no full-suite or CI claim.

Private server `game-server.8b3819b`, SHA-256 `32ebbee39adf13fc08b566bdd1c72d59bbe2c64963710365accb07d2ed7ac8e7`, uses a copy of the approved offline redb and a read-only world.db backup. UDP 5314, Weston pf34, account `fb_professions`, own character ID 57. Admin learned Tailoring 3908, Classic Tailoring 264616 and Linen Bandage 3275; granted ten Linen Cloth. No server/protocol changes.

Four total client launches: first failed before login on wrong JS helper names; second exposed actual DB2 `SpellReagents` spell 44864 count -1. Reproduced RED fixture before fixing signed parsing to match the server. Third and fourth succeeded:

- Modern: K → recipe3275 → Create; Cloth10→9, Linen Bandage×1, Classic Tailoring1→2/300. Inspected `modern-before.webp` and `modern-after.webp`, with corresponding UI trees and inventory receipt.
- Forever: P → General → `SpellBookItem3908Button` → recipe3275 → Create; Cloth9→8, Linen Bandage×2, Classic Tailoring2→3/300. Inspected `forever-before.webp` and `forever-after.webp`, with corresponding UI trees and inventory receipt. Re-entry retained prior authoritative inventory/rank.

Artifact root: canonical `data/professions-live-20261007/`; `proof-ledger.txt`, `setup.log` and `cleanup.txt` retain exact inputs/results. Owned client/server/Weston PIDs and compositor children exited; `agents-professions.slice` inactive; UDP5314 free. No merge or push. Dozen/Wayland capability warnings and unrelated world spell-attachment errors remain; this is profession-window/crafting proof, not general renderer health or leak-free shutdown proof.

### Known gaps (current cycle)

- [ ] Native trainer and exact Retail book-entry/layout/art parity remain unimplemented; historical checkboxes above describe the retired client, not native proof.
- [ ] Existing CastFailed UI error path is wired into the schematic status; Escape/X closure is implemented. Profession-specific refusal/closure runtime matrices have not been exercised.

### Out of scope

- Server or shared protocol changes; crafting orders and Retail systems without existing server messages.

### Retail references

Local cache root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_FrameXML/Bindings_Standard.xml:1238-1240`: `TOGGLEPROFESSIONBOOK` invokes `ToggleProfessionsBook()`.
- `Blizzard_Professions/Blizzard_ProfessionsCrafting.lua:39,52-55,967-984,1011-1021`: search, Create All and selected quantity passed to crafting transaction.
- `Blizzard_Professions/Blizzard_ProfessionsRankBar.lua:103,136`: profession rank text and rank/max ratio.
