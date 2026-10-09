# Button text-fit audit — 2026-10-09

Branch `buttonfit`; original base `9dea9afa7`. Runtime/code proof at `b27a710fc`; subsequent documentation-only commits do not invalidate it. Full working ledger: `/home/osso/.worktrees/handoff-buttonfit.md`.

## Findings and Retail fixes

- `ProfessionsCreateAll`: "Create All [0]" measures 74px, but the original 80px button leaves only 40px after Retail fit padding. Retain the count and GameFontNormal12; use measured text + 40px, preserving the right anchor. Counts 0/7/100/9999 produce 114/114/129/137px. Cached Retail `Blizzard_Professions/Blizzard_ProfessionsCrafting.lua:698`, `.xml:219-224`; inherited `Blizzard_SharedXML/SecureUIPanelTemplates.lua:83-91`, `.xml:39-43`. [Contract](../specs/professions-frame.md).
- `AuctionPagePrev` / `AuctionPageNext`: custom "Prev page" / "Next page" text measures 65/69px in 64px buttons. Replace these text panels with Retail 32×32 page-arrow states and an independent Results summary, with 5px gaps. Existing protocol actions and already-fitting local row paging remain unchanged. Cached Retail `Blizzard_PagedContent/Blizzard_PagingControls.xml:4-25,82-108`. Mainline AuctionHouse uses incremental scrolling; this is its reusable Retail paging control, not a claim Mainline AH contains our protocol pager. [Contract](../specs/auction-house-ui.md#native-search-paging-button-fit).

## Evidence

- Ignored `launcher/tests/buttonfit_audit.rs` runs `godot/tests/buttonfit_audit.gd`: native offline preview inventory, both skins, actual 1920×1080 layout and projected font metrics. Supplemental merchant/buyback/mail/trade/guild-bank/character/menu/Options cases and existing quest fixtures included.
- RED native audit: 138 cases, 3,638 labels, 52 overflow occurrences across the three controls above. Exact sorted list in external handoff and `audit-red.log`.
- Final GREEN native audit: 190 cases, 5,295 labels, zero unclipped overflows, zero fixture failures. All 66 intentional wrapped/ellipsis natural-width exceedances reported separately, not silently omitted. Four unrelated missing-asset reports (three BLP payloads) are logged; this does not prove all screen artwork is available.
- Exact regression RED: 0/2 passed at `e5b3e8744`, failing on 74>80−40 and 65>64. GREEN: 18/18 targeted tests (2 fit, 14 auction, 2 profession-art); concrete-width follow-up 2/2. Native extension helper build exits 0; changed Rust formatter and whitespace checks pass.
- Native cage capture exits 0: Professions and enabled auction search paging (Results 2/3), Modern and Forever. Four PNGs inspected; framebuffer/window/PNG dimensions each 1920×1080. Create All 74≤114−40; Results summary 64≤140; page arrows 32×32.

Source evidence: `data/diagnostics/buttonfit-2026-10-09/`. Byte-verified copies of four PNGs and RED/GREEN/capture logs: `/syncthing/AgentShared/2026-10-09/buttonfit/`. Build/test logs: `/home/osso/.worktrees/logs/buttonfit-*.log` (terminal EXIT markers).

No push, merge, server, or UDP 5000 operation. All owned jobs completed; `agents-buttonfit.slice` stopped. Initial untracked `scripts/__pycache__/` retained.
