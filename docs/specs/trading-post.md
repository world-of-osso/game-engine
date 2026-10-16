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

## Accepted single-item confirmation lifetime

- Purchase/Refund acceptance retains its real confirmation owner while a request is outstanding: both buttons disabled, shared dark overlay and decoded rotating spinner painted above dialog contents immediately (Retail `StaticPopup_OnAcceptWithSpinner`, delay0). GameDialog.xml296–320 supplies the overlay's11px inset and spinner60×60 center placement. Native proof must show painted output changes, not merely existing or rotating nodes. Duplicate Confirm/Cancel/Escape cannot enqueue a second transaction or mutate Tender/ownership.
- Purchase acceptance removes only that vendor product from the local cart, per Elements.lua1–4; other members remain. Authoritative snapshot receipt ends the existing request-pending state and dismisses its dialog; balances/collection never update optimistically.
- Per-item delivery-pending,45-second slow/server-error dialogs and richer success-event correlation remain separate provider/lifecycle gaps. This request-modal slice does not claim those states or change production refund policy.
- [x] Model2 and actual missing-owner/static-spinner/inset RED controls precede both-skin natived545c8310/a820dfda/dc30250 painted GREEN: real paused-private-server reply boundary, rotating visible spinner/overlay counterfactuals, source inset, disabled controls, cart pruning, duplicate rejection and one purchase/refund round trip per skin. Ten full-HD originals+two contacts+two unscaled crops: `/syncthing/AgentShared/2026-10-16/trading-post-pending-dialogs/`; diagnostic manifest binds exact inputs. Native uses each account's actual starting balance; earlier RED purchases are retained in private DB, never reset.

## Confirmation keyboard ownership

- Purchase, Refund, Purchase Cart, Clear Cart and frozen replacement confirmations consume Escape without dismissal or requests. Their Retail Elements.lua definitions omit `hideOnEscape`; GameDialog.lua737–745's keyboard cover hides only when that flag exists. Explicit Cancel remains available and preserves Tender/purchase/cart state before acceptance.
- Purchase/refund authority remains unchanged: one accepted request changes the server balance/collection; Cancel/Escape do not. Production refund policy stays zero; native refund inspection may use the existing explicit private3600-second test policy, never an invented production window.
- This subsection changes keyboard dismissal only; [accepted request-modal lifetime](#accepted-single-item-confirmation-lifetime) owns spinner coverage and remaining provider/error/timeout gaps. Existing earlier layout proof's claim that Escape cancels these confirmations was incorrect; native fixtures now require source-preserved modal plus explicit Cancel.
- [x] Actual premature purchase-dismissal RED→both-skin native7a25a4cd7/a820dfda/dc30250 GREEN for all four non-freeze covers, explicit Cancel and exact decoded Purchase/Refund item-frame art. One authorized purchase/refund per skin restores Tender19670→19340→19670 under private policy3600 only. Twelve full-HD originals+two contacts: `/syncthing/AgentShared/2026-10-16/trading-post-confirmations/`; diagnostic `confirmations-proof-manifest.json` pins actual artifacts/fixtures/receipts.

## Cart item tooltips

- Regular cart items and ensemble member rows inherit Retail Elements.lua1603–1623 details-row `GetItemByID` tooltip behavior: actual hovered item ID/name/quality/catalog details, no sell price, `ANCHOR_LEFT` X−8/Y−20 (UI y-down offset−8/+20).
- Resolve the exact currently expanded cart owner, not the selected product/model. Ensemble headers, members belonging to another vendor, removed rows and closed panels do not supply item tooltips. Hover cannot change cart members/preview selection or authoritative Tender; central pointer leave/occlusion/teardown clears ownership.
- Preserve explicit missing metadata errors; use the shared Retail item formatter in both art skins. Frozen/detail panes/currency/cart-toggle tooltips and item-effect formatting remain separate.

## Button tooltips

- Purchase, View Cart, Clear Cart, Purchase Cart and disabled Refund use Retail Elements.lua767–782/832–838/869–873/2065–2070/2099–2112 `ShowTooltip` branches. Tooltip owner is the actual projected button at `ANCHOR_LEFT`, offsets0/0; exact labels come from GlobalStrings70391.
- Purchase/Checkout show their action when enabled; when disabled, insufficient Tender takes the gold normal-text explanation, otherwise white “Purchasing Unavailable”. View Cart remains inspectable while disabled; disabled Clear Cart and enabled Refund produce no tooltip. Pointer exit, covering dialog and vendor teardown clear the old owner. No request or balance/selection mutation follows from hover.
- NAMED GAP: per-item `isPurchasePending` is absent from the current wire. Do not map generic request-pending state to the source's two-line delivery-pending tooltip; keep that provider branch unsupported. Currency/cart-toggle/frozen/detail-row tooltips remain separate.
- [x] Native missing-tooltip RED→both-skin GREEN atb282d3793/a820dfda/dc30250 with fixture4965a8ca4: actual View Cart disabled/enabled, Purchase/Clear/Checkout, left anchors, hover leave/modal occlusion/cart teardown and unchanged selected712/Tender19670. Model2 covers remaining disabled/affordability branches; native empty-cart/disabled-refund states were not forced. Sixteen inspected full-HD originals+four contacts: `/syncthing/AgentShared/2026-10-16/trading-post-button-tooltips/`. Fixture: `trading_post_live.gd`, phase `button-tooltips`; content: `trading_post_tooltip.rs`.

## Product-row item tooltips

- Hovering a vendor product shows that hovered row's Retail item ID, quality/name and catalog item details, independently of the selected model preview. Use the shared native tooltip renderer, not a product-description substitute.
- Retail PerksProgramElements.lua306–314 uses `GetItemByID`, excludes `SellPrice`, and owns the tooltip at `ANCHOR_RIGHT` with X−16/Y0. Preserve the shared screen clamp; hide on pointer leave or vendor teardown. Hover must not change selection, Tender or server state.
- [x] Native missing-tooltip RED→both-skin GREEN at engine569419cb7/servera820dfda/protocoldc30250. Fixture `godot/tests/trading_post_live.gd`, phase `item-tooltips`, verifies two hovered item identities/details/right−16 positions without sell price, unchanged selected row/Tender, pointer leave and Escape/vendor teardown/reopen/HUD restoration. Eight inspected full-HD originals+two contacts: `/syncthing/AgentShared/2026-10-16/trading-post-item-tooltips/`; diagnostic `item-tooltips-proof-manifest.json` pins actual artifact hashes and receipts. Product-row coverage does not certify frozen/cart/detail-row or footer-button tooltips, item spell-effect formatting, or framing.

## Menu lifecycle sounds

Source: Retail PerksProgram.lua226/253 and SoundKitConstants.lua, with SoundKit/SoundKitEntry12.1.5.70391. [Implementation/proof inventory](../wiki/systems/trading-post.md#menu-lifecycle-sounds--bounded-proof-october15); behavioral tests: `godot/core/tests/trading_post_menu_sound.rs`, `godot/tests/trading_post_live.gd` (`menu-sounds`).

- [x] Actual menu show/hide plays OPEN218932/CLOSE218933 once per visibility edge via the existing UI SoundKit player; repeated updates/closed state do not replay either kit. A menu never projected must not emit a close sound.
- [x] Use the five authored open files at gain0.562341332 and ten authored close files at gain0.01, preserving existing master/effects/mute settings and explicit asset/decode failures. Native proof must observe real decoded AudioStreamPlayer playback, not only requested kit IDs.
- [ ] NAMED GAP — close kit218933 pitch adjustment/variation is unsupported by the existing UI kit player. Lead October15 authorizes lifecycle/files/gain only; do not add a pitch path or claim complete acoustic parity in this slice. Hover/selection/purchase/refund/rotation sounds remain outside it.

## Named gap — native pet framing

- [ ] No allowed oracle currently establishes native ModelScene default FOV, inset projection semantics or pet DesiredScale provider. Garrlok224 clips45.5 physical pixels at the left edge; Spectral Tiger Cub429 fits. Scene-only pet/player work remains unlanded and Rank3 incomplete until authentic framing is resolved. Do not apply calculated fit thresholds, tune a reframe, hide clipped pets or substitute an unapproved source. Allowed evidence: Retail BlizzardInterfaceCode cache, ModelScene Actor/Camera CSVs12.1.5.70391, WebWowViewerCpp and wow.export. Lead October15 accepted this as a named gap, not a framing fix.

## Authored base camera/orbit

- Trading Post uses the selected display scene's authored UiModelSceneCamera base target, yaw/pitch/roll and zoom/min/max, not mesh-bounds fitting. Product overrides select their first camera even when its tag is `override` rather than `primary`; otherwise use the category display scene. Display translation rounds targets and angles to one decimal place (angles in degrees before conversion back to radians).
- Held footer buttons and left drag orbit the actual camera while authored actor transforms remain unchanged; release outside stops rotation. Mouse wheel changes the source zoom percentage within authored min/max. Source: Products.lua279–364, Model.lua225–238, OrbitCameraMixin33–55,395–438,480–505,574–579 and ModelSceneMixin328–337.
- Lead October15 authorized base-camera/orbit acceptance separately from inherited two-point native zoom splines and ModelScene default-FOV equivalence. Existing project FOV remains unchanged; neither gap may be represented as exact Retail framing or replaced with a guessed formula.
- [x] Installed-data4 tests and compatible generic-camera native RED→both-skin GREEN at engine57d522994/server50fcd1c5/protocol7313ec4. Actual authored619 base pose, camera-only held/drag orbit, wheel zoom and rider/category/HUD lifetimes pass.24 inspected full-HD PNGs: `/syncthing/AgentShared/2026-10-15/trading-post-camera/`; exact artifact hashes/revisions and six PSI/swap boundaries in diagnostic `camera-proof-manifest.json`. Dedicated pet/player comparison remains separate.

## Frozen replacement consent

- Occupied-slot drops preview the candidate in the slot while a Retail GameDialog identifies the old frozen product and warns: “Are you sure you want to replace your current frozen item? The existing frozen item may no longer be available.”
- Confirm alone stays disabled for five monotonic seconds, showing the rounded-up remaining seconds, then “Confirm”. Cancel is immediately available; global right mouse-down also cancels. Escape is consumed, not cancellation (this popup has no hideOnEscape).
- Cancel restores the authoritative frozen slot and preserves local cart order. Acceptance sends one Freeze request and removes only its candidate from the cart; the snapshot remains server-authoritative. Empty-slot freeze is immediate with the same cart pruning. No Tender mutation.
- Effective frozen-slot geometry honors `FrozenProductContainerMixin:OnLoad` (Products.lua844–854), not the ordinary row's inherited sizes: Icon64×64 at LEFT+32, centered95×95 mask, Label233×45 at Icon.RIGHT+8. FrozenArtContainer follows the icon; XML's centered FrostFrame64 and pending ItemGlow88 keep their authored sizes. Settled frost frame stays inside the84px row and leaves8px before the label; do not shrink decorative art to compensate for an incorrectly sized icon.
- [x] Frozen-slot overlap regression: geometry and decoded real-icon alpha RED→GREEN; both-skin old32px-icon before versus effective64px-icon after passes native layout/art, pending glow and cancel/Tender checks.16 full-HD originals+8 unscaled paired crops at `/syncthing/AgentShared/2026-10-15/trading-post-frozen-geometry/`; diagnostic `frozen-geometry-proof-manifest.json` retains exact before/after binary hashes/revisions and12 memory boundaries. After native67db13531/server852ec57f/protocol62865c2, fixture7f4ed39a9.
- Sources: Blizzard_PerksProgramElements.lua87–100,583–670; Blizzard_PerksProgram.lua274–278; StaticPopup.lua525–538; GameDialog.lua737–745.
- [x] Native RED→both-skin GREEN at engineb09e0a70d/server7356cad8/protocolcac6080: delay/early click, Cancel, Escape, right mouse-down, replacement/empty-slot cart pruning and unchanged Tender.20 inspected full-HD PNGs: `/syncthing/AgentShared/2026-10-15/trading-post-freeze/`; diagnostic `freeze-proof-manifest.json` retains original revisions/hashes and six PSI/swap boundaries.

## Retail layout acceptance — October13

- [x] Both skins share ProductsScrollBoxContainer 527×751 at LEFT+46/CENTER+22, ScrollBox TOPLEFT(0,-58)/BOTTOMRIGHT(-24,120), stretched 176×48 product template rows with 32px icons at LEFT+48, and Filter LEFT+9/BOTTOM to container TOP+22. The 122×25 WowStyle2 template is overridden by PerksProgramProducts.lua to effective145×25 with fixed "Filter" text.
- [x] Native price/name sort headers, action-bar icon mask, source-sized frozen/hold art, shared large-button state art and StaticPopup confirmation; no solid fill replacing authored art. No unsourced search control/API remains.
- [ ] Time header: per-item remaining time has no sourced calendar binding; visible but disabled, not a fabricated duration.
- [x] Authored action-bar mask FDID4626072, hold-edge atlas FDID4331660 and button atlas FDID6794983 extracted from installed12.1.5.70391 and published by the lead October13. Positive native decoded/masked-pixel assertions gate visible art; the ordinary32px Icon samples the central32px of its centred48px MaskTexture rather than squeezing the full mask. Both-skin alpha comparisons cover the complete sampled output, with separate missing-source tests that never count as fidelity proof.
- [x] Before/after both-skin private UDP5791 PNG comparison against XML; 1920×1080 after captures and native freeze/replace/drag-off, purchase/refund, popup cancellation and Leave. Proof: `modern-after9.log`, `forever-after1.log` under engine `data/diagnostics/trading-post-ui-2026-10-13/`; PNGs under `/syncthing/AgentShared/2026-10-13/trading-post-layout/`. Refund proof uses explicit private policy3600s; production default stays zero.

## Fidelity rework — October13

Independent verification of `51f659ccd` failed D1–D6; earlier native gameplay passes are not Retail-fidelity acceptance. Background gradients select Orgrimmar only for the player's sourced Horde faction and Stormwind otherwise (PerksProgramProducts.lua:23-30). The native host uses the existing cached ChrRaces faction catalog, not a hardcoded race list. Item icons follow C_Item.GetItemIconByID base-appearance resolution when direct Item.IconFileDataID is zero; item223185 regression requires authored icon135411. Current-build `12.1.5.70391` art is staged privately for lead publication. Shared GameDialog item confirmation supplies source prompts, 128×21 dialog-box state art, item icon/name frame, source border and half-black full-screen cover; defaultbutton art is not used for Trading Post confirmation. Header labels/icons are LEFT-anchored with source normal/hover colors, Price is centered to Time, and freeze instructions/Leave arrow use the complete sourced text/markup dimensions. Effective filtering uses Collected/Not Collected/Useable Only checkboxes, multiple Type checkboxes and Sort By/Ascending/radio submenus. The reset callback restores filter flags, not sort state; frozen selection remains independent of the filtered list. Usability comes from the server's supported class/race/level requirements, with unknown source facts excluded only when Useable Only is selected. Projected and live fidelity gates remain open until D1–D6 re-verification.

## Retail continuation — October13

- [ ] Resolve live catalog delivery under production startup/request ordering (isolated1321-row UDP round-trip passes; private delivery intermittently exceeds60s).
- [x] Native multi-product cart and server-authoritative purchase/refund across protocol and persisted collections, sourced from Retail Lua; no invented policy. Modern and Forever checkout→new-client relog→individual refunds pass on one preserved private UDP5791 server/database; four receipts total6546 Tender, no panic/script-error markers. Proof: `cart-{modern,forever}-{identity-final,refund-identity-final}.log` in engine `data/diagnostics/trading-post-ui-2026-10-13/`, engine6b772c3a9/server4dcb1774/protocold9944f5.
- [x] Native Hide Armor and selected-item purchase-history widgets. Installed70391 UiModelScene Autodress bit4 supplies the default; user override survives selections and resets on vendor open. Both-skin native captures prove dressed/hidden changes, ownership after relog and each authoritative refund.
- [x] Retail footer rotation buttons:100×48 container BOTTOM(+140,+42),48×48 left/right buttons with26×25 sourced icons. Press holds camera rotation at sourced0.1 increment per model-scene update; release stops, including outside button bounds. Pushed art/icon offset is(+1,+1) in screen-down coordinates; hover art is additive at0.6 alpha. Toys/unavailable previews have no rotation controls. Both-skin process proof at engine3b1f5c2ea/serverbad076596/protocold9944f5: `rotate-registry-green-{modern,forever}.log` PASS, zero panic/script-error markers,8 PNGs each. Native registry-layout and decoded-icon gates pass separately; Hide Armor and drag rotation alone never certified this widget.
- [x] Post-landing integration: engine8b44a3c97/server22ba4903b/protocol492ee2833 compatible binaries pass held controls and both-skin checkout→new-client relog→individual refunds. Four products cost6546 Tender; both balances return19670, ownership revokes individually, zero panic/script-error markers. Modern also survives the admission-window server restart between checkout and relog/refund; Forever uses one server lifetime. Proof: `postlanding-rotate-{modern,forever}.log`, `postlanding-cart-ready-modern-cart.log`, `postlanding-cart-owned-window-{modern-refund,forever-cart,forever-refund}.log`;86 inspected1920×1080 PNGs under `/syncthing/AgentShared/2026-10-14/trading-post-postlanding/`. Each phase has PSI/swap start/end evidence; source revisions remain explicit.
- [x] Ordinary kit0 mount TogglePlayerPreview (Hide Character): mount-only checkbox, initially unchecked; retain the user's choice across product/category changes and server snapshots within the client. Checked hides only the rider, leaving the mount visible without refitting its camera; unchecked restores the current dressed, weapon-hidden player in the sourced mounted pose and attachment. Honor self-mount/disabled-player-preview metadata and authored UiModelScene actor transforms/scale; never substitute a standing unattached player or guessed flags. Behavioral/data and both-skin process proof: native engineb23615e57/server804a042d4/protocol4135e6975, `rider-stable-b236-{modern,forever}.log`,20 inspected1920×1080 PNGs under `/syncthing/AgentShared/2026-10-14/trading-post-rider/`. Celestial Steed vendor223/mount376 proves seated91/saddle0, weapon-hidden dressed rider, stable-camera hide/show, retained choice, category teardown, close/reopen and HUD restoration; no purchase/refund replay. Hide Armor does not certify this composition.
- [x] Close during an actual asynchronous rider request: observe pending before attachment, then60 frames with no late actor/scene and restored HUD in both skins. `rider-pending-c90-{modern,forever}.log`; four inspected full-HD PNGs under `/syncthing/AgentShared/2026-10-15/trading-post-rider-pending/`. Nativeb23615e57/CLI1b2ce4210, fixturec90ed62b3, server81a06baf/protocol4135e6975.
- [ ] Exact native CalculateMountScale equivalence and nonzero rider animation/spell-visual kits; retain the explicit source boundary below, not an ordinary-pose substitution.

### Rider preview source boundary — October14

UiModelSceneActor/Display metadata controls mount/player-rider origin, orientation and normalization. Scene418 uses mount1212 (flags9, normalized0.25, requested3.5) and player-rider1214 (flags0, normalized0, requested1). MountFlags2/4096 suppress rider creation. Ordinary riders use attachment0 and seated animation91, equipped armor and no weapons; Hide Character cancels pending rider work and frees only that actor. Selection/close cancels both detached lifetimes.

Retail SharedXML calls native `CalculateMountScale`; its formula remains unestablished. Lead decision21:36: retain authored rider scale1.0, no invented compensation. Installed CreatureModelData MountScaleSelf/Other/OtherIndex/MountHeight/ModelScale, CreatureDisplayInfo CreatureModelScale/PlayerOverrideScale and MountFlyRideHeight are not permission to infer the formula. Neither inspected WebWowViewerCpp nor Trinity DB2 field declarations establish its application. Exact mount/rider scale parity remains a named gap. Nonzero rider animation/visual kits are explicitly unavailable, never replaced with the ordinary seated pose. Ordinary kit0 composition has the bounded process proof above; it does not certify the unresolved native scale API or special kits.

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
