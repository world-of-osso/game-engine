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
- [x] Paperdoll and illusion hover use current border ADD (`Blizzard_TransmogTemplates.lua:676–677,821–822`); state borders remain ordinary alpha. Native fixture asserts source tooltip content, set eight-piece content, menu-over-card nonactivation, both Custom Sets pages, icon editing, and fresh-process paid appearance/gold/icon relog; both-skin native browse and separate-process relog execution pass; [current sourced witnesses](../wiki/systems/transmog-apply.md#retail121-native-proof--verified2026-10-12).
- [x] Static portrait uses `transmog-icon-ui` (`Blizzard_Transmog.lua:113`), not a player bust. Set/custom-set hover uses `transmog-itemcard-default` at194×234 with ADD (`Blizzard_TransmogTemplates.xml:1103–1110`). Set cards have no inline title: XML1075–1275 defines none; `TransmogSetModelMixin:RefreshTooltip` owns the title instead.
- [x] Outfit row right-click → Edit Outfit opens the native IconSelector name/icon editor; Okay sends `CommitOutfitInfo`-equivalent metadata without paid application. The palette contains authentic offered appearance icons and the sourced question-mark entry134400 (`IconDataProvider.lua:2,74–80`); the global spell/macro icon catalogue is not claimed. Edit restores the saved selection (`Blizzard_Transmog.lua:750–751`).
- [x] Saved outfit rows draw required protocol `icon_fdid`; migrated records with icon0 stay empty. No Retail missing-icon fallback was found.
- [x] Retail12.1 item state art is `transmog-itemcard-transmogrified` (pending suffix for staged changes), with ordinary alpha and114×146 hover art. Historical Wardrobe additive overlays do not define the new frame.
- [x] Retail12.1 frame natural size1618×883; left outfit region312×860, central character658×860, right wardrobe644×860. Slot buttons59×59 show authoritative applied/pending appearance icons without the generic project button skin.
- [x] Native Items/Sets/Custom Sets tabs, outfit list/save/load and illusion selection use Retail12.1 elements. Save commits pending appearances through the unchanged paid Apply before saving configuration; unavailable operations do not pretend to succeed.
- [x] Sources uses authored `Enum.TransmogSource` categories1/2/3/4/7/8/10, check-all/uncheck-all and filtering before paging. Hidden pending choices retain their exact paid quotes. Sets completion filters distinguish Collected/Not Collected; PvE/PvP classification stays disabled until an authoritative or exhaustively verified flag mapping exists.
- [x] Preview shows the equipped and pending main-hand weapon, independently of world equipment, with rotation and Sheathe/Draw controls. Weapon-cell camera fitting must show the blade profile rather than its end.
- [x] Saved outfits follow the working core apply path; illusion UI uses authentic TransmogIllusion/SpellItemEnchantment names, icons and default-collected identities.

## How it works

- [Transmog system](../wiki/systems/transmog-apply.md)
- Retail12.1 `Blizzard_Transmog/Blizzard_Transmog.xml` owns the1618×883 frame and three region sizes. `Blizzard_TransmogTemplates.xml` owns59×59 appearance slots and100×1323D item cards, replacing historical78×104 Wardrobe cells. Native both-skin proof covers the supported outfit/list/tab/filter/illusion contract; retained classification and acquisition gaps below prevent a full Retail API-parity claim.

## Implementation inventory

- `godot/ui-model/src/transmog.rs` — pending choices, quotes and application state.
- `godot/ui-model/src/transmog_frame.rs` — shared-skin native frame and slot/source picker.
- Sibling `shared-protocol/src/protocol/transmog_messages.rs` — authoritative quote/application wire contract.

## Tests asserting this spec

- `godot/ui-model/src/transmog_tests.rs` — pending versus applied state, exact prices, refusals, stale GUIDs,20-cell paging and independent source preview equipment.
- `godot/ui-model/tests/transmog_wardrobe.rs` — both-skin slot/model controls and collected/applied/pending labels.
- `godot/tests/transmog_wardrobe_live.gd` — actual skin/layout, main-hand/card profile pixels, composited alpha/hover overlays, rotate/Sheathe, search/source filters, set membership/completion filters/staging, paid outfit/illusion save/load, native second-page Custom Set save/load and Escape teardown on private5926.

## Known gaps (current cycle)

- [x] Native host/network integration: private55087, Hashom gossip, target2850 GUID123, sources821/1181, exact1500copper applications, self/observer rebuilds and relog in both configured art skins. Private-server restart preserved the same redb modifiers. [Bounded proof and evidence](../wiki/systems/transmog-apply.md#native-proof--verified-2026-10-11); this is not every-slot/product certification.
- [x] Both actual skins: weapon models remain visible through Retail12.1 alpha state borders and additive hover; character main-hand profile, native source/search filters and exact1500-copper Apply pass. [Current UI proof](../wiki/systems/transmog-apply.md#retail121-native-proof--verified2026-10-12). Item paging is CPU-tested with41sources/3pages; native5-source item collection is not an every-slot/second-item-page claim. Custom Sets has real native second-page proof.
- [x] Private5926 native both-skin outfit/illusion save/load, exact1500-copper Apply, collected/uncollected set membership and presets are proven. [Current UI proof](../wiki/systems/transmog-apply.md#retail121-native-proof--verified2026-10-12).
- [ ] PvE/PvP set-classification mapping is not authoritative or exhaustively verified; those filter rows remain disabled. Purchased outfit slots, Situations and event/conditional acquisition eligibility remain unavailable or uncertified.
- [ ] Illusion selection/save/load is proven as outfit metadata, not visual enchant effects, equipped enchant application or illusion-price settlement.
- [ ] Dynamic pricing, conditional source policies and illusion availability remain the linked server contract's explicit local-data gaps.

## Out of scope

- Fabricated appearance data/prices, Forever-only behavior/layout, world.db writes and port5000 mutations are prohibited for this task.
