# Native gathering

Retail Mainline defines Mining, Herbalism and Skinning; Forever only re-skins the same interactions. The [server contract](../../../game-server/docs/specs/gathering.md) owns authored metadata, skill checks, loot and lifecycle.

- Render replicated gathering nodes with their GameObjectDisplayInfo CASC model, not placeholder geometry.
- Right-click / soft interaction sends existing UseGameObject for GatherableResource nodes and fully looted skinnable corpses; normal cast feedback, loot window and profession snapshots show the outcome. IPC quest-interact follows the same resource interaction.
- Minimap tracking exposes Find Herbs only for learned spell2383 and Find Minerals only for learned spell2580. Each toggle filters the replicated authored parent skill182/186 and clips to the map mask. The open tracking menu draws above other HUD canvases so both filter rows remain visible and clickable. Unlearned, unchecked, out-of-range and Skinning resources do not produce herb/ore blips.
- Blips use Retail atlas647 professions_tracking_herb/member17635 and professions_tracking_ore/member17636, from local UiTextureAtlasMember.csv. Both skins use the same filters and source art.
- Acceptance requires inspected Modern + Forever PNGs of tracking, herb/ore gathering, skinning a kill and skill progression, plus behavioral proof. Source changes and model extraction alone do not satisfy that gate.
