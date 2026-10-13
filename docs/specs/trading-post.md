# Trading Post

Retail Mainline Blizzard_PerksProgram and the local C_PerksProgram/PerksActivities API documentation define the vendor and Traveler's Log. Forever changes art only. All catalog rows derive from local CASC Retail 12.1.0.69933, with exact layout validation; no invented items, prices, availability or monthly rotations.

## What it must do

- [ ] Decode authored categories, vendor items/prices, preview/UI group metadata and interval relations; retain unreadable counts and schema/DB2 provenance. Prepare SQL without writing world.db.
- [ ] Track supported authored activity criteria through real gameplay events, account-wide, without historical login credit; retain unsupported criteria/conditions as named gaps.
- [ ] Reset the Log and monthly Tender award cap at 00:00 UTC on each calendar month's first day, using the injectable server clock. Earn each sourced default group1 threshold once per account/month (lead policy October12). Preserve earned Tender, collection ownership and a frozen item across rollover.
- [ ] Offer all sourced catalog rows unless an authored availability field can be verified; honor authored categories/UI groups. Unknown interval/calendar binding remains a named gap, not a fabricated rotation.
- [ ] Vendor requests require an authentic capital NPC interaction and server-authoritative Tender balance. Purchases grant existing account collections atomically and persist across relog; one frozen vendor item survives rollover.
- [ ] Refund requests are server-authoritative, policy-duration bounded and reverse only grants introduced by that purchase. Default policy duration is zero because no sourced duration exists (lead decision October12); Retail refund UI remains disabled.
- [ ] Shared native full-screen vendor, PerksProgramFilter category dropdown, scrollable authored item list, transmog/mount/pet3D MainModelScene and Retail ToyOverlayFrame icon/details, purchase confirmation, freeze/drag-to-freeze/unfreeze and server-time-controlled refund. Both skins share layout/behaviour. Lead clarified October12: Retail source takes precedence over PLAN's shorthand 'category tabs'/'3D toys'.
- [ ] Traveler's Log displays authoritative monthly Tender earned/cap/balance; vendor Log and Collections buttons close the vendor interaction before opening existing windows. No invented mail awards.
- [ ] Targeted behavioural tests and inspected private UDP5731 fb_* account PNG proof in Modern and Forever.

## How it works

- [Existing monthly criteria service](../wiki/systems/adventure-guide.md#realm-activities-and-suggestions).

## Implementation inventory

- `scripts/trading_post_import.py`: qualified local DB2 decode and SQL/provenance preparation.
- `godot/ui-model/src/trading_post.rs`: shared vendor state/actions and source layout.
- `godot/rust/src/trading_post.rs`: native host, interaction/request lifecycle, detached previews and source-typed TryOn.
- `godot/network/src/lib.rs`, `godot/rust/src/account.rs`: reply registration/cache and requests.
- `adventure_guide.rs`, `adventure_activities.rs`: monthly Tender integration.

## Tests asserting this spec

- `scripts/tests/test_trading_post_import.py`: authentic category/price/UI group round-trip and mismatched-layout rejection.
- `godot/ui-model/tests/trading_post.rs`: both-skin Tender/price/frozen/refund controls, modal confirmation/no local debit/pending state and second-page/search clipping.
- `godot/network/src/trading_post_wire_tests.rs`: real UDP snapshot/request round-trip.

## Known gaps (current cycle)

- [ ] Monthly featured catalog/seasonal reward calendar binding; no verified epoch.
- [ ] Refund duration; server-provided remaining time is documented but no duration sourced.
- [ ] Unknown PerksVendorItem fields and one unreadable vendor category; never infer semantics.
- [ ] Server/protocol/native integration and live proof in progress.

## Out of scope

None of the user-requested Trading Post section is excluded. Source gaps are explicit unavailable cases, not substituted content.
