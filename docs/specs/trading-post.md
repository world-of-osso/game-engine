# Trading Post

Retail Mainline Blizzard_PerksProgram and the local C_PerksProgram/PerksActivities API documentation define the vendor and Traveler's Log. Forever changes art only. All catalog rows derive from local CASC Retail 12.1.0.69933, with exact layout validation; no invented items, prices, availability or monthly rotations.

## What it must do

- [x] Decode authored categories, vendor items/prices, preview/UI group metadata and interval relations; retain unreadable counts and schema/DB2 provenance. Prepare SQL without writing world.db.
- [x] Track supported authored activity criteria through real gameplay events, account-wide, without historical login credit; retain unsupported criteria/conditions as named gaps.
- [x] Reset the Log and monthly Tender award cap at 00:00 UTC on each calendar month's first day, using the injectable server clock. Earn each sourced default group1 threshold once per account/month (lead policy October12). Preserve earned Tender, collection ownership and a frozen item across rollover.
- [x] Offer all sourced catalog rows unless an authored availability field can be verified; honor authored categories/UI groups. Unknown interval/calendar binding remains a named gap, not a fabricated rotation.
- [x] Vendor requests require an authentic capital NPC interaction and server-authoritative Tender balance. Purchases grant existing account collections atomically and persist across relog; one frozen vendor item survives rollover.
- [x] Refund requests are server-authoritative, policy-duration bounded and reverse only grants introduced by that purchase. Default policy duration is zero because no sourced duration exists (lead decision October12); Retail refund UI remains disabled.
- [x] Shared native full-screen vendor, PerksProgramFilter category dropdown, scrollable authored item list, transmog/mount/pet3D MainModelScene and Retail ToyOverlayFrame icon/details, purchase confirmation, freeze/drag-to-freeze/unfreeze and server-time-controlled refund. Both skins share layout/behaviour. Lead clarified October12: Retail source takes precedence over PLAN's shorthand 'category tabs'/'3D toys'.
- [x] Traveler's Log displays authoritative monthly Tender earned/cap/balance. No invented mail awards. Retail XML defines visible controls; remove unsourced vendor SearchBox, Log/Collections and Freeze/Unfreeze buttons (lead clarification October13).
- [x] Targeted behavioural tests and inspected private UDP5731 fb_* account PNG proof in Modern and Forever.

## Retail layout acceptance — October13

- [x] Both skins share ProductsScrollBoxContainer 527×751 at LEFT+46/CENTER+22, ScrollBox TOPLEFT(0,-58)/BOTTOMRIGHT(-24,120), stretched 176×48 product template rows with 32px icons at LEFT+48, and Filter LEFT+9/BOTTOM to container TOP+22. The 122×25 WowStyle2 template is overridden by PerksProgramProducts.lua to effective145×25 with fixed "Filter" text.
- [x] Native price/name sort headers, action-bar icon mask, source-sized frozen/hold art, shared large-button state art and StaticPopup confirmation; no solid fill replacing authored art. No unsourced search control/API remains.
- [ ] Time header: per-item remaining time has no sourced calendar binding; visible but disabled, not a fabricated duration.
- [x] Authored action-bar mask FDID4626072, hold-edge atlas FDID4331660 and button atlas FDID6794983 extracted from installed12.1.5.70391 and published by the lead October13. Positive native decoded/masked-pixel assertions gate visible art; the32px Icon samples the central32px of its centred48px MaskTexture rather than squeezing the full mask. Both-skin alpha comparisons cover the complete sampled output, with separate missing-source tests that never count as fidelity proof.
- [x] Before/after both-skin private UDP5791 PNG comparison against XML; 1920×1080 after captures and native freeze/replace/drag-off, purchase/refund, popup cancellation and Leave. Proof: `modern-after9.log`, `forever-after1.log` under engine `data/diagnostics/trading-post-ui-2026-10-13/`; PNGs under `/syncthing/AgentShared/2026-10-13/trading-post-layout/`. Refund proof uses explicit private policy3600s; production default stays zero.

## Fidelity rework — October13

Independent verification of `51f659ccd` failed D1–D6; earlier native gameplay passes are not Retail-fidelity acceptance. Background gradients select Orgrimmar only for the player's sourced Horde faction and Stormwind otherwise (PerksProgramProducts.lua:23-30). The native host uses the existing cached ChrRaces faction catalog, not a hardcoded race list. Item icons follow C_Item.GetItemIconByID base-appearance resolution when direct Item.IconFileDataID is zero; item223185 regression requires authored icon135411. Current-build `12.1.5.70391` art is staged privately for lead publication. Shared GameDialog item confirmation supplies source prompts, 128×21 dialog-box state art, item icon/name frame, source border and half-black full-screen cover; defaultbutton art is not used for Trading Post confirmation. Header labels/icons are LEFT-anchored with source normal/hover colors, Price is centered to Time, and freeze instructions/Leave arrow use the complete sourced text/markup dimensions. Effective filtering uses Collected/Not Collected/Useable Only checkboxes, multiple Type checkboxes and Sort By/Ascending/radio submenus. The reset callback restores filter flags, not sort state; frozen selection remains independent of the filtered list. Usability comes from the server's supported class/race/level requirements, with unknown source facts excluded only when Useable Only is selected. Projected and live fidelity gates remain open until D1–D6 re-verification.

## Retail continuation — October13

- [ ] Resolve live catalog delivery under production startup/request ordering (isolated1321-row UDP round-trip passes; private delivery intermittently exceeds60s).
- [x] Native multi-product cart and server-authoritative purchase/refund across protocol and persisted collections, sourced from Retail Lua; no invented policy. Modern and Forever checkout→new-client relog→individual refunds pass on one preserved private UDP5791 server/database; four receipts total6546 Tender, no panic/script-error markers. Proof: `cart-{modern,forever}-{identity-final,refund-identity-final}.log` in engine `data/diagnostics/trading-post-ui-2026-10-13/`, engine6b772c3a9/server4dcb1774/protocold9944f5.
- [x] Native Hide Armor and selected-item purchase-history widgets. Installed70391 UiModelScene Autodress bit4 supplies the default; user override survives selections and resets on vendor open. Both-skin native captures prove dressed/hidden changes, ownership after relog and each authoritative refund.
- [x] Retail footer rotation buttons:100×48 container BOTTOM(+140,+42),48×48 left/right buttons with26×25 sourced icons. Press holds camera rotation at sourced0.1 increment per model-scene update; release stops, including outside button bounds. Pushed art/icon offset is(+1,+1) in screen-down coordinates; hover art is additive at0.6 alpha. Toys/unavailable previews have no rotation controls. Both-skin process proof at engine3b1f5c2ea/serverbad076596/protocold9944f5: `rotate-registry-green-{modern,forever}.log` PASS, zero panic/script-error markers,8 PNGs each. Native registry-layout and decoded-icon gates pass separately; Hide Armor and drag rotation alone never certified this widget.
- [ ] Post-landing integration: repeat held controls and both-skin checkout→new-client relog→individual refunds with compatible binaries after the stats/Renown wire changes in protocol492ee28/server22ba4903/engine9a2b04cd4. Earlier clean proofs retain their exact revisions, not relabeled as latest-head acceptance.
- [ ] Remaining outfit/character controls: sourced mount TogglePlayerPreview; Hide Armor does not certify mounted-player composition.

## How it works

- [Existing monthly criteria service](../wiki/systems/adventure-guide.md#realm-activities-and-suggestions).

## Implementation inventory

- `scripts/trading_post_import.py`: qualified local DB2 decode and SQL/provenance preparation.
- `godot/ui-model/src/trading_post.rs`: shared vendor state/actions and source layout.
- `godot/rust/src/trading_post.rs`: native host, interaction/request lifecycle, detached previews and source-typed TryOn.
- `godot/network/src/lib.rs`, `godot/rust/src/account.rs`: reply registration/cache and requests.
- `adventure_guide.rs`, `adventure_activities.rs`: monthly Tender integration.
- `scripts/stage_trading_post_model_assets.py`: authenticated local-CASC preview asset closure (mount/pet displays, transmog/set item displays) into a private package with an additive receipt fragment; reuses `stage_mount_model_assets.stage_closure`.
- The open vendor hides every other client CanvasLayer except the tooltip (Retail `UIPanelWindows` area `full` hides UIParent) and restores them on close. Unavailable rows show their source gap instead of a model; a failed preview clears the previous model.

## Tests asserting this spec

- `scripts/tests/test_trading_post_import.py`: authentic category/price/UI group round-trip and mismatched-layout rejection.
- `godot/ui-model/tests/trading_post.rs`: both-skin XML geometry/dropdown, Tender/price/frozen/refund controls, modal confirmation/no local debit/pending state, freeze/replace/drag-off requests and second-page/category clipping.
- `godot/network/src/trading_post_wire_tests.rs`: real UDP snapshot/request round-trip.
- `scripts/tests/test_trading_post_asset_seeds.py`: server preview rules (mount/pet/transmog/set) and whole-chain receipt verdicts.
- `godot/tests/trading_post_live.gd`: private 127.0.0.1:5791 fb_* native-input proof — Log Tender line, authentic NPC gossip, HUD hidden/restored, category dropdown, player transmog/set and creature mount/pet previews, drag rotation, ToyOverlay, unavailable row, drag-to-freeze, unaffordable gating and frozen slot after relog.

## Known gaps (current cycle)

- [ ] Monthly featured catalog/seasonal reward calendar binding; no verified epoch.
- [ ] Refund duration; server-provided remaining time is documented but no duration sourced.
- [ ] Unknown PerksVendorItem fields and one unreadable vendor category; never infer semantics.
- [x] Live purchase/collection/relog/refund proof in both skins; Tender comes from the server admin-socket test grant because no supported activity has reachable private-world content (lead decision October12).
- [ ] Preview asset publication: the staged package raises complete authenticated chains from 167 to 1287 of 1309 previewable rows (transmog 60→910/911, mounts 98/98, pets 3→51/52, sets 6→228/248): 43,275 artifacts, 1.87 GB, zero extraction failures. Lead publishes it with `apply_mount_model_assets.py`. The other 22 rows have no sourced preview display (unreadable item or set metadata).
- [ ] Single-item ranged weapon previews (e.g. Battlefield Lover's Bow) are applied to the player's ranged slot but not clearly visible beside equipped melee weapons; Retail model-scene weapon framing is not reproduced.
- [ ] Rows whose ItemSparse/Item metadata is unreadable locally (e.g. vendor item for item235674) stay listed as unavailable with no name/icon.

## Out of scope

None of the user-requested Trading Post section is excluded. Source gaps are explicit unavailable cases, not substituted content.
