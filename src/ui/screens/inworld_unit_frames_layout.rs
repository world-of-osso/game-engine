use crate::ui::anchor::FrameName;

pub(super) struct ShellConfig {
    pub width: f32,
    pub height: f32,
    pub texture: &'static str,
    pub anchor_x: &'static str,
    pub anchor_y: &'static str,
}

pub(super) struct PortraitConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub(super) struct TextConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

pub(super) struct BarConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub text_x: f32,
}

pub(super) struct MarkerConfig {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub(super) struct FrameConfig {
    pub frame_x: f32,
    pub shell: ShellConfig,
    pub portrait: PortraitConfig,
    pub name: TextConfig,
    pub level: TextConfig,
    pub health_bar: BarConfig,
    pub mana_bar: BarConfig,
}

pub(super) const FRAME_W: f32 = 232.0;
pub(super) const FRAME_H: f32 = 100.0;
pub(super) const FRAME_BOTTOM_Y: f32 = 130.0;
pub(super) const BAR_H: f32 = 20.0;
pub(super) const MANA_H: f32 = 10.0;
pub(super) const PORTRAIT_BG: &str = "0.02,0.02,0.02,0.92";
pub(super) const PLAYER_HEALTH_BG: &str = "0.07,0.02,0.02,0.90";
pub(super) const PLAYER_HEALTH_FILL: &str = "0.11,0.65,0.20,0.95";
pub(super) const TARGET_HEALTH_BG: &str = "0.08,0.02,0.02,0.90";
pub(super) const TARGET_HEALTH_FILL: &str = "0.80,0.12,0.12,0.95";
pub(super) const MANA_BG: &str = "0.03,0.05,0.12,0.90";
pub(super) const MANA_FILL: &str = "0.14,0.43,0.88,0.95";
pub(super) const BAR_EDGE: &str = "1.0,0.93,0.75,0.18";
pub(super) const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
pub(super) const NAME_TEXT: &str = "0.98,0.95,0.90,1.0";
pub(super) const VALUE_TEXT: &str = "1.0,1.0,1.0,0.95";
pub(super) const UNIT_NAME_FONT: &str = "FrizQuadrata";
pub(super) const UNIT_NAME_FONT_SIZE: f32 = 10.0;
pub(super) const UNIT_LEVEL_FONT_SIZE: f32 = 10.0;
pub(super) const STATUS_BAR_FONT: &str = "FrizQuadrata";
pub(super) const STATUS_BAR_FONT_SIZE: f32 = 10.0;
pub(super) const READY_CHECK_W: f32 = 40.0;
pub(super) const READY_CHECK_H: f32 = 40.0;

pub(super) const PLAYER_FRAME_CONFIG: FrameConfig = FrameConfig {
    frame_x: 268.0,
    shell: ShellConfig {
        width: 396.0,
        height: 142.0,
        texture: "data/ui/unitframes/player-frame-shell.ktx2",
        anchor_x: "0",
        anchor_y: "-2",
    },
    portrait: PortraitConfig {
        x: 24.0,
        y: 19.0,
        width: 60.0,
        height: 60.0,
    },
    name: TextConfig {
        x: 88.0,
        y: 27.0,
        width: 96.0,
    },
    level: TextConfig {
        x: -24.5,
        y: 28.0,
        width: 24.0,
    },
    health_bar: BarConfig {
        x: 85.0,
        y: 40.0,
        width: 124.0,
        text_x: 0.0,
    },
    mana_bar: BarConfig {
        x: 85.0,
        y: 61.0,
        width: 124.0,
        text_x: 0.0,
    },
};

pub(super) const TARGET_FRAME_CONFIG: FrameConfig = FrameConfig {
    frame_x: 1100.0,
    shell: ShellConfig {
        width: 384.0,
        height: 134.0,
        texture: "data/ui/unitframes/target-frame-shell.ktx2",
        anchor_x: "-2",
        anchor_y: "0",
    },
    portrait: PortraitConfig {
        x: 148.0,
        y: 19.0,
        width: 58.0,
        height: 58.0,
    },
    name: TextConfig {
        x: 51.0,
        y: 26.0,
        width: 90.0,
    },
    level: TextConfig {
        x: 24.0,
        y: 27.0,
        width: 24.0,
    },
    health_bar: BarConfig {
        x: 22.0,
        y: 28.0,
        width: 126.0,
        text_x: 0.0,
    },
    mana_bar: BarConfig {
        x: 22.0,
        y: 39.0,
        width: 134.0,
        text_x: -4.0,
    },
};

pub(super) const PLAYER_LEADER: MarkerConfig = MarkerConfig {
    x: 86.0,
    y: 10.0,
    width: 0.0,
    height: 0.0,
};
pub(super) const PLAYER_ROLE: MarkerConfig = MarkerConfig {
    x: 196.0,
    y: 27.0,
    width: 12.0,
    height: 12.0,
};
pub(super) const PLAYER_ATTACK: MarkerConfig = MarkerConfig {
    x: 64.0,
    y: 62.0,
    width: 0.0,
    height: 0.0,
};
pub(super) const PLAYER_CORNER: MarkerConfig = MarkerConfig {
    x: 58.5,
    y: 53.5,
    width: 0.0,
    height: 0.0,
};
pub(super) const PLAYER_PVP: MarkerConfig = MarkerConfig {
    x: 25.0,
    y: 50.0,
    width: 0.0,
    height: 0.0,
};
pub(super) const PLAYER_PRESTIGE: MarkerConfig = MarkerConfig {
    x: -2.0,
    y: 38.0,
    width: 50.0,
    height: 52.0,
};
pub(super) const PLAYER_PRESTIGE_BADGE_W: f32 = 30.0;
pub(super) const PLAYER_PRESTIGE_BADGE_H: f32 = 30.0;
pub(super) const PLAYER_PRESTIGE_PORTRAIT_FRAME: FrameName = FrameName("PlayerPrestigePortrait");
pub(super) const PLAYER_PORTRAIT_FRAME: FrameName = FrameName("PlayerPortrait");

pub(super) const TARGET_REPUTATION: MarkerConfig = MarkerConfig {
    x: 157.0,
    y: 25.0,
    width: 0.0,
    height: 0.0,
};
pub(super) const TARGET_HIGH_LEVEL: MarkerConfig = MarkerConfig {
    x: 28.0,
    y: 25.0,
    width: 0.0,
    height: 0.0,
};
pub(super) const TARGET_LEADER: MarkerConfig = MarkerConfig {
    x: 147.0,
    y: 8.0,
    width: 0.0,
    height: 0.0,
};
pub(super) const TARGET_RAID_ICON: MarkerConfig = MarkerConfig {
    x: 0.0,
    y: 0.0,
    width: 26.0,
    height: 26.0,
};
pub(super) const TARGET_PRESTIGE: MarkerConfig = MarkerConfig {
    x: 180.0,
    y: 38.0,
    width: 50.0,
    height: 52.0,
};
pub(super) const TARGET_PET_BATTLE: MarkerConfig = MarkerConfig {
    x: 187.0,
    y: 52.0,
    width: 32.0,
    height: 32.0,
};
pub(super) const TARGET_PRESTIGE_BADGE_W: f32 = 30.0;
pub(super) const TARGET_PRESTIGE_BADGE_H: f32 = 30.0;
pub(super) const TARGET_THREAT: MarkerConfig = MarkerConfig {
    x: 147.0,
    y: 5.0,
    width: 49.0,
    height: 18.0,
};
pub(super) const TARGET_PRESTIGE_PORTRAIT_FRAME: FrameName = FrameName("TargetPrestigePortrait");
pub(super) const TARGET_PORTRAIT_FRAME: FrameName = FrameName("TargetPortrait");

pub const PLAYER_HEALTH_BAR_W: f32 = PLAYER_FRAME_CONFIG.health_bar.width;
pub const TARGET_HEALTH_BAR_W: f32 = TARGET_FRAME_CONFIG.health_bar.width;
pub const TARGET_MANA_BAR_W: f32 = TARGET_FRAME_CONFIG.mana_bar.width;
