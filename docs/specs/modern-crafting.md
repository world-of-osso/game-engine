# Modern crafting

Extend the existing [ProfessionsFrame](professions-frame.md), trainer and server craft pipeline. Retail Mainline defines behavior; Forever changes art only.

## Acceptance

- [ ] One locally best-covered crafting profession is playable from trainer learning through recipe search/category selection, authored reagent requirements, cast, result item and authored skill-up in both skins.
- [x] Recipe categories follow `TradeSkillCategory.OrderIndex`, not alphabetical category names; recipes inside each category remain name-sorted.
- [ ] Reagent crafting tiers are shown from `Item.CraftingQualityID` joined to `CraftingQuality` and its authored inventory overlay atlas, separately from item rarity.
- [ ] Behavioral tests cover trainer purchase, reagent consumption, cast completion, output and skill progression; private UDP5770 native proof covers both skins.

## Data choice

Pinned Retail 12.1.0.69933 `SkillLineAbility`/`SpellEffect`/`SpellReagents` census: Leatherworking has 1,286 effect24 create-item rows with authored reagent rows, ahead of Blacksmithing1,091, Jewelcrafting967 and Tailoring904. Of those Leatherworking rows,1,229 have a nonzero skill-up line and trivial-high threshold. Leatherworking therefore provides the largest directly authored fixed-output corpus. Counts are rows, not unique recipes or proof that all outputs have server templates.

Local world.db has only627 of1,286 distinct Leatherworking effect24 outputs joined to `content_item`; pinned CSV presence does not imply server runtime readiness. Existing Classic Leatherworking is the first end-to-end acceptance slice, not a claim of complete Midnight support.

## Explicit gaps

- Effect288 quality crafting and effect157 loot-derived crafts require authored output selection and operation rules; fixed-output effect24 support is not evidence for either.
- Concentration/inspiration/resourcefulness/multicraft and quality-dependent skill contributions must not use invented formulas. Static difficulty/bonus percentages alone do not derive the server operation.
- Missing quality-reagent alternatives/slot bindings remain gaps; an item quality ID is not permission to substitute another item or aggregate tier inventories.
- Gathering branch `gathering-nodes` is independent and not integrated here. No canonical world.db writes.

## Sources

Existing profession implementation cites TrinityCore a352b1fa `Trainer::TeachSpell`, `Spell::CheckItems`/`TakeReagents`, `Player::UpdateCraftSkill` and `SkillGainChance`. Retail local UI: `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeList.lua`, `Blizzard_ProfessionsRecipeReagentSlot.lua` (quality overlay), `Blizzard_Professions.lua:GetReagentQualityInfo`; both skins reuse these elements.
