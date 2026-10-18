# Modern crafting source gaps

Verified: 2026-10-16. Allowed inputs: local Retail 12.1.5.70391 CSV/CASC, read-only world.db/TDB1210, pinned TrinityCore a352b1fa, pinned WoWDBDefs d6f50d4f351193e84226aa62a36faa0f58528afa, installed Mainline UI. Receipt JSON files record raw URLs and SHA-256.

## CRAFTING_OPERATION_COMPOSITION

TrinityCore `src/server/game/Spells/SpellEffects.cpp` lines380/389 maps `SPELL_EFFECT_CRAFT_ITEM`288 and `SPELL_EFFECT_RECRAFT_ITEM`297 to `Spell::EffectNULL`. SHA-256: `fb804a9dfa59a1bb3ccaea7aeaec5850f2eeb6f5f177681dbe5da9377aa05b74`. The pinned raw URL is recorded in `trinitycore.json`.

Installed `Blizzard_ProfessionsTemplates/Blizzard_ProfessionsRecipeSchematicForm.lua` lines282–286 calls `C_TradeSkillUI.GetCraftingOperationInfoForOrder` or `C_TradeSkillUI.GetCraftingOperationInfo`; it does not implement their calculations. Lines290–299 consume their result. CraftingData/quality/difficulty/reagent/effect/curve tables identify authored inputs, but do not define operation composition. No default quality, skill contribution, random roll, concentration cost or recraft lineage rule can be inferred from those identities alone.

Lead clarification explicitly confirms no permitted operation oracle. Authoritative outcome evaluation must return this gap; crafted quality remains unassigned. Reagent selection validation is not craft success and must never consume items or advance skill while this gap remains.

## Existing sourced behavior

Specializations already reuse the server trait engine and authoritative currency spending. Public/guild/personal fixed-output orders already have server escrow, visibility/eligibility and mail delivery. Recraft UI already exposes an owned-item element with disabled submission. Existing proof is catalogued in `docs/wiki/systems/crafting-ui.md`; it is not evidence for quality/concentration/recraft outcomes or knowledge acquisition.

## CRAFTING_CONCENTRATION_ORIGIN

`ProfessionExpansion` maps Dragon Isles Leatherworking SkillLine2830 to CurrencyTypes3049. Its authored fields are MaxQty1000, RechargingAmountPerCycle1 and RechargingCycleDurationMS345600; these describe capacity and cycle units, not an initial wallet grant or a recharge epoch. No permitted source establishes when a character first receives this currency or its recharge origin. Missing wallet entries must remain unknown, not zero/full; no epoch, login-time origin, automatic grant or offline refill is assigned.

Installed `ProfessionsCurrencyTemplate` is110×32, icon24×24 LEFT, label LEFT of icon RIGHT+6,y+2; CraftingPage anchors it TOPLEFT120,-35. Lua728–824 reads native currency info, refreshes per cycle duration/1000 and displays quantity/max. The native API does not expose its implementation here. Metadata/display support is not evidence for a live recharge authority, concentration cost, or crafting outcome. Source hashes are in local-db2.json and local-ui.json.
