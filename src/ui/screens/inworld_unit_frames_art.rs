//! Retail unit frame art: `UiTextureAtlasMember` crops of their `UiTextureAtlas` texture.
//! Each constant cites `CommittedName` (member id) and the atlas texture FileDataID.

use shared::components::PowerType;

use crate::status::SecondaryResourceKindEntry;

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

const fn art(fdid: u32, atlas: (f32, f32), rect: (f32, f32, f32, f32)) -> AtlasArt {
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

/// A class resource pip: `background` always, `lit` on top while the point is available.
/// `unlit` replaces the background for empty points where Retail has a distinct atlas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PipArt {
    pub background: AtlasArt,
    pub unlit: Option<AtlasArt>,
    pub lit: AtlasArt,
    /// Pip cell size and gap from the Retail `ClassResourceBarTemplate` layout.
    pub cell: (f32, f32),
    pub spacing: f32,
}

/// Paladin holy power: runes inside one holder (`PaladinPowerBar.xml`).
pub struct HolyPowerArt;

impl HolyPowerArt {
    /// `uf-holypower-runeholder` (17384), atlas 2141 `uipaladinholypower.blp`.
    pub const HOLDER: AtlasArt = holy((1.0, 151.0, 440.0, 483.0));
    /// `uf-holypower-rune{n}-active` (17345..17373), centred on each rune's LEFT anchor
    /// `(x, y, width, height)` from `PaladinPowerBar.xml`.
    pub const RUNES: [(AtlasArt, (f32, f32, f32, f32)); 5] = [
        (holy((475.0, 498.0, 258.0, 281.0)), (18.0, -1.0, 18.0, 18.0)),
        (holy((323.0, 347.0, 348.0, 370.0)), (41.6, 0.0, 20.0, 16.0)),
        (holy((323.0, 346.0, 421.0, 442.0)), (68.0, 0.5, 17.0, 15.0)),
        (holy((323.0, 344.0, 467.0, 489.0)), (92.0, 0.0, 15.0, 16.0)),
        (
            holy((299.0, 321.0, 475.0, 498.0)),
            (112.2, -1.0, 18.0, 16.0),
        ),
    ];
}

const fn holy(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_719_247, (512.0, 512.0), rect)
}

/// Pip art for every class resource except holy power (see [`HolyPowerArt`]).
pub fn pip_art(kind: &SecondaryResourceKindEntry) -> Option<PipArt> {
    Some(match kind {
        SecondaryResourceKindEntry::HolyPower => return None,
        // uf-roguecp-bg (19991) / uf-roguecp-icon-red (19993), atlas 2302 uiroguecombpoints.blp
        SecondaryResourceKindEntry::ComboPoints => PipArt {
            background: rogue((299.0, 319.0, 1.0, 21.0)),
            unlit: None,
            lit: rogue((299.0, 313.0, 106.0, 121.0)),
            cell: (20.0, 20.0),
            spacing: 4.0,
        },
        // uf-chi-bg (21139) / uf-chi-icon (21146), atlas 2224 uimonkchi.blp
        SecondaryResourceKindEntry::Chi => PipArt {
            background: chi((225.0, 250.0, 43.0, 68.0)),
            unlit: None,
            lit: chi((1.0, 15.0, 174.0, 189.0)),
            cell: (21.0, 21.0),
            spacing: 2.0,
        },
        // uf-soulshard-holder (17230) / uf-soulshard-icon (17231), atlas 2138 uiwarlocksoulshard.blp
        SecondaryResourceKindEntry::SoulShards => PipArt {
            background: shard((90.0, 113.0, 162.0, 192.0)),
            unlit: None,
            lit: shard((206.0, 221.0, 191.0, 211.0)),
            cell: (23.0, 30.0),
            spacing: 1.0,
        },
        // uf-arcane-bg (21047) / uf-arcane-icon (21051), atlas 2378 uimagearcanecharge.blp
        SecondaryResourceKindEntry::ArcaneCharges => PipArt {
            background: arcane((56.0, 77.0, 228.0, 249.0)),
            unlit: None,
            lit: arcane((79.0, 98.0, 228.0, 247.0)),
            cell: (21.0, 21.0),
            spacing: 10.0,
        },
        // uf-dkrunes-bgactive (19112) / uf-dkrunes-bgdis (19113) +
        // uf-dkrunes-blood-skullactive (19121), atlas 2273 uideathknightrunes.blp
        SecondaryResourceKindEntry::Runes => PipArt {
            background: runes((1.0, 28.0, 147.0, 174.0)),
            unlit: Some(runes((1.0, 28.0, 176.0, 203.0))),
            lit: runes((30.0, 46.0, 234.0, 251.0)),
            cell: (24.0, 24.0),
            spacing: -1.0,
        },
        // uf-essence-bg (16259) / uf-essence-icon (16276), atlas 2034 uievokeressence.blp
        SecondaryResourceKindEntry::Essence => PipArt {
            background: essence((324.0, 348.0, 107.0, 131.0)),
            unlit: None,
            lit: essence((350.0, 374.0, 133.0, 157.0)),
            cell: (24.0, 24.0),
            spacing: -1.0,
        },
    })
}

const fn rogue(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_902_605, (512.0, 512.0), rect)
}

const fn chi(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_744_156, (256.0, 256.0), rect)
}

const fn shard(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_715_163, (512.0, 256.0), rect)
}

const fn arcane(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(5_045_210, (512.0, 256.0), rect)
}

const fn runes(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_876_501, (512.0, 256.0), rect)
}

const fn essence(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_631_562, (512.0, 256.0), rect)
}
