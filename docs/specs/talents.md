# Read-only Talents

The PlayerSpellsFrame Talents page displays the local Retail class/spec talent tree in Modern and Forever skins. Core loading lives in `godot/core/src/talent_data.rs`; [data/layout provenance](../wiki/systems/talents.md) describes the projection.

## What it must do

- [x] Map character class to its category-7 SkillLine, then SkillLineXTraitTree (variant 0), not to a hardcoded tree ID.
- [x] Filter nodes/entries by TraitCond Visible conditions and SpecSetMember. Preserve sufficient (OR) hero-spec conditions.
- [ ] Display class nodes left, specialization nodes right, eligible hero subtrees in the middle; retain ordered choice entries and DB2 edges.
- [ ] Translate PosX/PosY using Blizzard's /10 scale and pan offsets. Hero nodes use normalized top/center positions and 0.85 scale.
- [ ] Show TraitDefinition spell icons, or OverrideIcon; missing art stays missing, never a substitute icon.
- [x] Only matching Granted conditions show learned ranks. Zero purchase cost alone is not a grant; no starter-loadout allocations are applied.
- [ ] Hover uses shared spell GameTooltip name/description; choice entries each expose their spell tooltip. No node click purchases, casts or mutates a character.
- [x] Mage Arcane (62) selects tree 658, class 43/spec 38 nodes; three cited nodes and choice 62087 match CSV. Eligible Sunfury/Spellslinger have 14 nodes each.
- [ ] Native cage captures show both skins at actual 1920×1080.

## How it works

- [Local DB2 mapping, Blizzard layout and witnesses](../wiki/systems/talents.md).
- [Shared PlayerSpellsFrame geometry](spellbook-action-bar.md).

## Implementation inventory

- `godot/core/src/talent_data.rs`: local DB2 graph projection, no engine/network dependency.
- `godot/core/tests/talent_data.rs`: concrete Mage Arcane CSV contract.
- `godot/ui-model/src/spellbook_preview.rs`: offline local Mage snapshot.
- `godot/ui-model/src/ui/screens/spellbook_frame_component.rs`: shared PlayerSpellsFrame state/page dispatch.

## Tests asserting this spec

- `godot/core/tests/talent_data.rs`: mapping/counts, three nodes/edges, choice entries, grants, hero gating.

## Known gaps (current cycle)

- [ ] UI implementation and targeted projection GREEN pending. Core five-test GREEN at `cc9531ed8` (also present in `ce90b9610`), log `talenttree-green-core.log`.
- [ ] Cage capture and hover proof pending.
- [ ] Wowdev retrieval returns HTTP 403; local schemas and Blizzard enums are directly inspected instead. URLs are recorded as requested references, not as successfully read sources.

## Out of scope — not yet: learning, loadouts, server

- Learning/spending, refunds, rank editing, activation of hero choices: later slice.
- Loadout application/import/export, saved allocations: later slice. TraitTreeLoadout rows are provenance only, never learned-state input.
- Server persistence, validation and protocol changes: later slice; this page is explicitly read-only.
