# WoW UI Sim Layout Spec

Saved on 2026-03-31 as the concrete layout spec for matching `wow-ui-sim` tabs plus the `PlayerFrame` and `TargetFrame` geometry in `game-engine`.

## Sources

- `wow-ui-sim/tests/frame_positions.rs`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.lua`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_UIPanels_Game/Mainline/CharacterFrame.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_UnitFrame/Mainline/PlayerFrame.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_UnitFrame/Mainline/TargetFrame.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_Fonts_Shared/Shared/FontStyles.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_Fonts_Shared/Shared/GameFontStyles.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_Fonts_Shared/Shared/Fonts.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_Fonts_Shared/Shared/GameFonts.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_Fonts_Shared/Mainline/GameFontStyles.xml`
- `wow-ui-sim/Interface/BlizzardUI/Blizzard_Fonts_Shared/Mainline/GameFonts.xml`

## Panel Tabs

These are the WoW bottom panel tabs used by `CharacterFrame`.

- Base tab size: `10 x 32`
- First tab anchor: `TOPLEFT` to panel `BOTTOMLEFT` at `x=11 y=2`
- Subsequent tab anchor: `TOPLEFT` to previous tab `TOPRIGHT` at `x=1 y=0`
- Generic anchor helper spacing: `x=3 y=0`
- Width formula: `textWidth + 20 + padding`
- Side padding constant: `20`
- Inactive left cap offset: `-3`
- Inactive right cap offset: `+7`
- Active left cap offset: `-1`
- Active right cap offset: `+8`
- Default text anchor: `CENTER y=2`
- Selected text offset: `CENTER y=-3`

## Fonts

Roman alphabet values from Blizzard font families:

- `GameFontNormalSmall`
  - inherits `SystemFont_Shadow_Small`
  - font: `FRIZQT__.TTF`
  - size: `10px`
  - shadow: `(1, -1)`
- `GameNormalNumberFont`
  - inherits `NumberFont_GameNormal`
  - font: `FRIZQT__.TTF`
  - size: `10px`
  - shadow: `(1, -1)`
- `TextStatusBarText`
  - inherits `SystemFont_Outline_Small`
  - font: `FRIZQT__.TTF`
  - size: `10px`
  - outline: `NORMAL`

## Player Frame

Outer rect:

- `x=268 y=850 w=232 h=100`
- hit rect insets: `left=6 right=0 top=4 bottom=9`

Internal geometry:

- portrait: `60 x 60`, `TOPLEFT x=24 y=-19`
- portrait mask: `60 x 60`, same anchor
- name: `96 x 12`, `TOPLEFT x=88 y=-27`, font `GameFontNormalSmall`
- level: `TOPRIGHT x=-24.5 y=-28`, font `GameNormalNumberFont`
- health container: `124 x 20`, `TOPLEFT x=85 y=-40`
- health bar: `124 x 20`, `TOPLEFT x=0 y=0`
- health text center: `CENTER x=0 y=0`
- health text left: `LEFT x=2 y=0`
- health text right: `RIGHT x=-2 y=0`
- health mask: `TOPLEFT x=-2 y=6`
- mana bar: `124 x 10`, `TOPLEFT x=85 y=-61`
- mana text center: `CENTER x=0 y=0`
- mana text left: `LEFT x=2 y=0`
- mana text right: `RIGHT x=-2 y=0`

Contextual icons:

- leader icon: `TOPLEFT x=86 y=-10`
- guide icon: `TOPLEFT x=86 y=-10`
- role icon: `12 x 12`, `TOPLEFT x=196 y=-27`
- attack icon: `TOPLEFT x=64 y=-62`
- portrait corner icon: `TOPLEFT x=58.5 y=-53.5`
- PvP icon: `TOP` to frame `TOPLEFT x=25 y=-50`
- prestige portrait: `50 x 52`, `TOPLEFT x=-2 y=-38`
- prestige badge: `30 x 30`, centered on prestige portrait
- ready check: `40 x 40`, centered on portrait

## Target Frame

Outer rect:

- `x=1100 y=850 w=232 h=100`
- hit rect insets: `left=0 right=5 top=4 bottom=9`

Internal geometry:

- portrait: `58 x 58`, `TOPRIGHT x=-26 y=-19`
- reputation color anchor: `TOPRIGHT x=-75 y=-25`
- name: `90 x 12`, relative to `ReputationColor.TOPRIGHT` at `x=-106 y=-1`, font `GameFontNormalSmall`
- level: relative to `ReputationColor.TOPRIGHT` at `x=-133 y=-2`, font `GameNormalNumberFont`
- health container: `126 x 20`, `BOTTOMRIGHT` to frame `LEFT` at `x=148 y=2`
- health bar: `126 x 20`, `TOPLEFT x=0 y=0`
- health text center: `CENTER x=0 y=0`
- health text left: `LEFT x=2 y=0`
- health text right: `RIGHT x=-5 y=0`
- dead text: `CENTER x=0 y=0`
- unconscious text: `CENTER x=0 y=0`
- health mask: `TOPLEFT x=-1 y=6`
- mana bar: `134 x 10`, `TOPRIGHT` to `HealthBarsContainer.BOTTOMRIGHT` at `x=8 y=-1`
- mana text center: `CENTER x=-4 y=0`
- mana text left: `LEFT x=2 y=0`
- mana text right: `RIGHT x=-13 y=0`
- mana mask: `TOPLEFT x=-61 y=3`

Contextual icons:

- high-level texture: relative to `LevelText.TOPLEFT` at `x=4 y=2`
- leader icon: `TOPRIGHT x=-85 y=-8`
- guide icon: `TOPRIGHT x=-85 y=-8`
- raid target icon: `26 x 26`, centered on portrait top
- boss icon: centered on portrait bottom
- quest icon: centered on portrait bottom
- PvP icon: `TOP` to frame `TOPRIGHT x=-26 y=-50`
- prestige portrait: `50 x 52`, `TOPRIGHT x=-2 y=-38`
- pet battle icon: `32 x 32`, `TOPRIGHT x=-13 y=-52`
- prestige badge: `30 x 30`, centered on prestige portrait
- numerical threat frame: `49 x 18`, `BOTTOM` to `ReputationColor.TOP x=0 y=0`
- numerical threat text: `TOP x=0 y=-4`
- numerical threat background: `37 x 14`, `TOP x=0 y=-3`

## Implementation Targets

- Keybinding section tabs should use measured `FRIZQT__.TTF` text widths at `10px` plus the WoW `20px` side padding.
- `PlayerFrame` and `TargetFrame` should be positioned by the exact screen rects above, not by relative offsets from each other.
- Player and target text should use `FrizQuadrata` `10px` to match the Blizzard font objects the XML references.
- Health and mana bars should use the exact sizes and offsets above even if the current shell textures remain temporary.
