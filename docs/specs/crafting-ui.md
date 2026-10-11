# Crafting UI

Rust ui-model screens and the native Godot extension extend [ProfessionsFrame](professions-frame.md). Retail Mainline `Blizzard_Professions*` defines layout/behavior; Forever changes art only. [System notes](../wiki/systems/crafting-ui.md) own implementation/proof.

## What it must do

### Specializations
- [ ] Recipes, Specializations and Crafting Orders tabs share the Retail profession frame in both skins.
- [ ] Authored trees, path positions, edges, names, descriptions, ranks and lock reasons come from authoritative snapshots; knowledge currency costs match TraitCurrencyIDs through the server currency mapping.
- [ ] Spending stages absolute purchased ranks, updates pending knowledge, never mutates committed ranks, and cannot spend an unavailable/maxed path or an insufficient currency.
- [ ] Apply submits selected-tree purchases once; Undo discards pending purchases without refunding committed knowledge. Tab unlock requires confirmation and server authority.
- [ ] Hover path tooltips show source description, purchased plus granted rank, perk requirements and lock reason.

### Fixed-output orders
- [ ] Customer NPC lifecycle opens a separate Retail customer view; crafter orders open from the profession tab.
- [ ] Public/guild/personal filters and customer/crafter views preserve server-scoped visibility and eligibility.
- [ ] Customer form sends exact authored reagent quantities/customer contributions, recipient, commission, duration and note. Server owns escrow and validation.
- [ ] Claim/release/fulfill/cancel operate on selected authoritative order IDs; completion is observed through server order/mail/inventory state, never fabricated locally.

### Recraft
- [ ] Expose the Retail owned-item input/output element; submission stays disabled while source operation/lineage is unavailable. Never substitute CREATE_ITEM24 for recrafting.

### Integration
- [ ] Behavioral tests and private UDP5872 fb_* native proof cover both skins, followed by origin/master integration, targeted tests, native build and fmt.

## How it works
- [Crafting UI system](../wiki/systems/crafting-ui.md)
- [Existing fixed-output crafting](../wiki/systems/native-professions-crafting.md)

## Implementation inventory
- `godot/ui-model/src/professions_frame.rs` — shared frame/page selection.
- `godot/ui-model/src/professions_specializations.rs` — staged knowledge decisions and tooltip content.
- `godot/ui-model/src/professions_frame_specializations.rs` — Retail tree/path/dial controls.
- `godot/rust/src/professions.rs` — native input, server requests and view projection.
- `godot/rust/src/account.rs`, `godot/network/src/lib.rs` — owner message transport/dispatch.
- `godot/rust/src/tooltip_sources.rs` — native hovered path tooltip ownership.

## Tests asserting this spec
- `godot/ui-model/src/professions_specializations_tests.rs`
- `godot/ui-model/tests/professions_art.rs`
- `godot/ui-model/tests/professions_specializations.rs`

## Known gaps (current cycle)
- [ ] Customer/crafter orders screens, NPC service integration and joint live proof pending.
- [ ] Recraft evaluator/owned-item lineage unsupported by current server contract; no submission message exists.

## Out of scope
- Variable quality, concentration and invented knowledge acquisition/formulas. [Modern crafting](modern-crafting.md) owns those explicit gaps.

## Source grounding
- `Blizzard_ProfessionsFrame.lua:40-42`: Retail page tabs.
- `Blizzard_ProfessionsSpecializations.lua:39-99,147-167`: staged rank, Apply/Undo and confirmed tab purchase.
- `Blizzard_ProfessionsSpecializations.xml`: 1144px specialization page, 706px tree and selected-path dial/footer.
- `Blizzard_ProfessionsSpecializationsTemplates.xml/.lua`: source node/edge art and tooltips.
- `Blizzard_ProfessionsTemplates/Blizzard_Professions.lua:1408-1410`: specialization background kit naming.
- `Blizzard_ProfessionsCustomerOrders*`, `Blizzard_ProfessionsCrafterOrder*`: role-specific forms/lists.
- `Blizzard_ProfessionsRecipeRecraftSlot.lua`: owned item GUID allocation; cannot be replaced with template selection.
