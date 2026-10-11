# Encounter timeline and warnings

Retail Mainline `Blizzard_EncounterTimeline`, `Blizzard_EncounterWarnings` and generated EncounterTimeline/Constants/EncounterWarnings APIs define the feature. Forever changes art, not elements or behavior.

## Contract
- Server sends only actual selected SmartAI UPDATE_IC cast timers and script EventMap spell schedules. No cooldown estimates, fabricated mechanics, or deadlines for aura/health conditions. Conditional timers are not promises: chance, target, phase and casting can defer/skip them.
- Accepted casts send severity (Low absent authored severity), authored SET_EVENT_PHASE sends phase changes. Events share the reliable ordered encounter lifecycle channel and are restricted to the dungeon copy.
- Timer snapshots replace one caster's timers; empty snapshots cancel. End, Start, loading/disconnect/reset clear timers and warnings. Countdown reaching zero never creates a warning: only a received real cast does.
- Bars show spell icon/name and remaining time, nearest-first, at most ten, with five-second imminent highlighting. Track view moves icons toward zero. Both use the same scheduler data and settings.
- Warnings replace the current line at their severity, display spell text and flanking icons, optionally play the cached Retail RaidBossEmote sound kit12197. Presentation hold3s is not an ability timer. Exact Retail severity→native API sound mapping is unavailable in cached Lua; do not claim that mapping.
- Interface settings persist timeline enable, Bars/Timeline view, warning enable and sound enable. Edit Mode moves EncounterTimeline and each of the three severity warning regions; settings/positions apply to both skins.

## Sources / acceptance
Cached `EncounterTimelineTimerView.xml` (300×300/max10), `EncounterTimelineTimerEvent.xml` (34px events/26px fill), `EncounterTimelineConstants.lua` (spacing2/highlight5), generated view enum Timeline1/Bars2; `EncounterWarnings.xml` (600×48/550×36/500×36), `EncounterWarningsView.lua` (per-severity replacement, expiration, sound), `Blizzard_SharedXML/Mainline/SoundKitConstants.lua:90` (12197).

Targeted UI state tests prove receipt-driven countdown, caster replacement, phase/reset and warning metadata. Server tests prove selected Cobrahn timer→actual cast, copy isolation/evade and Hogger script deadlines. Native acceptance remains pending until both-skin private UDP5850 countdown→real-cast/warning PNGs are inspected; no source/unit test substitutes for live proof.
