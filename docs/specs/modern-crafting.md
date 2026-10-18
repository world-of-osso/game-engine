# Modern crafting

Extend the existing [ProfessionsFrame](professions-frame.md), trainer and server craft pipeline. Retail Mainline defines behavior; Forever changes art only.

## Acceptance

- [x] One locally best-covered crafting profession is playable from trainer learning through recipe search/category selection, authored reagent requirements, cast, result item and authored skill-up in both skins.
- [x] A trainer-learned recipe is not blocked by the DB2 automatic-learning threshold. Authored trainer requirements and authoritative recipe knowledge own acquisition; bag requirements own craft count.
- [x] Recipe categories follow `TradeSkillCategory.OrderIndex`, not alphabetical category names; recipes inside each category remain name-sorted.
- [x] Reagent crafting tiers are shown from `Item.CraftingQualityID` joined to `CraftingQuality` and its authored inventory overlay atlas, separately from item rarity.
- [x] Behavioral tests cover trainer purchase, reagent consumption, cast completion, output and skill progression; private UDP5770 native proof covers both skins for Classic Leatherworking.

## Data choice

Pinned Retail 12.1.0.69933 `SkillLineAbility`/`SpellEffect`/`SpellReagents` census: Leatherworking has 1,286 effect24 create-item rows with authored reagent rows, ahead of Blacksmithing1,091, Jewelcrafting967 and Tailoring904. Of those Leatherworking rows,1,229 have a nonzero skill-up line and trivial-high threshold. Leatherworking therefore provides the largest directly authored fixed-output corpus. Counts are rows, not unique recipes or proof that all outputs have server templates.

Local world.db has only627 of1,286 distinct Leatherworking effect24 outputs joined to `content_item`; pinned CSV presence does not imply server runtime readiness. Existing Classic Leatherworking is the first end-to-end acceptance slice, not a claim of complete Midnight support.

## Learned-recipe boundary

Private Modern proof learned Handstitched Leather Pants2153 from Simon Tanner at authored trainer rank15/cost50. Bags held14 Light Leather and1 Coarse Thread. Client incorrectly showed Create All[0]: pinned `SkillLineAbility.MinSkillLineRank` is30. TrinityCore a352b1fa `Player.cpp:25450` uses that field to remove **AutomaticSkillRank** abilities below their threshold; `Spell.cpp:CheckItems` does not impose it on manually learned spells. Retail `ProfessionsCrafting.lua:631-637` requires a learned recipe, and `ValidateCraftRequirements:569-623` delegates actual requirements/reagents. The client must not invent an additional automatic-learning gate for a recipe the authoritative trainer taught.

## Operation authority (2026-10-16)

- Quality/concentration/recraft outcome evaluation returns the named `CRAFTING_OPERATION_COMPOSITION` gap. Crafted quality stays unassigned; no default tier, reagent consumption or skill gain is permitted on refusal.
- Authored reagent slots, eligible item tiers and owned allocations can be selected and validated independently; validation is not successful crafting.
- [Pinned source receipts and gap ledger](../sources/modern-crafting/gaps.md) distinguish static identities from missing operation composition.

## Authored quality-reagent selection

- Mainline quality recipes remain selectable even when no output can be assigned. Only fixed CREATE_ITEM24 outputs use a concrete output item; CRAFT_ITEM288 does not choose the lowest tier.
- Required basic slots use ModifiedCraftingSpellSlot → MCRSlotXMCRCategory → CraftingReagentQuality. Item.CraftingQualityID joins the authored tier independently of rarity; mixed allocations preserve per-item quantities. Mainline ReagentSlot.lua267–306 displays the selected inventory-tier glyph or IconMixedInv for multiple tiers; before quality allocations exist, actual owned stock determines that glyph. No stock means no glyph, never a default tier.
- The shared 329×207 Mainline quality dialog has 100×75 tier containers, source item buttons and NumericInputSpinner dimensions, Accept/Cancel and a non-consuming server validation response. Values clamp to owned/required; increasing one tier removes overflow from other tiers in source order. Cancel discards only dialog edits.
- Server validates recipe knowledge, membership, exact slot totals and aggregate bag ownership. Both skins share this behavior. Modifying/finishing eligibility, currency-slot allocation and unevaluated slot conditions remain explicit gaps, not substitute basic slots.

Native acceptance verified2026-10-16: `godot/tests/modern_crafting_quality_live.gd` passes both explicit skins using owned accounts on UDP5951, a rendered1920×1080 viewport, physical quality-dialog clicks, typed ownership clamps, Cancel, authoritative incomplete-slot refusal, accepted mixed allocations, reagent item tooltip and disabled crafting with quality unassigned. [Native proof](../wiki/systems/native-professions-crafting.md#authored-quality-selection--verified2026-10-16) owns artifact paths and root-cause findings. Private test grants are not trainer/knowledge acquisition evidence.

## Knowledge/specialization proof

Reuse the existing authoritative trait engine and its authored currency mapping; do not create a second progression system. Native fixture `godot/tests/modern_crafting_knowledge_live.gd` exercises tree unlock, node/perk tooltips, staged spending, Undo and Apply on private UDP5951 at 1920×1080 in both skins. It uses recipe375188, tree350, path28546 and perk28534 from the existing authored catalog. Its explicitly seeded knowledge is not evidence of acquisition; quest-reward acquisition is independently covered by the existing server specialization tests. Both native runs passed2026-10-16: rank0→1/30, knowledge25→24, Undo before Apply, committed rank retained after Apply. Fresh server/client relogs and an offline redb reopen both retain tree350 unlocked/rank1/knowledge24 without advancing profession skill. [Native proof](../wiki/systems/native-professions-crafting.md#knowledge-reuse--verified2026-10-16) owns evidence and limits.

## Explicit gaps

- Effect288 quality crafting and effect157 loot-derived crafts require authored output selection and operation rules; fixed-output effect24 support is not evidence for either.
- Concentration/inspiration/resourcefulness/multicraft and quality-dependent skill contributions must not use invented formulas. Static difficulty/bonus percentages alone do not derive the server operation.
- Missing quality-reagent alternatives/slot bindings remain gaps; an item quality ID is not permission to substitute another item or aggregate tier inventories.
- Gathering branch `gathering-nodes` is independent and not integrated here. No canonical world.db writes.

## Quality source receipt

Local CASC extraction from Retail build key `dcfc90fffd79ba00406ae46f5f657592`: `CraftingQuality`4373538 (11 rows, layout5E7AC242), `CraftingQualityAtlasSet`7012110 (7 rows, layout4FC8E439), `CraftingDataItemQuality`4545612 (1,687 rows, layoutDE11A051), no encrypted rows dropped. DB2 headers preserve shared static68914 payloads selected by the69933 build root; headers are not rewritten. Layout field order checked against wowdev/WoWDBDefs. Runtime reads the first two exports, `Item` and `UiTextureAtlasElement`; output-variant export records available identities but is not an outcome formula. QualityID14 means tier2/atlas-set7, not tier14 or item rarity2. Its inventory glyph is element32404, authored33×28 on sheet7487371 (locally extracted and included in fixture assets); preserve `UseAtlasSize` rather than squashing it to20×20.

## Native acceptance — verified2026-10-11

Private UDP5770, owned `fb_moderncraft_m`/`fb_moderncraft_f`, Classic Leatherworking2532. Both skins: normal charselect→EnterWorld, Simon Tanner trainer56, primary profession264578, recipe categories/search,14 Light Leather2881 casts consume42 Ruined Leather Scraps and raise rank1→15; trainer teaches2153, single pants craft consumes4 Leather+1 Coarse Thread, grants item2303 and raises rank16. Native InventoryList observes10 Leather, zero Thread and1 Handstitched Leather Pants. Modern result also verifies persisted trainer/recipe/rank state after private server restart. Canonical diagnostics: `data/diagnostics/modern-crafting-2026-10-11/`; [native system](../wiki/systems/native-professions-crafting.md) owns proof/artifact details. Static reagent-quality glyphs have both-skin model/geometry proof; this acceptance does **not** certify variable-quality output math.

## Sources

Existing profession implementation cites TrinityCore a352b1fa `Trainer::TeachSpell`, `Spell::CheckItems`/`TakeReagents`, `Player::UpdateCraftSkill` and `SkillGainChance`. Retail local UI: `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeList.lua`, `Blizzard_ProfessionsRecipeReagentSlot.lua` (quality overlay), `Blizzard_Professions.lua:GetReagentQualityInfo`; both skins reuse these elements.

## Concentration currency display

- Read-only owner queries require a learned skill line and return the authored ProfessionExpansion→CurrencyTypes ID, cap, icon, recharge amount and millisecond cycle. No crafting outcome is inferred.
- Both skins use Mainline ProfessionsCurrencyTemplate:110×32 at CraftingPage TOPLEFT120,-35,24×24 icon and source label anchor. Known quantities use the source quantity/max format; unknown quantities use an em dash rather than an invented initial zero/full wallet.
- The currency tooltip preserves the Retail description/full-cap warning where applicable and discloses `CRAFTING_CONCENTRATION_ORIGIN`. No initial grant, recharge epoch, login origin or live recharge is implemented while this gap remains unsourced. Static recharge fields are not live-regeneration proof.
- Concentration metadata never enables Create or bypasses `CRAFTING_OPERATION_COMPOSITION`.

Both rendered1920×1080 skins passed `modern_crafting_concentration_live.gd` on owned private UDP5951: unknown quantity, authored tooltip with named gap and disabled Create. [Native proof](../wiki/systems/native-professions-crafting.md#concentration-metadata-boundary) owns artifacts and exact gate scope; this is display/metadata proof, not live-regeneration proof.
