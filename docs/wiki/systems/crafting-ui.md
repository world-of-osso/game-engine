# Crafting UI

Verified: 2026-10-11 (bounded development proof, not final/live acceptance).

[Contract](../../specs/crafting-ui.md) owns requirements/gaps. [Existing professions](native-professions-crafting.md) owns fixed-output recipe/trainer behavior. Retail Mainline owns both skins' layout; Forever only selects art.

## Current projection

`ProfessionView` selects Recipes, Specializations, Orders or the Recipes-page recraft schematic. Customer orders render a separate825×568 PortraitFrame; crafter orders use the942×658 ProfessionsFrame. Specializations use the1144px page and authored profession background kit. Shared top tabs use Mainline TabSystem rotation/active art, not an additional skin-specific layout.

Specialization state keeps server snapshots separate from staged absolute purchased ranks. Currency costs use TraitCurrencyIDs; snapshot currency records map wallet-backed CurrencyTypesIDs and derived unlock tokens. Apply submits selected-tree purchases once, Undo discards local purchases, root unlock requires confirmation. Native tooltip ownership resolves hovered node frames and presents source description/ranks/lock reasons. Authored edges come from the server. Retail Specializations.lua432-578 supplies radial placement for zero-coordinate path nodes. Entries remain in authored order; one node button selects the first unfinished entry, with drafts keyed by tree/node/entry. Displayed knowledge ranks exclude the unlock tier. No invented tree topology or level-based knowledge grants.

Selected-path dial perks use the source `path_node_id`, requirement and independent `is_major_bonus` bit. Mainline Templates.lua23-26 supplies the1.4-radian dial gap; Specializations.lua724-749 places minor/major milestones at177/189px. Perk flipbooks show their first90px cell for unearned/pending and last for committed; only pending milestones show the authored border glow. This addition awaits the announced protocol major-bit extension before compilation/live verification.

Orders use the server's source-scoped fixed-output recipe list, not the customer's profession knowledge or all local create-item spells. The customer form preserves exact requirement quantities separately from contribution counts; money boxes use copper conversion, recipient/duration dropdowns expose explicit choices. Claim/fulfill/release/cancel consume server eligibility flags and never optimistically change orders/inventory. Server completion/mail is the authority.

Recraft's owned-item input slot and disabled submission are visible. The server contract has no supported operation evaluator/owned-item lineage, so there is no recraft submission wire message and no CREATE_ITEM24 substitution. This remains a goal gap, not completed recrafting.

## Development proof ledger

Canonical artifacts: `data/diagnostics/crafting-ui-2026-10-11/`. No final native/private-server screenshots yet.

- Tabs: RED absent Specializations text; subsequent authored atlas-name correction; both-skin tab/art tests GREEN.
- Staged ranks: RED no staging; currency/edge protocol extension adopted; three model tests GREEN as part of15 scoped model tests atd98bbcc4d.
- Orders: RED no place/claim request;15 scoped model tests GREEN atd98bbcc4d. Source-catalog/can_release extension later changes that proof scope; rerun required.
- Screens: RED absent customer/crafter/recraft frames;10 scoped integration tests GREEN at2e32e9463.
- Menus: RED cyclic recipient control; four scoped order screen tests GREEN atbdb429cd2.
- Takeover at2824296b4/protocol5b3cdcc:20 model+14 screen tests,5 native dispatch/geometry/mask tests and native extension/CLI build GREEN. New tier metadata exposed an incomplete catalog fixture;38e711d16 corrects its source columns/table. Logs:takeover-model-full.log (RED),takeover-model-green.log,takeover-native-tests.log,takeover-native-build.log. Native live fixture uses normal character selection and physical frame/NPC/mailbox input; no result injection.

## Remaining integration

Customer NPC lifecycle and ordered unlock/knowledge entries are wired. Selected-path perk ownership is supplied; major-perk art flags and source-resolved trait descriptions remain pending in the shared contract. Joint private UDP5871 fb_* proof is pending. Final origin/master integration, targeted tests/check/fmt, independent verification, push and inspected both-skin PNG publication remain open. Never use UDP5000 or canonical world.db writes for this work.

## Sources

- [Crafting UI contract](../../specs/crafting-ui.md) — source grounding and acceptance.
- `godot/ui-model/src/professions_specializations.rs`, `professions_orders.rs`, `professions_frame*.rs` — state and render decisions.
- `godot/rust/src/professions.rs`, `professions_orders.rs`, `account.rs`, `tooltip_sources.rs` — native input/transport/tooltip wiring.
- Canonical diagnostic logs listed above — scoped command evidence.

## See Also

- [[native-professions-crafting]] — established Recipes/trainer pipeline.
