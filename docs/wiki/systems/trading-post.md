# Trading Post

Native Retail Mainline `Blizzard_PerksProgram` vendor and Traveler's Log Tender. [Spec](../../specs/trading-post.md) owns requirements and gaps; [Adventure Guide](adventure-guide.md#realm-activities-and-suggestions) owns monthly activity criteria.

## Flow

1. Authentic capital NPCs (Stormwind Tawny/Wilder Seabraid 185468/185467, Dornogal 219243/219244) carry the sourced NPCFlags2 Trading Post bit; their TDB gossip option `OptionNpc=47` opens `NpcRole::TradingPost`. `merchant.rs` routes that interaction to `open_trading_post`, which sends `TradingPostRequest::Query`.
2. The server snapshot carries Tender, points, thresholds, the frozen item and all 1321 sourced vendor rows; rows with unreadable metadata or unbound grants have `grant_unavailable`.
3. `godot/ui-model/src/trading_post.rs` renders the list, `PerksProgramFilter` dropdown, details, ToyOverlay (category 5), frozen slot and confirmation. `godot/rust/src/trading_post.rs` owns input, the detached preview and requests. Purchase, freeze and refund are server decisions; the client never debits locally.
4. While open, every other client `CanvasLayer` except `GameTooltipUI` is hidden: Retail registers `PerksProgramFrame` with `UIPanelWindows` area `full`, and `UIParentPanelManager` hides UIParent for fullscreen panels. The world stays visible behind the gradients. Close restores the hidden layers.

## Previews

- Mounts/pets: `UnitAppearance::Creature` from the row's authored display, else the granted mount's unconditional `MountXDisplay` or the pet species display.
- Transmog/sets: `UnitAppearance::Player` with the local appearance plus the row's preview items (sets start from an empty outfit).
- Unavailable rows show their gap text and no model. A failed preview clears the previous model, so a stale selection cannot pass for the current one (2026-10-12 live RED: an unreadable top Transmog row left the previous gryphon on screen).
- Assets: [product-isolated receipts](../../specs/product-isolated-model-assets.md). Before staging, only 167/1309 previewable rows had complete chains. `scripts/stage_trading_post_model_assets.py` (seeded from the same server SQL) authenticated 43,275 artifacts from local CASC for 1287/1309. Package: `/home/osso/.worktrees/trading-post-evidence-2026-10-12/tp-assets-package/`. Validate with `apply_mount_model_assets.py --package`; the lead publishes with `--apply`.

## Live proof — 2026-10-12

Engine `b39e7c703`, server `4cc6b821`, protocol `1f245e3`; private 127.0.0.1:5731 with an owned world copy, fresh redb and `fb_tradingpost`. `godot/tests/trading_post_live.gd` uses native input only. Modern browse, Forever browse and Forever relog PASS: Log Tender line (0/500, balance 0), Tawny gossip → vendor, HUD 17 layers hidden and restored on Leave, dropdown, player transmog/set and creature pet/mount previews, rotation pixel change, ToyOverlay, unavailable row, drag-to-freeze (vendor item 456) surviving relog, and unaffordable purchase without confirmation. The 26 inspected PNGs are in `/syncthing/AgentShared/2026-10-12/trading-post/`.

Not proven live: purchase/collection/refund. None of the 22 server-supported activities has reachable private-world content, so a fresh account has no Tender. Server behavioural tests cover those paths.

## Canonical-write boundary

Runtime cache misses write to the source data path (`<checkout>/data/textures`), not `ASSET_RESOLVER_SHARED_DATA_DIR`. With the normal [shared slot links](../../headless-live-run.md) they reach canonical data. This task forbade canonical writes, so its slot replaced the asset directories with per-entry symlink unions that also overlay the unpublished package.
