# Transmog application

Native Retail Mainline Transmogrify service for collected appearances. [Authoritative server contract](../../../game-server/docs/specs/transmog-apply.md). Forever is an art skin only; both skins use the same window, choices and application semantics.

## What it must do

- [x] Transmogrifier NPC/gossip opens the Transmogrify frame with collected, locally supported eligible per-slot appearance choices.
- [x] Stage pending changes without modifying replicated equipment; preview selected appearance and show the exact authoritative gold quote.
- [x] Apply only affordable locally defined quotes; disable duplicate application while awaiting server acceptance and show explicit refusal errors.
- [x] Show original appearance restoration and hide choices only for server-authorized slots.
- [x] Preserve pending-state safety across item replacement, interaction closure and character/session changes.
- [x] Native self and observer displays update after successful application and remain after relog, in both skins.
- [ ] Collection uses paged 3D appearance models (3 rows × 6 columns), collected/applied/pending art, native name search, and icon slot buttons in two fixed columns around the character preview. Filtering must preserve pending choices and exact quotes; clearing it restores model cells.
- [ ] Outfits dropdown/Save belong above the preview; authoritative cost/Apply belong below it. Outfits remain disabled with an explicit gap when not server-backed.
- [ ] Preview shows the equipped and pending main-hand weapon, independently of world equipment, with rotation and Sheathe/Draw controls. Weapon-cell camera fitting must show the blade profile rather than its end.
- [ ] Saved outfits follow the working core apply path; illusion UI requires authentic local illusion data.

## How it works

- [Transmog system](../wiki/systems/transmog-apply.md)
- User-requested Wardrobe service arrangement: two slot columns around the model, top outfits, bottom cost/Apply and right-hand paged collection. Current cached Retail no longer contains `WardrobeFrame`/`WardrobeTransmogFrame`; `Blizzard_Wardrobe.xml:5–98` and `.lua:566–568` still define the authentic 78×104 DressUpModel collection cells, border states and 18-cell pages. Exact historical Wardrobe frame anchors are not present in that cache. `Blizzard_Transmog.lua` owns pending-cost/apply semantics.

## Implementation inventory

- `godot/ui-model/src/transmog.rs` — pending choices, quotes and application state.
- `godot/ui-model/src/transmog_frame.rs` — shared-skin native frame and slot/source picker.
- Sibling `shared-protocol/src/protocol/transmog_messages.rs` — authoritative quote/application wire contract.

## Tests asserting this spec

- `godot/ui-model/src/transmog_tests.rs` — pending versus applied state, exact prices, refusals, stale GUIDs,18-cell paging and independent source preview equipment.
- `godot/ui-model/tests/transmog_wardrobe.rs` — both-skin slot/model controls and collected/applied/pending labels.
- `godot/tests/transmog_wardrobe_live.gd` — native main-hand pixel contribution, loaded collection meshes, staging, paid Apply and Escape teardown on private55087.

## Known gaps (current cycle)

- [x] Native host/network integration: private55087, Hashom gossip, target2850 GUID123, sources821/1181, exact1500copper applications, self/observer rebuilds and relog in both configured art skins. Private-server restart preserved the same redb modifiers. [Bounded proof and evidence](../wiki/systems/transmog-apply.md#native-proof--verified-2026-10-11); this is not every-slot/product certification.
- [ ] Native source cells own 3D model scenes; weapon cells frame the weapon alone. Both-skin native acceptance remains pending.
- [ ] Saved outfits remain unimplemented; core live application proof is available.
- [ ] Dynamic pricing, conditional source policies and illusion availability remain the linked server contract's explicit local-data gaps.

## Out of scope

- Fabricated appearance data/prices, Forever-only behavior/layout, world.db writes, master merges and port5000 mutations are prohibited for this task.
