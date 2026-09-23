# Combat HUD initial design

**Status:** Design scope only. No implementation is authorized by this document.

## Accepted decisions

- Combat HUD is the next design scope.
- Place a central cluster above the bottom-center action bars: compact player and target frames flank a shared resource/cast area.
- Use a restrained fantasy style: thin metal borders, dark backing, no portraits, and familiar spell artwork.

## Pending proposals

The following guide later design discussion; none are accepted requirements.

- **Aura placement:** keep target buffs/debuffs close to the target frame or place them in a separate cluster row.
- **Action bars:** determine the number of visible bars, their rows, and their relationship to the central cluster.
- **Sizing:** define final frame, resource, cast-bar, icon, spacing, and screen-safe-area measurements.
- **Interaction and data wiring:** define input behavior, edit/move mode, combat-state visibility, authoritative data sources, and synchronization boundaries.

## Scope boundary

This cycle designs the combat HUD only. It does not redesign game windows, minimap, or chat, and it does not implement gameplay abilities. These are design-stage boundaries, not permanent exclusions.

## Existing source paths

- `src/ui/screens/inworld_unit_frames_component.rs` — current player/target frame presentation, including target aura rows.
- `src/ui/screens/inworld_hud_component.rs` — current bottom action bars and wider InWorld HUD composition.
- `src/ui/screens/casting_bar_frame_component.rs` — current bottom-center casting-bar presentation and state fields.
