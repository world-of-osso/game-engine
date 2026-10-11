# Transmog application

Native Retail Mainline Transmogrify service for collected appearances. [Authoritative server contract](../../../game-server/docs/specs/transmog-apply.md). Forever is an art skin only; both skins use the same window, choices and application semantics.

## What it must do

- [x] Transmogrifier NPC/gossip opens the Transmogrify frame with collected, locally supported eligible per-slot appearance choices.
- [x] Stage pending changes without modifying replicated equipment; preview selected appearance and show the exact authoritative gold quote.
- [x] Apply only affordable locally defined quotes; disable duplicate application while awaiting server acceptance and show explicit refusal errors.
- [x] Show original appearance restoration and hide choices only for server-authorized slots.
- [x] Preserve pending-state safety across item replacement, interaction closure and character/session changes.
- [x] Native self and observer displays update after successful application and remain after relog, in both skins.
- [ ] Saved outfits follow the working core apply path; illusion UI requires authentic local illusion data.

## How it works

- [Transmog system](../wiki/systems/transmog-apply.md)
- Local Retail `Blizzard_Transmog.xml` defines the three-region frame (outfit312, character658, wardrobe644; total1618×883). `Blizzard_Transmog.lua` owns pending-cost/apply semantics.

## Implementation inventory

- `godot/ui-model/src/transmog.rs` — pending choices, quotes and application state.
- `godot/ui-model/src/transmog_frame.rs` — shared-skin native frame and slot/source picker.
- Sibling `shared-protocol/src/protocol/transmog_messages.rs` — authoritative quote/application wire contract.

## Tests asserting this spec

- `godot/ui-model/src/transmog_tests.rs` — pending versus applied state, exact prices, refusals and stale GUIDs.

## Known gaps (current cycle)

- [x] Native host/network integration: private55087, Hashom gossip, target2850 GUID123, sources821/1181, exact1500copper applications, self/observer rebuilds and relog in both configured art skins. Private-server restart preserved the same redb modifiers. [Bounded proof and evidence](../wiki/systems/transmog-apply.md#native-proof--verified-2026-10-11); this is not every-slot/product certification.
- [ ] Source cells currently expose authentic appearance icons, not Retail 3D thumbnail scenes.
- [ ] Saved outfits remain unimplemented; core live application proof is available.
- [ ] Dynamic pricing, conditional source policies and illusion availability remain the linked server contract's explicit local-data gaps.

## Out of scope

- Fabricated appearance data/prices, Forever-only behavior/layout, world.db writes, master merges and port5000 mutations are prohibited for this task.
