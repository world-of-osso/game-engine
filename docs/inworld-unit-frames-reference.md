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

Key geometry from the XML:

- `PlayerFrame` size: `232 x 100`
- portrait anchor: top-left at `(24, -19)` with size `60 x 60`
- player name anchor: top-left at `(88, -27)`
- level anchor: top-right at `(-24.5, -28)`
- health container anchor: top-left at `(85, -40)` with size `124 x 20`
- mana bar lives directly below the health bar region

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

If the visuals drift later, update this file before changing the RSX tree again.
