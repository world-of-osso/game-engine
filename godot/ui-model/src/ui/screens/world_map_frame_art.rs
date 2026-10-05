//! Retail world-map art: `UiTextureAtlasMember` crops (committed pixel rects on the
//! `UiTextureAtlas` sheet) and plain textures used by the world map frame.

/// A crop of one texture: `rect` is `(left, right, top, bottom)` in sheet pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapArt {
    pub fdid: u32,
    pub sheet: (f32, f32),
    pub rect: (f32, f32, f32, f32),
}

impl MapArt {
    /// Normalized `[left, right, top, bottom]`.
    pub fn tex_coords(&self) -> [f32; 4] {
        let (left, right, top, bottom) = self.rect;
        let (width, height) = self.sheet;
        [left / width, right / width, top / height, bottom / height]
    }

    /// Committed size in sheet pixels.
    pub fn size(&self) -> (f32, f32) {
        (self.rect.1 - self.rect.0, self.rect.3 - self.rect.2)
    }
}

const fn art(fdid: u32, sheet: (f32, f32), rect: (f32, f32, f32, f32)) -> MapArt {
    MapArt { fdid, sheet, rect }
}

const fn whole(fdid: u32, size: f32) -> MapArt {
    art(fdid, (size, size), (0.0, size, 0.0, size))
}

/// UiTextureAtlas 1390 `interface/framegeneral/uiframemetal2x.blp`.
const METAL: (u32, (f32, f32)) = (2_406_979, (512.0, 512.0));
/// UiTextureAtlas 1394, vertical edges.
const METAL_VERTICAL: (u32, (f32, f32)) = (2_406_984, (512.0, 32.0));
/// UiTextureAtlas 1395, horizontal edges.
const METAL_HORIZONTAL: (u32, (f32, f32)) = (2_406_987, (64.0, 256.0));
/// UiTextureAtlas 2116 red panel buttons.
const RED_BUTTONS: (u32, (f32, f32)) = (4_698_972, (256.0, 128.0));
/// UiTextureAtlas holding `TaxiNode_*` and `QuestNormal` (sheet 1121272).
const OBJECT_ICONS: (u32, (f32, f32)) = (1_121_272, (1024.0, 1024.0));
/// UiTextureAtlas holding `UI-QuestPoi-*` (sheet 5320914).
const QUEST_POI: (u32, (f32, f32)) = (5_320_914, (256.0, 128.0));

/// `ui-frame-portraitmetal-cornertopleft-2x` (8504).
pub const PORTRAIT_CORNER_TOP_LEFT: MapArt = art(METAL.0, METAL.1, (1.0, 151.0, 153.0, 303.0));
/// `ui-frame-metal-cornertopleft-2x` (8501), the portrait-less corner.
pub const CORNER_TOP_LEFT: MapArt = art(METAL.0, METAL.1, (1.0, 151.0, 1.0, 151.0));
/// `ui-frame-metal-cornertopright-2x` (8502).
pub const CORNER_TOP_RIGHT: MapArt = art(METAL.0, METAL.1, (153.0, 303.0, 1.0, 151.0));
/// `ui-frame-metal-cornerbottomleft-2x` (8499).
pub const CORNER_BOTTOM_LEFT: MapArt = art(METAL.0, METAL.1, (153.0, 217.0, 153.0, 217.0));
/// `ui-frame-metal-cornerbottomright-2x` (8500).
pub const CORNER_BOTTOM_RIGHT: MapArt = art(METAL.0, METAL.1, (219.0, 283.0, 153.0, 217.0));
/// `!ui-frame-metal-edgeleft-2x` (8514).
pub const EDGE_LEFT: MapArt = art(METAL_VERTICAL.0, METAL_VERTICAL.1, (1.0, 151.0, 0.0, 32.0));
/// `!ui-frame-metal-edgeright-2x` (8515).
pub const EDGE_RIGHT: MapArt = art(
    METAL_VERTICAL.0,
    METAL_VERTICAL.1,
    (153.0, 303.0, 0.0, 32.0),
);
/// `_ui-frame-metal-edgetop-2x` (8517).
pub const EDGE_TOP: MapArt = art(
    METAL_HORIZONTAL.0,
    METAL_HORIZONTAL.1,
    (0.0, 64.0, 1.0, 151.0),
);
/// `_ui-frame-metal-edgebottom-2x` (8516).
pub const EDGE_BOTTOM: MapArt = art(
    METAL_HORIZONTAL.0,
    METAL_HORIZONTAL.1,
    (0.0, 32.0, 153.0, 217.0),
);
/// `redbutton-exit-2x` (16908).
pub const CLOSE_BUTTON: MapArt = art(RED_BUTTONS.0, RED_BUTTONS.1, (39.0, 75.0, 1.0, 39.0));
/// `redbutton-expand-2x` (16912) and `redbutton-condense-2x` (16904).
pub const EXPAND_BUTTON: MapArt = art(RED_BUTTONS.0, RED_BUTTONS.1, (77.0, 113.0, 1.0, 39.0));
pub const CONDENSE_BUTTON: MapArt = art(RED_BUTTONS.0, RED_BUTTONS.1, (1.0, 37.0, 1.0, 39.0));
/// `Interface\QuestFrame\UI-QuestLog-BookIcon`, the windowed portrait.
pub const BOOK_ICON: u32 = 136_797;
/// `interface/framegeneral/ui-background-rock.blp`, the frame background.
pub const BACKGROUND: MapArt = whole(374_155, 256.0);
/// `interface/worldmap/worldmaparrow.blp`, the player pin (`UnitPositionFrame`).
pub const PLAYER_ARROW: MapArt = whole(803_894, 32.0);
/// `TaxiNode_Alliance` (6617).
pub const TAXI_ALLIANCE: MapArt = art(OBJECT_ICONS.0, OBJECT_ICONS.1, (627.0, 659.0, 798.0, 830.0));
/// `TaxiNode_Horde` (6618).
pub const TAXI_HORDE: MapArt = art(OBJECT_ICONS.0, OBJECT_ICONS.1, (627.0, 659.0, 832.0, 864.0));
/// `TaxiNode_Neutral` (6619).
pub const TAXI_NEUTRAL: MapArt = art(OBJECT_ICONS.0, OBJECT_ICONS.1, (627.0, 659.0, 866.0, 898.0));
/// `VignetteKill` (4947) and `VignetteKillElite` (4951), 32×32.
pub const VIGNETTE_KILL: MapArt = art(OBJECT_ICONS.0, OBJECT_ICONS.1, (599.0, 663.0, 197.0, 261.0));
pub const VIGNETTE_KILL_ELITE: MapArt =
    art(OBJECT_ICONS.0, OBJECT_ICONS.1, (203.0, 267.0, 395.0, 459.0));
/// `UI-QuestPoi-QuestNumber` (23605): numbered objective circle.
pub const QUEST_NUMBER: MapArt = art(QUEST_POI.0, QUEST_POI.1, (67.0, 99.0, 35.0, 67.0));
/// `UI-QuestPoi-QuestBangTurnIn` (25009): completed quest turn-in.
pub const QUEST_TURN_IN: MapArt = art(QUEST_POI.0, QUEST_POI.1, (67.0, 99.0, 1.0, 33.0));

/// Every texture the frame chrome draws, for hosts that cache textures on demand.
pub const CHROME_FDIDS: [u32; 9] = [
    METAL.0,
    METAL_VERTICAL.0,
    METAL_HORIZONTAL.0,
    RED_BUTTONS.0,
    BOOK_ICON,
    374_155,
    803_894,
    OBJECT_ICONS.0,
    QUEST_POI.0,
];
