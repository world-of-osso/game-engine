# Transmog application

Native Retail Mainline Transmogrify service for collected appearances. [Authoritative server contract](../../../game-server/docs/specs/transmog-apply.md). Forever is an art skin only; both skins use the same window, choices and application semantics.

## What it must do

- [x] Transmogrifier NPC/gossip opens the Transmogrify frame with collected, locally supported eligible per-slot appearance choices.
- [x] Stage pending changes without modifying replicated equipment; preview selected appearance and show the exact authoritative gold quote.
- [x] Apply only affordable locally defined quotes; disable duplicate application while awaiting server acceptance and show explicit refusal errors.
- [x] Show original appearance restoration and hide choices only for server-authorized slots.
- [x] Preserve pending-state safety across item replacement, interaction closure and character/session changes.
- [x] Native self and observer displays update after successful application and remain after relog, in both skins.
- [x] Collection uses paged native3D appearance models. Retail12.1 `TransmogItemModelTemplate` is100×132; its right-hand collection has five columns and four rows. Filtering must preserve pending choices and exact quotes; clearing it restores model cells.
- [x] Retail12.1 item state art is `transmog-itemcard-transmogrified` (pending suffix for staged changes), with ordinary alpha and114×146 hover art. Historical Wardrobe additive overlays do not define the new frame.
- [x] Retail12.1 frame natural size1618×883; left outfit region312×860, central character658×860, right wardrobe644×860. Slot buttons59×59 show authoritative applied/pending appearance icons without the generic project button skin.
- [ ] Outfit list/save/load, Items/Sets/Custom Sets tabs, filters and illusion slots must match Retail12.1. Existing authoritative cost/Apply semantics remain unchanged; unavailable server-backed operations must not pretend to succeed.
- [x] Preview shows the equipped and pending main-hand weapon, independently of world equipment, with rotation and Sheathe/Draw controls. Weapon-cell camera fitting must show the blade profile rather than its end.
- [ ] Saved outfits follow the working core apply path; illusion UI requires authentic local illusion data.

## How it works

- [Transmog system](../wiki/systems/transmog-apply.md)
- Retail12.1 `Blizzard_Transmog/Blizzard_Transmog.xml` owns the1618×883 frame and three region sizes. `Blizzard_TransmogTemplates.xml` owns59×59 appearance slots and100×1323D item cards, replacing historical78×104 Wardrobe cells. The new outfit/list/tab/filter/illusion runtime contract is not interchangeable with the old Wardrobe dropdown. Revamp acceptance remains open until native both-skin proof and the outstanding controls are implemented.

## Implementation inventory

- `godot/ui-model/src/transmog.rs` — pending choices, quotes and application state.
- `godot/ui-model/src/transmog_frame.rs` — shared-skin native frame and slot/source picker.
- Sibling `shared-protocol/src/protocol/transmog_messages.rs` — authoritative quote/application wire contract.

## Tests asserting this spec

- `godot/ui-model/src/transmog_tests.rs` — pending versus applied state, exact prices, refusals, stale GUIDs,20-cell paging and independent source preview equipment.
- `godot/ui-model/tests/transmog_wardrobe.rs` — both-skin slot/model controls and collected/applied/pending labels.
- `godot/tests/transmog_wardrobe_live.gd` — actual skin/layout, main-hand profile pixels, composited collection pixels through applied/pending overlays, rotate/Sheathe, native search/restore, staging, exact paid Apply and Escape teardown on private55087.

## Known gaps (current cycle)

- [x] Native host/network integration: private55087, Hashom gossip, target2850 GUID123, sources821/1181, exact1500copper applications, self/observer rebuilds and relog in both configured art skins. Private-server restart preserved the same redb modifiers. [Bounded proof and evidence](../wiki/systems/transmog-apply.md#native-proof--verified-2026-10-11); this is not every-slot/product certification.
- [x] Both actual skins: isolated weapon models remain visible through pending/applied additive borders, character main-hand profile visible, native search/restore and exact1500copper Apply pass. [UI proof](../wiki/systems/transmog-apply.md#wardrobe-ui-proof--verified-2026-10-11). Paging is behaviorally verified with37sources/3pages; native accounts have5eligible weapon sources, not a native second-page census.
- [ ] Saved outfits remain unimplemented; core live application proof is available.
- [ ] Dynamic pricing, conditional source policies and illusion availability remain the linked server contract's explicit local-data gaps.

## Out of scope

- Fabricated appearance data/prices, Forever-only behavior/layout, world.db writes and port5000 mutations are prohibited for this task.
