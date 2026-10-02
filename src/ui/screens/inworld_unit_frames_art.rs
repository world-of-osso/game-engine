//! Retail unit frame art: `UiTextureAtlasMember` crops of their `UiTextureAtlas` texture.
//! Each constant cites `CommittedName` (member id) and the atlas texture FileDataID.

use shared::components::PowerType;

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

/// UiTextureAtlas 2060 `interface/hud/uiunitframe.blp`.
const UNIT_FRAME: (u32, (f32, f32)) = (4_631_591, (1024.0, 512.0));

const fn unit_frame(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: UNIT_FRAME.0,
        atlas: UNIT_FRAME.1,
        rect,
    }
}

pub const fn art(fdid: u32, atlas: (f32, f32), rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt { fdid, atlas, rect }
}

/// `UI-HUD-UnitFrame-Player-PortraitOff` (16107): name tab over a health and power slot.
pub const FRAME_PORTRAIT_OFF: AtlasArt = unit_frame((195.0, 328.0, 160.0, 211.0));
/// `UI-HUD-UnitFrame-Player-PortraitOn-Bar-Health` (16108), 124×20 like the slot.
pub const HEALTH_BAR: AtlasArt = unit_frame((705.0, 829.0, 213.0, 233.0));
/// `UI-HUD-UnitFrame-Target-PortraitOn-Type` (16713): reaction strip behind the target name.
pub const REACTION_STRIP: AtlasArt = unit_frame((195.0, 330.0, 235.0, 253.0));
/// `UI-HUD-UnitFrame-Player-CombatIcon` (16779).
pub const COMBAT_ICON: AtlasArt = unit_frame((1007.0, 1023.0, 133.0, 149.0));
/// First 60×60 cell of `UI-HUD-UnitFrame-Player-Rest-Flipbook` (16568, 6×7 cells),
/// atlas 2075 `interface/hud/uiunitframerestingflipbook.blp`.
pub const REST_ICON: AtlasArt = art(4_659_635, (512.0, 512.0), (1.0, 61.0, 1.0, 61.0));

/// UiTextureAtlas 2130 `interface/hud/uiunitframeboss.blp`.
const UNIT_FRAME_BOSS: (u32, (f32, f32)) = (4_703_659, (256.0, 256.0));

const fn unit_frame_boss(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(UNIT_FRAME_BOSS.0, UNIT_FRAME_BOSS.1, rect)
}

/// `UI-HUD-UnitFrame-Target-PortraitOn-Boss-Gold` (17120): the elite dragon, 80×79.
pub const BOSS_GOLD: AtlasArt = unit_frame_boss((1.0, 81.0, 84.0, 163.0));
/// `ui-hud-unitframe-target-portraiton-boss-rare-silver` (19019): the rare elite dragon.
pub const BOSS_RARE_SILVER: AtlasArt = unit_frame_boss((1.0, 81.0, 165.0, 244.0));
/// `UI-HUD-UnitFrame-Target-PortraitOn-Boss-Rare-Star` (17122), drawn at its 20×20
/// override size.
pub const BOSS_RARE_STAR: AtlasArt = unit_frame_boss((83.0, 109.0, 148.0, 174.0));
pub const BOSS_RARE_STAR_SIZE: f32 = 20.0;

/// Retail power bar texture: `UI-HUD-UnitFrame-Player-PortraitOff-Bar-<Power>` where the
/// atlas has a 124×10 portrait-off variant, the portrait-on one for mana, and the
/// `PowerBarColor[...].atlas` fill for spec powers (`PowerBarColorUtil.lua`).
pub fn power_bar_art(power: PowerType) -> Option<AtlasArt> {
    Some(match power {
        // UI-HUD-UnitFrame-Player-PortraitOn-Bar-Mana (16109)
        PowerType::Mana => unit_frame((647.0, 771.0, 301.0, 311.0)),
        // UI-HUD-UnitFrame-Player-PortraitOff-Bar-Rage (16868)
        PowerType::Rage => unit_frame((779.0, 903.0, 289.0, 299.0)),
        // UI-HUD-UnitFrame-Player-PortraitOff-Bar-Focus (16867)
        PowerType::Focus => unit_frame((653.0, 777.0, 289.0, 299.0)),
        // UI-HUD-UnitFrame-Player-PortraitOff-Bar-Energy (16866)
        PowerType::Energy => unit_frame((527.0, 651.0, 289.0, 299.0)),
        // UI-HUD-UnitFrame-Player-PortraitOff-Bar-RunicPower (16869)
        PowerType::RunicPower => unit_frame((269.0, 393.0, 301.0, 311.0)),
        // Unit_Druid_AstralPower_Fill_1x (24917), atlas 2649
        PowerType::LunarPower => art(5_410_916, (128.0, 32.0), (1.0, 127.0, 1.0, 11.0)),
        // Unit_Shaman_Maelstrom_Fill_1x (24922), atlas 2651
        PowerType::Maelstrom => art(5_410_922, (128.0, 32.0), (1.0, 127.0, 1.0, 11.0)),
        // Unit_Priest_Insanity_Fill_1x (25012), atlas 2661
        PowerType::Insanity => art(5_412_495, (1024.0, 1024.0), (1.0, 127.0, 1.0, 11.0)),
        // Unit_DemonHunter_Fury_Fill_1x (24914), atlas 2647
        PowerType::Fury => art(5_410_910, (128.0, 32.0), (1.0, 127.0, 1.0, 11.0)),
        // _DemonHunter-DemonicPainBar (5458), atlas 690
        PowerType::Pain => art(1_237_599, (128.0, 32.0), (0.0, 128.0, 13.0, 23.0)),
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
