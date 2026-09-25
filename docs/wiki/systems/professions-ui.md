# Professions UI

Retail profession trainer, ProfessionsBook and ProfessionsFrame on the server profession protocol. The spec is [professions-frame](../../specs/professions-frame.md); server rules are game-server `docs/wiki/systems/professions.md`.

## Data flow

- **Recipes** are client data, as in Retail: `game/professions_data.rs` builds a `ProfessionCatalog` from the pinned DB2 CSVs (`SkillLine`, `SkillLineAbility` rows with a skill-up line, `SpellReagents`, `SpellEffect` CREATE_ITEM, `TradeSkillCategory`, `SpellName`/`SpellMisc`, `ItemSparse`) on first use and caches it under `data/cache/profession_catalog-<build>.bin`.
- **Learned state**: the server sends `ProfessionSnapshot { lines, spells }` on world entry and after every change. `networking::messages::receive_profession_snapshot` is the only reader: it stores `ProfessionStatusSnapshot` and posts the gained-skill / skill-up / learned-recipe chat lines (`profession::apply_snapshot`, silent for the first snapshot).
- **Bags**: craftable counts and reagent `have` counts sum `InventoryState` stacks.
- **Crafting**: Create / Create All write `CraftRequest`; `profession.rs` sends `CraftRecipe { spell_id, casts }`. The cast bar, reagent removal, the new item and the skill-up come back through the normal `CastState`, `InventoryDelta` and `ProfessionSnapshot` paths.

## Trainer

`game/networking/trainer.rs` fills `TrainerState` from `TrainerList` (opens / refreshes the frame), shows `TrainerBuyFailed` text, closes on `NpcFrameEvent::Closed` and sends `TrainerBuySpell` for `TrainerRequest`s. `scenes/trainer_frame` builds the `ClassTrainerFrame` view (names and icons from the spell catalog, requirement text from the catalog line names), syncs the `Trainer` window with the session (like the merchant), and asks `CONFIRM_PROFESSION` through `PopupStack` before a service with `profession = true`.

## Book and frame

`scenes/professions_frame` owns both screens. K toggles `WindowId::ProfessionsBook`; a book entry's spell button sets `ProfessionsFrameSelection.profession` and opens `WindowId::Professions` (Wide). `view.rs` holds the pure view models: the tier line is the learned child with the highest `ParentTierIndex`; recipes of that line are grouped by their `TradeSkillCategory`; difficulty uses shared `recipe_difficulty`. The search box is an edit box that takes keyboard focus on click (`UiInputMode::Text`), so K and other bindings do not fire while typing.

## Gaps

Filter dropdowns, expansion switcher, tooltips, unlearn, Specializations / Crafting Orders tabs, cast bar docked on the page, quality and concentration. Gathering (herb/ore nodes) waits for server game objects; doodad mining nodes are not interactable.
