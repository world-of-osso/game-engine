# Watched reputation status bar

Retail Mainline `Blizzard_ActionBar/Shared/ReputationBar.lua`, Mainline `ReputationBarOverrides.lua`, `StatusTrackingManagerOverrides.lua`, `StatusTrackingBar.xml` and `StatusTrackingBarTemplate.xml` define behavior and geometry. Forever changes art only; the existing XP preset stays unchanged.

- Show only in world with an authoritative, nonempty watched faction. Clearing watch removes it; restore persisted watch at login without opening Journeys or fetching its reward catalog.
- XP priority4 precedes reputation priority1: reputation occupies the secondary container while XP is visible, then the main container when XP is capped/absent. Source containers571×17 with565×11 bar inset1px. Retail's secondary-container offset is17px relative to the first (Standard/EditModePresetLayoutConstants.lua:30), not a separate UIParent anchor. Modern stacks above the bottom-centered XP container. The approved Forever preset stacks below its existing top-centered XP container (top23 with XP, top6 without), leaving XP and action bars unchanged.
- Standard reputation displays normalized progress within the authoritative reaction tier, using the source red/orange/yellow/green fill. Capped Exalted uses the source full1/1 normalization. Major factions use blue and actual renown standing/threshold; capped renown retains its authoritative values, not a fabricated full fill.
- Hover shows `<name> <value> / <maximum>`; capped factions show name only. Leaving hides text. Source10px FrizQuadrata overlay is centered1px above the bar center.
- Authoritative owner-only `ReputationStateUpdate` carries the small watched-bar payload independently of the bulk Journeys catalog. Login, watch/clear acknowledgements and existing reputation updates keep it current. Refusals preserve previous watch; clients never optimistically update it.
- Both skins retain the same source containers, anchors, progress and text. Forever uses the existing supplied bronze border; no replacement art or invented unlock/collection state.

## Verification boundaries

Implementation and proof are recorded in [Adventure Guide](adventure-guide.md). Genuine positive Journeys watch eligibility remains unavailable while all pinned gates are unevaluated. Explicit evaluator fixtures cannot establish runtime eligibility. Native captures certify their exact executable and cases only.

## Remaining source behavior

Account-wide labels, friendship/paragon metadata and tooltip, gain/level/transition animation, other status bars, always-on text CVars and Edit Mode bar settings are not yet implemented; do not claim complete StatusTrackingBarManager parity.
