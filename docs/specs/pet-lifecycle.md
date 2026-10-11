# Combat pet lifecycle UI

## Contract

- Retail Mainline defines gameplay, controls and layout; Forever changes art only. StableFrame uses the installed Blizzard_StableUI1040×638 layout: stabled list left, selected pet/model right and five Call Pet positions in the footer. Capacity comes from the local Retail constants (5 active +200 stable, including secondary position).
- A real Stable Master hello/gossip choice opens the stable. Only server contents may change pet records; moves, rename, specialization and permanent release send owner-scoped requests. Pending operations cannot be duplicated.
- Stable/Make Active follows the Retail empty-position policy; active/stabled drag swaps are supported. Search/scroll must retain access to records beyond the first page. Locked Call Pet positions remain locked.
- Rename edits the selected pet's name. Release requires confirmation and permanently removes that record only after the server replies. Closing, NPC closure or leaving world clears the session/models; delayed replies cannot reopen it.
- Existing pet frame/bar reflect the owned pet. Hunter call/dismiss/revive/Mend and supported feeding, plus demon summon/dismiss/persistence, use server authority and authentic local spell data.

## Verification

Behavioral request/state tests and genuine private-server native mouse/cast/relog proof, inspected PNGs in both skins. No manufactured spell IDs, food classifications, pet counts or placeholder-success flows.
