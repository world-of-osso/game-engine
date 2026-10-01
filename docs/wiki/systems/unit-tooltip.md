# Unit Tooltip

How the client builds the Retail unit `GameTooltip` and the NPC drops/vendor section with appearance-collection marks. The requirements are in [unit-tooltip spec](../../specs/unit-tooltip.md); this page is how it works.

## Content

### Data flow

- **Hover.** `rendering/ui/unit_hover.rs` recomputes `HoveredUnit` every in-world frame: the unit of the unit frame cluster under the cursor (`FrameUnitSources::unit_at_frame`), else with no UI frame under the cursor `NameplatePicker::pick`, else a `MeshRayCast` through the cursor. The ray stops at its first hit (default `MeshRayCastSettings`), so an occluding crate hides the NPC behind it.
- **Server data.** `game/networking/unit_tooltip.rs` keeps `CreatureTooltipCache` (per creature entry, asked once with `CreatureTooltipQuery`) and `AccountAppearances` (replaced by every `AppearanceCollectionUpdate`). Both reset on leaving the world. The server builds `CreatureTooltip` from `WorldData` (subname, type), `LootTables::drop_chances` and `VendorCatalog`, and learns appearances from equipped and soulbound items (game-server `creature_tooltip.rs`, `appearance_collection.rs`).
- **Content.** `scenes/tooltip_frame/unit_sources.rs` reads the hovered unit's replicated `Npc`/`Player`, `UnitLevel`, `UnitFactionTemplate`, `GuildMembership`; reaction through `FactionTemplates` + `shared::faction_reaction`; the reputation faction name from `Faction.csv` (`FactionNames`, only `ReputationIndex >= 0`). `unit_tooltip.rs` turns that into `TooltipFrameState` lines; `section_lines` holds the ordering and truncation rule.
- **Frame.** `tooltip_frame/mod.rs` asks UI frame providers first and falls back to the unit tooltip; each provider sets `TooltipFrameState::anchor` to its Retail `OnEnter` anchor (default anchor or `SetOwner` beside the hovered frame) and `place_tooltip` resolves it against the owner's layout rect, clamped to the screen. Authored screen and line/art presentation live in `src/ui/screens/tooltip_presentation.rs` (`tooltip_frame_screen`); Bevy projects its owner/record state into `TooltipPresentation`. Item lines carry `item_mark`: the text is indented 14 px and a 12×12 `UI-LFG-ReadyMark` / `UI-LFG-DeclineMark` crop of `uilfgprompts.blp` (5171843) is drawn for collected / uncollected.

### Gotchas

- `AppearanceCollection` ids are `ItemAppearance` ids, not `ItemDisplayInfo` ids; items that share an `ItemAppearance` share collection state (Gladius and Worn Shortsword both 154; the bind-on-pickup Scout's Arrow teaches Brother Danil's Rough Arrow, 869).
- Admin `grant-equipment` writes equipment without recording an inventory change, so its appearance is learned at the next world entry; `grant-item` of a bind-on-pickup item is learned at once.
- Headless proof: under cage there is no pointer device, so `Window::set_cursor_position` (IPC `hover`) keeps its value until a real pointer event. JS `ui.key("B")` / `ui.click` with `--screen inworld` did not act in the 2026-09-26 run (not investigated).

## Sources

- [unit-tooltip spec](../../specs/unit-tooltip.md) — requirements and deviations
- `src/ui/screens/tooltip_presentation.rs`, `src/scenes/tooltip_frame/mod.rs` — shared authored screen and Bevy owner/anchor projection
- game-server `docs/specs/loot.md`, `docs/specs/appearance-collection.md` — drop chances, junk to gold, collection rules

## See Also

- [[loot-and-flight]] — corpses and loot window; greys now become coins
- [[merchant-frame]] — vendor lists come from the same `VendorCatalog`
- [[ui-system]] — tooltip frame and rsx screens
