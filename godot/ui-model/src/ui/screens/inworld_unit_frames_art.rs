//! Retail unit frame art. Unit frames name `UiTextureAtlasElement`s, which the atlas
//! resolver draws from the active skin's set; [`AtlasArt`] crops serve the other frames.

use shared::components::PowerType;
use ui_toolkit::atlas::{ActiveSkin, resolve_region};

/// One atlas member: pixel rect inside an atlas texture of `atlas` size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasArt {
    pub fdid: u32,
    pub atlas: (f32, f32),
    /// Pixel rect `(left, right, top, bottom)` as committed in `UiTextureAtlasMember`.
    pub rect: (f32, f32, f32, f32),
}

impl AtlasArt {
    pub const fn size(&self) -> (f32, f32) {
        (self.rect.1 - self.rect.0, self.rect.3 - self.rect.2)
    }

    /// Normalised `left,right,top,bottom` of the leftmost `fraction` of the crop, the way a
    /// Retail `StatusBar` reveals its bar texture.
    pub fn tex_coords(&self, fraction: f32) -> String {
        let (left, right, top, bottom) = self.rect;
        let right = left + (right - left) * fraction.clamp(0.0, 1.0);
        let (width, height) = self.atlas;
        format!(
            "{},{},{},{}",
            left / width,
            right / width,
            top / height,
            bottom / height
        )
    }
}

pub const fn art(fdid: u32, atlas: (f32, f32), rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt { fdid, atlas, rect }
}

/// Size of atlas element `name` under `skin` (`SetAtlas(name, useAtlasSize)`).
///
/// # Panics
/// When `skin` has no member for `name`: the frame names art its data lacks.
pub fn atlas_size(name: &str, skin: ActiveSkin) -> (f32, f32) {
    let region = resolve_region(name, skin)
        .unwrap_or_else(|| panic!("atlas {name} has no UiTextureAtlasMember under {skin:?}"));
    (region.width, region.height)
}

/// `UI-HUD-UnitFrame-Player-PortraitOff`: name tab over a health and power slot.
pub const FRAME_PORTRAIT_OFF: &str = "UI-HUD-UnitFrame-Player-PortraitOff";
/// PlayerFrame's `FrameTexture`.
pub const PLAYER_PORTRAIT_ON: &str = "UI-HUD-UnitFrame-Player-PortraitOn";
/// TargetFrame's `FrameTexture`.
pub const TARGET_PORTRAIT_ON: &str = "UI-HUD-UnitFrame-Target-PortraitOn";
pub const HEALTH_BAR: &str = "UI-HUD-UnitFrame-Player-PortraitOn-Bar-Health";
pub const TARGET_HEALTH_BAR: &str = "UI-HUD-UnitFrame-Target-PortraitOn-Bar-Health";
/// TargetFrame's `ReputationColor`: reaction strip behind the target name.
pub const REACTION_STRIP: &str = "UI-HUD-UnitFrame-Target-PortraitOn-Type";
/// PlayerFrame's `AttackIcon`.
pub const COMBAT_ICON: &str = "UI-HUD-UnitFrame-Player-CombatIcon";
/// PlayerFrame's `RestTexture` flipbook (PlayerFrame.xml:387): 6 columns × 7 rows.
pub const REST_FLIPBOOK: &str = "UI-HUD-UnitFrame-Player-Rest-Flipbook";
/// The flipbook's first cell, `left,right,top,bottom` of its atlas crop.
pub const REST_ICON_COORDS: &str = "0,0.16666667,0,0.14285715";

/// PetFrame's `PetFrameTexture` (PetFrame.xml:42-47; the pet frame uses the
/// target-of-target art).
pub const TOT_PORTRAIT_ON: &str = "UI-HUD-UnitFrame-TargetofTarget-PortraitOn";
pub const TOT_HEALTH_BAR: &str = "UI-HUD-UnitFrame-TargetofTarget-PortraitOn-Bar-Health";

/// The `TargetofTarget` frame type's power bar: `UI-HUD-UnitFrame-TargetofTarget-
/// PortraitOn-Bar-<atlasElementName>` (`UnitFrameManaBar_UpdateType`, UnitFrame.lua:527).
/// Powers without such a member draw no fill.
pub fn tot_power_bar_atlas(power: PowerType) -> Option<&'static str> {
    Some(match power {
        PowerType::Mana => "UI-HUD-UnitFrame-TargetofTarget-PortraitOn-Bar-Mana",
        PowerType::Rage => "UI-HUD-UnitFrame-TargetofTarget-PortraitOn-Bar-Rage",
        PowerType::Focus => "UI-HUD-UnitFrame-TargetofTarget-PortraitOn-Bar-Focus",
        PowerType::Energy => "UI-HUD-UnitFrame-TargetofTarget-PortraitOn-Bar-Energy",
        PowerType::RunicPower => "UI-HUD-UnitFrame-TargetofTarget-PortraitOn-Bar-RunicPower",
        _ => return None,
    })
}

/// TargetFrame's `BossPortraitFrameTexture` for elites: the gold dragon.
pub const BOSS_GOLD: &str = "UI-HUD-UnitFrame-Target-PortraitOn-Boss-Gold";
/// The rare elite silver dragon.
pub const BOSS_RARE_SILVER: &str = "ui-hud-unitframe-target-portraiton-boss-rare-silver";
/// TargetFrame's `BossIcon`, the rare star, drawn at its override size.
pub const BOSS_RARE_STAR: &str = "UI-HUD-UnitFrame-Target-PortraitOn-Boss-Rare-Star";

/// Retail power bar texture: `UI-HUD-UnitFrame-Player-PortraitOff-Bar-<Power>` where the
/// atlas has a 124×10 portrait-off variant, the portrait-on one for mana, and the
/// `PowerBarColor[...].atlas` fill for spec powers (`PowerBarColorUtil.lua`).
pub fn power_bar_atlas(power: PowerType) -> Option<&'static str> {
    Some(match power {
        PowerType::Mana => "UI-HUD-UnitFrame-Player-PortraitOn-Bar-Mana",
        PowerType::Rage => "UI-HUD-UnitFrame-Player-PortraitOff-Bar-Rage",
        PowerType::Focus => "UI-HUD-UnitFrame-Player-PortraitOff-Bar-Focus",
        PowerType::Energy => "UI-HUD-UnitFrame-Player-PortraitOff-Bar-Energy",
        PowerType::RunicPower => "UI-HUD-UnitFrame-Player-PortraitOff-Bar-RunicPower",
        PowerType::LunarPower => "Unit_Druid_AstralPower_Fill",
        PowerType::Maelstrom => "Unit_Shaman_Maelstrom_Fill",
        PowerType::Insanity => "Unit_Priest_Insanity_Fill",
        PowerType::Fury => "Unit_DemonHunter_Fury_Fill",
        PowerType::Pain => "_DemonHunter-DemonicPainBar",
        _ => return None,
    })
}

/// One `<Alpha>` of a Retail animation group: `from` to `to` over `start..start + duration`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlphaKey {
    pub from: f32,
    pub to: f32,
    pub start: f32,
    pub duration: f32,
}

pub const fn key(from: f32, to: f32, start: f32, duration: f32) -> AlphaKey {
    AlphaKey {
        from,
        to,
        start,
        duration,
    }
}

/// Alpha `t` seconds into a group: the latest-starting started key (the later one on a
/// tie), held at `to` once done
/// (`setToFinalAlpha`), or 0 before the first key (the `ResetVisuals` alpha).
pub fn key_alpha(keys: &[AlphaKey], t: f32) -> f32 {
    let started = keys.iter().filter(|key| key.start <= t);
    let latest = started.reduce(|latest, key| {
        if key.start >= latest.start {
            key
        } else {
            latest
        }
    });
    latest.map_or(0.0, |key| {
        let progress = if key.duration > 0.0 {
            ((t - key.start) / key.duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        key.from + (key.to - key.from) * progress
    })
}

/// A `<FlipBook>` over the texture's atlas crop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlipBook {
    pub columns: u32,
    pub rows: u32,
    pub frames: u32,
    pub duration: f32,
}

impl FlipBook {
    /// The crop of the frame shown `t` seconds in; the last frame once done.
    pub fn frame_art(&self, art: &AtlasArt, t: f32) -> AtlasArt {
        let last = self.frames - 1;
        let frame = ((t / self.duration * self.frames as f32) as u32).min(last);
        let (left, right, top, bottom) = art.rect;
        let width = (right - left) / self.columns as f32;
        let height = (bottom - top) / self.rows as f32;
        let x = left + (frame % self.columns) as f32 * width;
        let y = top + (frame / self.columns) as f32 * height;
        AtlasArt {
            rect: (x, x + width, y, y + height),
            ..*art
        }
    }
}
