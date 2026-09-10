# In-World Unit Frames Reference

Saved on 2026-03-30 as the local reference for the first RSX player/target-frame pass in `game-engine`.

## Source

- wow-ui-sim player frame: `/syncthing/Sync/Projects/wow/wow-ui-sim/Interface/BlizzardUI/Blizzard_UnitFrame/PlayerFrame.xml`
- wow-ui-sim target frame: `/syncthing/Sync/Projects/wow/wow-ui-sim/Interface/BlizzardUI/Blizzard_UnitFrame/TargetFrame.xml`

## Player Frame Tree

The mainline wow-ui-sim tree relevant to the first RSX port is:

```text
PlayerFrame
  PlayerFrameContainer
    PlayerPortrait
    PlayerPortraitMask
    VehicleFrameTexture
    FrameTexture
    AlternatePowerFrameTexture
    FrameFlash
  PlayerFrameContent
    PlayerFrameContentMain
      PlayerName
      PlayerLevelText
      StatusTexture
      HealthBarsContainer
        HealthBar
        HealthBarText
        LeftText
        RightText
        HealthBarMask
      ManaBarArea
        ManaBar
        ManaBarText
        LeftText
        RightText
        ManaBarMask
    PlayerFrameContentContextual
      AttackIcon
      PlayerPortraitCornerIcon
      PVPIcon
      PrestigePortrait
```

### Historical XML geometry

The source XML uses a logical `232 x 100` frame: portrait `(24, -19)` at `60 x 60`, name `(88, -27)`, level `(-24.5, -28)`, and health `(85, -40)` at `124 x 20`.

### Current player-shell geometry

`player-frame-shell.png` is unmodified custom `396 x 142` artwork, not a `232 x 100` XML frame. Its colored paint encloses connected transparent openings. Player content uses those exact native bounds, scaled 75% into a `297 x 106.5` HUD frame:

- portrait `(18, 13, 111 x 113)`; keyhole-shaped gold opening, including its square lower-right
- health `(135, 52, 249 x 40)`
- mana `(135, 94, 249 x 20)`

The renderer derives white-alpha masks from the shell, cover-resizes the cached class icon into the portrait mask, and renders player resource background/fill through the bar masks. Player art overlays fills to retain painted shadows and edges. `tex_coords` crops the fill mask from the left as health changes; it does not squeeze a full mask into a shorter quad. Target portrait and target-bar geometry remain separate.

## Target Frame Tree

The matching target tree used for the RSX port is:

```text
TargetFrame
  TargetFrameContainer
    Portrait
    PortraitMask
    FrameTexture
    Flash
    BossPortraitFrameTexture
  TargetFrameContent
    TargetFrameContentMain
      ReputationColor
      Name
      LevelText
      HealthBarsContainer
        HealthBar
        HealthBarText
        LeftText
        RightText
        DeadText
        UnconsciousText
        HealthBarMask
      ManaBar
        ManaBarText
        LeftText
        RightText
        ManaBarMask
    TargetFrameContentContextual
      HighLevelTexture
      LeaderIcon
      GuideIcon
      RaidTargetIcon
      BossIcon
      QuestIcon
```

Key geometry from the XML:

- `TargetFrame` size: `232 x 100`
- portrait anchor: top-right at `(-26, -19)` with size `58 x 58`
- name anchor: offset from `ReputationColor`
- level anchor: offset from `ReputationColor`
- health container anchor: bottom-right of the left-side frame body at `(148, 2)` with size `126 x 20`
- mana bar size: `134 x 10`, anchored under the health bar

## Current RSX Mapping

The first `game-engine` RSX pass intentionally mirrors the structural names above:

- `PlayerFrame`
- `PlayerFrameContainer`
- `PlayerPortrait`
- `PlayerName`
- `PlayerLevelText`
- `PlayerHealthBar`
- `PlayerManaBar`
- `TargetFrame`
- `TargetFrameContainer`
- `TargetPortrait`
- `TargetName`
- `TargetLevelText`
- `TargetHealthBar`
- `TargetManaBar`

This pass is using replicated ECS data, not mock UI data:

- player: `LocalPlayer` plus `shared::components::{Player, Health, Mana}`
- target: `CurrentTarget(Entity)` plus the target entity's replicated components

If the visuals drift later, measure the shell openings before changing the RSX tree; do not reuse the historical XML coordinates for the custom player artwork.
