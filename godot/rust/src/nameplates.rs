//! In-world nameplates with Retail visibility: the shared `nameplate_visibility_data`
//! rules decide which units get a plate (default CVars: only the target and units
//! fighting the player, enemies only), and a camera ray against terrain and WMO
//! collision dims plates of units behind world geometry (`nameplateOccludedAlphaMult`).
//! Look and layout follow docs/specs/nameplate-style.md: the reference-derived health
//! frame and fill skins of the Bevy client, the fill tinted by the unit's reaction to the
//! local player, and on the Thick bar the white outlined Friz name at its left with the
//! health value and percent at its right (the user's reference of 2026-10-04).

use std::collections::HashMap;
use std::path::Path;

use game_engine_core::nameplate_style_data::{
    NameplateStyle, THICK_HEALTH_HEIGHT, THIN_HEALTH_HEIGHT,
};
use game_engine_core::nameplate_visibility_data::{
    NameplateCvars, PlateUnit, in_combat_with_player, nameplate_alpha, plate_alpha, plate_shown,
    selection_in_combat_is_hostile,
};
use game_engine_core::status_text_data::{abbreviate_large_numbers, percent};
use game_engine_core::warband_scene_data::{AtlasArt, read_atlas_art};
use game_engine_network::replica::{Replica, Unit as ReplicatedUnit};
use game_engine_session::SessionScreen;
use game_engine_ui_model::inworld_unit_frames_component::{
    RAID_TARGET_ICONS_FDID, raid_target_tex_coords,
};
use godot::{
    builtin::Side,
    classes::{
        AtlasTexture, Camera3D, CanvasLayer, ColorRect, Control, Image, ImageTexture, Label,
        NinePatchRect, PhysicsRayQueryParameters3D, TextureRect, control::MouseFilter,
        text_server::OverrunBehavior, texture_rect::ExpandMode, texture_rect::StretchMode,
    },
    prelude::*,
};
use shared::{
    casting::CastState,
    components::{CreatureClassification, Health, Player, UnitFlags, UnitLevel},
    faction_reaction::{Unit, can_attack},
    level_scaling::{LevelScaling, level_for_viewer},
    protocol::RAID_TARGET_ICON_COUNT,
};
use ui_toolkit::atlas::{self, ActiveSkin};

use crate::{
    GameClient,
    faction_reaction::{FactionTemplateEntry, Reaction, parse_faction_template_csv, reaction},
    nameplate_auras::{self, PlateAura, aura_icon_rect, plate_auras},
    nameplate_cast_bar::{CastArt, CastNodes, atlas_art, place, skin_art, text_bbcode},
    nameplate_casts::{BarType, Interrupter, PlateCasts},
    replicated::UnitFields,
    targeting::{unit_classification, unit_pick_shape},
    wmo::collision::{TERRAIN_LAYER, WMO_LAYER},
};

const LAYER_NAME: &str = "Nameplates";
const FACTION_TEMPLATE_CSV: &str = "db2/12.1.0.69933/FactionTemplate.csv";
/// Retail `HEALTH_BAR_TO_NAME_ABOVE_SPACING` (Blizzard_NamePlateConstants.lua:33).
const NAME_ABOVE_BAR_SPACING: f32 = 2.0;
/// The texts inside the Thick bar start and end this far from the body's ends
/// (data/diagnostics/forever-reference/user-nameplate-reference-2026-10-04.png).
const TEXT_INSET: f32 = 2.0;
/// The least space between the name and the health text; a longer name is trimmed.
const NAME_HEALTH_GAP: f32 = 6.0;
/// Bevy `NAMEPLATE_SCALE`: the skins are unscaled reference-screenshot pixels.
const NAMEPLATE_SCALE: f32 = 0.5;
/// Bevy `BAR_Y_OFFSET` (health_bar.rs): the health body centre above the unit origin, in
/// the unit's space.
const BAR_Y_OFFSET: f32 = 2.5;
/// Retail `RaidTargetFrame` size (Blizzard_NamePlates.xml:185-193).
const RAID_ICON_SIZE: f32 = 22.0;
/// `ClassificationFrame` and its `classificationIndicator` size (Blizzard_NamePlates.xml:196,203).
const CLASSIFICATION_SIZE: f32 = 20.0;
const ELITE_GOLD_ATLAS: &str = "nameplates-icon-elite-gold";
const ELITE_SILVER_ATLAS: &str = "nameplates-icon-elite-silver";
const RARE_STAR_ATLAS: &str = "UI-HUD-UnitFrame-Target-PortraitOn-Boss-Rare-Star";
/// `PixelUtil.SetPoint(RaidTargetFrame, "BOTTOM", name, "TOP", 0, 10)` on name-only plates.
const RAID_ICON_ABOVE_NAME: f32 = 10.0;
/// `LEVEL_INDICATOR_WIDTH`, `LARGE_`/`SMALL_LEVEL_INDICATOR_HEIGHT` and `LEVEL_FONT_HEIGHT`
/// (Camelot/Blizzard_NamePlateConstants.lua:40,43,44,48) at `classification` scale 1.
const LEVEL_INDICATOR_WIDTH: f32 = 28.0;
const LARGE_LEVEL_INDICATOR_HEIGHT: f32 = 23.0;
const SMALL_LEVEL_INDICATOR_HEIGHT: f32 = 16.0;
const LEVEL_FONT_HEIGHT: i32 = 10;
/// Outline of the plate's texts, in pixels.
const TEXT_OUTLINE: i32 = 2;
/// Physics layers that hide a unit from the camera.
const OCCLUDER_MASK: u32 = TERRAIN_LAYER | WMO_LAYER;

/// Bevy `HealthSkin` (health_bar.rs), in reference pixels: the fill is shorter than the
/// body and the frame bitmap extends past it by a fixed margin.
struct HealthSkin {
    fill_inset: f32,
    fill_y: f32,
    frame_margin: Vector2,
    frame_offset: Vector2,
    /// Transparent columns at the frame bitmap's left and right ends: the drawn border
    /// starts and ends this far inside the bitmap (health-thick.png: ink in columns
    /// 4..386 of 396; health-thin.png: 3..387).
    frame_blank: (f32, f32),
}

fn health_skin(thick: bool, forever: bool) -> HealthSkin {
    if forever {
        return HealthSkin {
            fill_inset: 0.0,
            fill_y: 0.0,
            frame_margin: Vector2::splat(2.0),
            frame_offset: Vector2::ZERO,
            frame_blank: (0.0, 0.0),
        };
    }
    let (fill_inset, fill_y, margin, offset, blank) = if thick {
        (
            2.0,
            0.0,
            Vector2::new(20.0, 8.0),
            Vector2::new(2.0, -1.0),
            (4.0, 10.0),
        )
    } else {
        (
            1.0,
            0.5,
            Vector2::new(20.0, 10.0),
            Vector2::new(2.0, 0.0),
            (3.0, 9.0),
        )
    };
    HealthSkin {
        fill_inset: fill_inset * NAMEPLATE_SCALE,
        fill_y: fill_y * NAMEPLATE_SCALE,
        frame_margin: margin * NAMEPLATE_SCALE,
        frame_offset: offset * NAMEPLATE_SCALE,
        frame_blank: (blank.0 * NAMEPLATE_SCALE, blank.1 * NAMEPLATE_SCALE),
    }
}

/// Bevy `NameplateStyle::health_preset`: the nearer of the Thin/Thick heights.
fn thick_preset(style: &NameplateStyle) -> bool {
    (style.health_height - THICK_HEALTH_HEIGHT).abs()
        <= (style.health_height - THIN_HEALTH_HEIGHT).abs()
}

/// Plate part rectangles relative to the anchor point (the health body's centre), y down,
/// in UI pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PlateLayout {
    frame: Rect2,
    fill: Rect2,
    text: PlateText,
    /// The y of the plate's highest part (the frame, the body without one, or the name
    /// above it), which the aura icons stand over.
    top: f32,
    /// The health row's visible left and right edge: the frame's drawn border (the body
    /// without one), and on the right the level frame's end under a skin with one. The
    /// cast row sits on the same span.
    span: (f32, f32),
    /// Bottom edge of the rendered health frame, or fill when borderless.
    bottom: f32,
}

/// Where a plate's texts sit, relative to the anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PlateText {
    /// Thick bar (the user's reference): the name's left middle and the health text's
    /// right middle, both inside the body.
    Inside {
        name_left: Vector2,
        health_right: Vector2,
    },
    /// Thin bar, lower than a text line: the name's bottom centre above the plate
    /// (Retail `CenteredAboveHealthBar`), and no health text.
    Above { name_bottom: Vector2 },
}

/// `level_width` is what the skin's level frame takes off the health bars' right end
/// (`HealthBarsContainer` BOTTOMRIGHT `xOffset = -levelFrameWidth`,
/// Blizzard_NamePlateUnitFrame.lua:701-707); a name above stays centred on the whole plate.
fn plate_layout(style: &NameplateStyle, fraction: f32, level_width: f32) -> PlateLayout {
    let skin = health_skin(thick_preset(style), level_width > 0.0);
    let body = Vector2::new(style.health_width - level_width, style.health_height);
    let frame_size = body + skin.frame_margin;
    // Bevy parity: the anchor is the centre of the health bars and level frame.
    let center = Vector2::new(-level_width / 2.0, 0.0);
    let frame = Rect2::new(center + skin.frame_offset - frame_size / 2.0, frame_size);
    let fill_size = Vector2::new(body.x * fraction, body.y - skin.fill_inset);
    let fill = Rect2::new(
        center + Vector2::new(-body.x / 2.0, skin.fill_y - fill_size.y / 2.0),
        fill_size,
    );
    let plate_top = if style.show_border {
        frame.position.y
    } else {
        -body.y / 2.0
    };
    let (text, top) = if thick_preset(style) {
        let left = center.x - body.x / 2.0;
        let text = PlateText::Inside {
            name_left: Vector2::new(left + TEXT_INSET, 0.0),
            health_right: Vector2::new(left + body.x - TEXT_INSET, 0.0),
        };
        (text, plate_top)
    } else {
        let name_bottom = Vector2::new(0.0, plate_top - NAME_ABOVE_BAR_SPACING);
        // Bevy `name_clearance`: the name line is as high as its font size.
        (
            PlateText::Above { name_bottom },
            name_bottom.y - style.name_font_size,
        )
    };
    let (left, right) = if style.show_border {
        (
            frame.position.x + skin.frame_blank.0,
            frame.end().x - skin.frame_blank.1,
        )
    } else {
        (center.x - body.x / 2.0, center.x + body.x / 2.0)
    };
    let right = if level_width > 0.0 {
        style.health_width / 2.0
    } else {
        right
    };
    PlateLayout {
        frame,
        fill,
        text,
        top,
        span: (left, right),
        bottom: if style.show_border {
            frame.end().y
        } else {
            fill.end().y
        },
    }
}

/// The text at the bar's right: the percent of `UnitHealthMax` ("100%"), after
/// `AbbreviateLargeNumbers(UnitHealth)` when the style shows the value (the reference's
/// "425 K  100%").
fn health_text(health: &Health, show_value: bool) -> String {
    let (current, max) = (health.current.round() as i64, health.max.round() as i64);
    let percent = percent(current, max);
    if show_value {
        format!("{}  {percent}%", abbreviate_large_numbers(current))
    } else {
        format!("{percent}%")
    }
}

/// The name's width on the Thick bar: its own, or what the health text leaves it.
fn name_width_inside(name: f32, health: f32, name_left: f32, health_right: f32) -> f32 {
    name.min((health_right - health - NAME_HEALTH_GAP - name_left).max(0.0))
}

/// The atlases of the level frame a skin hangs on its plates.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LevelAtlases {
    icon: &'static str,
    selected: &'static str,
    skull: &'static str,
}

/// Forever plates carry Camelot's `NameplateLevelFrame`
/// (Camelot/Blizzard_NamePlateLevelFrame.xml:10,19,25; `ShouldDisplay` is true for every
/// unit, Camelot/Blizzard_NamePlateLevelFrame.lua:3-17). Mainline's shows only under the
/// Plunderstorm game rule (Blizzard_NamePlateLevelFrame.lua:9), so Modern has none.
fn level_frame_atlases(skin: ActiveSkin) -> Option<LevelAtlases> {
    match skin {
        ActiveSkin::Modern => None,
        ActiveSkin::Forever => Some(LevelAtlases {
            icon: "ui-hud-nameplates-levelindicator",
            selected: "ui-hud-nameplates-levelindicator-selected",
            skull: "ui-hud-nameplates-levelindicator-skull",
        }),
    }
}

/// Level frame part rectangles relative to the plate anchor, y down.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LevelLayout {
    /// The frame, which `playerLevelDiffIcon` fills and the level text centres on.
    frame: Rect2,
    selected: Rect2,
    skull: Rect2,
}

/// `PlayerLevelDiffFrame`: `playerLevelDiffWidth` × `playerLevelDiffHeight`
/// (Blizzard_NamePlateUnitFrame.lua:237; the large height on the thick health bar,
/// `GetLevelIndicatorHeight`, Blizzard_NamePlates.lua:396-404), LEFT on the health bars'
/// RIGHT (xml:6); `selectedBorder` 1 px wider and 2 px taller on each side (xml:21-22);
/// `highLevelTexture` a square of the frame's height on its centre (lua:240, xml:28).
fn level_layout(style: &NameplateStyle) -> LevelLayout {
    let height = if thick_preset(style) {
        LARGE_LEVEL_INDICATOR_HEIGHT
    } else {
        SMALL_LEVEL_INDICATOR_HEIGHT
    };
    let size = Vector2::new(LEVEL_INDICATOR_WIDTH, height);
    let frame = Rect2::new(
        Vector2::new(style.health_width / 2.0 - size.x, -height / 2.0),
        size,
    );
    LevelLayout {
        frame,
        selected: Rect2::new(
            frame.position - Vector2::new(1.0, 2.0),
            size + Vector2::new(2.0, 4.0),
        ),
        skull: Rect2::new(
            frame.center() - Vector2::splat(height / 2.0),
            Vector2::splat(height),
        ),
    }
}

/// `RaidTargetFrame` placement (Blizzard_NamePlateUnitFrame.lua:809-817), relative to
/// the anchor: RIGHT on the health bars' LEFT, or BOTTOM 10px above the name's TOP when
/// the plate shows only the name.
fn raid_icon_rect(style: &NameplateStyle, show_health_bars: bool, name: Rect2) -> Rect2 {
    let size = Vector2::splat(RAID_ICON_SIZE);
    let position = if show_health_bars {
        Vector2::new(-style.health_width / 2.0 - size.x, -size.y / 2.0)
    } else {
        Vector2::new(
            name.position.x + (name.size.x - size.x) / 2.0,
            name.position.y - RAID_ICON_ABOVE_NAME - size.y,
        )
    };
    Rect2::new(position, size)
}

/// `NamePlateClassificationFrameMixin:GetClassificationAtlasElement`
/// (Blizzard_NamePlateClassificationFrame.lua:90-130) on Retail defaults: only
/// `NamePlateEnemyFrameOptions` sets `showClassificationIndicator`
/// (Blizzard_NamePlateFrameOptions.lua:53,84; chosen for every unit that is not
/// `UnitIsFriend`, Blizzard_NamePlateBase.lua:57-63), `nameplateInfoDisplay`'s default
/// `\x02D` has the `RarityIcon` bit, and a raid icon or a name-only plate hides it.
fn classification_atlas(
    classification: CreatureClassification,
    friend: bool,
    raid_target: Option<u8>,
    show_only_name: bool,
) -> Option<&'static str> {
    if friend || raid_target.is_some() || show_only_name {
        return None;
    }
    match classification {
        CreatureClassification::Elite | CreatureClassification::WorldBoss => Some(ELITE_GOLD_ATLAS),
        CreatureClassification::Rare => Some(RARE_STAR_ATLAS),
        CreatureClassification::RareElite => Some(ELITE_SILVER_ATLAS),
        _ => None,
    }
}

/// `ClassificationFrame` (Blizzard_NamePlates.xml:195-214): 20×20, RIGHT on the
/// `RaidTargetFrame`'s LEFT, whose own RIGHT sits on the health bars' LEFT
/// (Blizzard_NamePlateUnitFrame.lua:815). `classificationScale` is 1: our plates have
/// only the Medium size (Blizzard_NamePlateConstants.lua:57).
fn classification_rect(style: &NameplateStyle) -> Rect2 {
    let size = Vector2::splat(CLASSIFICATION_SIZE);
    let raid_left = -style.health_width / 2.0 - RAID_ICON_SIZE;
    Rect2::new(Vector2::new(raid_left - size.x, -size.y / 2.0), size)
}

/// The three PvE classification atlases, cropped from their `UiTextureAtlas` sheets.
fn classification_textures(
    data_root: &Path,
) -> Result<HashMap<&'static str, Gd<AtlasTexture>>, String> {
    let names = [ELITE_GOLD_ATLAS, ELITE_SILVER_ATLAS, RARE_STAR_ATLAS];
    let atlases = read_atlas_art(data_root, &names)?;
    names
        .into_iter()
        .map(|name| {
            Ok((
                name,
                atlas_art(&atlases[&name.to_ascii_lowercase()], data_root)?,
            ))
        })
        .collect()
}

/// `SetRaidTargetIconTexture`: one `UI-RaidTargetingIcons` cell per icon 1–8.
fn raid_icon_textures(data_root: &Path) -> Result<Vec<Gd<AtlasTexture>>, String> {
    let path = data_root.join(format!("textures/{RAID_TARGET_ICONS_FDID}.blp"));
    let bytes =
        std::fs::read(&path).map_err(|error| format!("Read {}: {error}", path.display()))?;
    let image = game_engine_core::blp::decode_rgba(&bytes)
        .map_err(|error| format!("Decode {}: {error}", path.display()))?;
    let (width, height) = (image.width as f32, image.height as f32);
    let image = Image::create_from_data(
        image.width as i32,
        image.height as i32,
        false,
        godot::classes::image::Format::RGBA8,
        &PackedByteArray::from(image.pixels.as_slice()),
    )
    .ok_or_else(|| format!("Godot rejected {}", path.display()))?;
    let sheet = ImageTexture::create_from_image(&image)
        .ok_or_else(|| format!("Godot rejected {}", path.display()))?;
    Ok((1..=RAID_TARGET_ICON_COUNT as u8)
        .map(|index| {
            let [left, right, top, bottom] = raid_target_tex_coords(index);
            let mut cell = AtlasTexture::new_gd();
            cell.set_atlas(&sheet);
            cell.set_region(Rect2::new(
                Vector2::new(left * width, top * height),
                Vector2::new((right - left) * width, (bottom - top) * height),
            ));
            cell
        })
        .collect())
}

fn nameplate_text_color(is_player: bool, colorblind_mode: bool) -> Color {
    if !colorblind_mode {
        return Color::WHITE;
    }
    // Legacy `nameplate_text_color`: Player = Friendly, NPC = Neutral.
    // These are UnitReaction::name_color_for_mode(true); health tint is separate.
    if is_player {
        Color::from_rgb(0.45, 0.9, 1.0)
    } else {
        Color::from_rgb(1.0, 0.92, 0.35)
    }
}

fn reaction_color(style: &NameplateStyle, reaction: Reaction) -> Color {
    let [r, g, b] = match reaction {
        Reaction::Hostile => style.health_colors.hostile,
        Reaction::Neutral => style.health_colors.neutral,
        Reaction::Friendly => style.health_colors.friendly,
    };
    Color::from_rgb(r, g, b)
}

struct PlateArt {
    thick_frame: Gd<ImageTexture>,
    thin_frame: Gd<ImageTexture>,
    thick_fill: Gd<ImageTexture>,
    thin_fill: Gd<ImageTexture>,
    font: Gd<godot::classes::FontFile>,
}

fn health_frame_bytes(skin: ActiveSkin, thick: bool) -> &'static [u8] {
    if skin == ActiveSkin::Forever {
        return include_bytes!("rendering/ui/nameplate_skins/forever-health-edge.png");
    }
    if thick {
        include_bytes!("rendering/ui/nameplate_skins/health-thick.png")
    } else {
        include_bytes!("rendering/ui/nameplate_skins/health-thin.png")
    }
}

fn health_fill_bytes(skin: ActiveSkin, thick: bool) -> &'static [u8] {
    if skin == ActiveSkin::Forever {
        return include_bytes!("rendering/ui/nameplate_skins/forever-health-fill.png");
    }
    if thick {
        include_bytes!("rendering/ui/nameplate_skins/health-fill-thick.png")
    } else {
        include_bytes!("rendering/ui/nameplate_skins/health-fill.png")
    }
}

fn png_texture(bytes: &[u8], desaturate: bool) -> Result<Gd<ImageTexture>, String> {
    let mut image = Image::new_gd();
    let error = image.load_png_from_buffer(&PackedByteArray::from(bytes));
    if error != godot::global::Error::OK {
        return Err(format!("Decode nameplate skin: {error:?}"));
    }
    if desaturate {
        // Bevy `desaturate`: each pixel's HSV value, so the tint is the colour.
        image.convert(godot::classes::image::Format::RGBA8);
        let mut pixels = image.get_data().to_vec();
        for pixel in pixels.chunks_exact_mut(4) {
            let value = pixel[0].max(pixel[1]).max(pixel[2]);
            pixel[..3].fill(value);
        }
        let (width, height) = (image.get_width(), image.get_height());
        image.set_data(
            width,
            height,
            false,
            godot::classes::image::Format::RGBA8,
            &PackedByteArray::from(pixels.as_slice()),
        );
    }
    ImageTexture::create_from_image(&image).ok_or_else(|| "Godot rejected a nameplate skin".into())
}

impl PlateArt {
    fn load(active_skin: ActiveSkin) -> Result<Self, String> {
        let _span = crate::profile::span(|| "nameplates.art".to_owned());
        let skin = |bytes: &[u8]| png_texture(bytes, false);
        let fill = |bytes: &[u8]| png_texture(bytes, true);
        Ok(Self {
            thick_frame: skin(health_frame_bytes(active_skin, true))?,
            thin_frame: skin(health_frame_bytes(active_skin, false))?,
            thick_fill: fill(health_fill_bytes(active_skin, true))?,
            thin_fill: fill(health_fill_bytes(active_skin, false))?,
            font: crate::ui::assets::load_font(
                ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            )?,
        })
    }
}

/// The level frame's crops under the skin that draws one.
struct LevelArt {
    icon: Gd<AtlasTexture>,
    selected: Gd<AtlasTexture>,
    skull: Gd<AtlasTexture>,
}

/// The cached c60 member table's selected crop contains a skull in this local BLP.
/// Numeric crops of the actual sheet; provenance/retirement: nameplate-style.md.
fn level_art_crop(name: &str, skin: ActiveSkin) -> Result<AtlasArt, String> {
    if skin == ActiveSkin::Modern {
        return skin_art(name, skin);
    }
    let pixels = match name {
        "ui-hud-nameplates-levelindicator" => [27.0, 45.0, 45.0, 63.0],
        "ui-hud-nameplates-levelindicator-selected" => [29.0, 47.0, 25.0, 43.0],
        "ui-hud-nameplates-levelindicator-skull" => [1.0, 27.0, 25.0, 51.0],
        _ => return Err(format!("Unknown Forever nameplate level art: {name}")),
    };
    Ok(AtlasArt {
        fdid: 8_165_538,
        tex_coords: pixels.map(|pixel| pixel / 64.0),
    })
}

/// `UnitLevel` hides units ten or more levels above the viewer.
const UNKNOWN_LEVEL_DIFFERENCE: u16 = 10;

/// CompactUnitFrame.lua:1794-1815 shows the skull when `UnitLevel` is not positive.
/// Bosses and the +10 boundary emulate that unknown level with zero here.
/// Elite classification alone never hides a known level; absent data hides the slot.
fn displayed_plate_level(unit: ReplicatedUnit, viewer_level: Option<u8>) -> Option<u8> {
    let level = *unit.get::<UnitLevel>()?;
    let effective = level_for_viewer(
        level,
        unit.get::<LevelScaling>(),
        viewer_level.unwrap_or(level.0),
    );
    let too_high = viewer_level
        .is_some_and(|viewer| u16::from(effective) >= u16::from(viewer) + UNKNOWN_LEVEL_DIFFERENCE);
    let boss = unit_classification(unit) == CreatureClassification::WorldBoss;
    Some(if boss || too_high { 0 } else { effective })
}

impl LevelArt {
    fn load(atlases: LevelAtlases, skin: ActiveSkin, data_root: &Path) -> Result<Self, String> {
        let art = |name| atlas_art(&level_art_crop(name, skin)?, data_root);
        Ok(Self {
            icon: art(atlases.icon)?,
            selected: art(atlases.selected)?,
            skull: art(atlases.skull)?,
        })
    }
}

struct LevelNodes {
    root: Gd<Control>,
    icon: Gd<TextureRect>,
    selected: Gd<TextureRect>,
    skull: Gd<TextureRect>,
    text: Gd<Label>,
}

struct PlateNodes {
    root: Gd<Control>,
    frame: Gd<NinePatchRect>,
    fill: Gd<TextureRect>,
    name: Gd<Label>,
    health: Gd<Label>,
    raid_icon: Gd<TextureRect>,
    classification: Gd<TextureRect>,
    /// Present under a skin with a level frame.
    level: Option<LevelNodes>,
    cast: CastNodes,
    /// Aura icon slots, grown to the most auras the plate has shown.
    auras: Vec<AuraNodes>,
}

struct AuraNodes {
    border: Gd<ColorRect>,
    icon: Gd<TextureRect>,
    timer: Gd<Label>,
}

/// One unit's plate this frame, also reported to automation.
#[derive(Clone, Debug, PartialEq)]
struct PlateView {
    name: String,
    alpha: f32,
    occluded: bool,
    anchor: Vector2,
    fraction: f32,
    /// `health_text` of the unit's health; empty without one.
    health_text: String,
    color: Color,
    name_color: Color,
    /// `GetRaidTargetIndex(unit)`.
    raid_target: Option<u8>,
    /// `classificationIndicator`'s atlas, when shown.
    classification: Option<&'static str>,
    /// `UnitEffectiveLevel(unit)`.
    level: Option<u8>,
    /// The local player's target.
    targeted: bool,
    /// The player can attack the unit: its plate shows the player's debuffs.
    enemy: bool,
    auras: Vec<PlateAura>,
}

pub(crate) struct Nameplates {
    cvars: NameplateCvars,
    templates: Option<Result<HashMap<u32, FactionTemplateEntry>, String>>,
    art: Option<PlateArt>,
    /// The atlas skin `cast_art`, `level_art` and the plates were built for.
    skin: Option<ActiveSkin>,
    cast_art: Option<CastArt>,
    level_art: Option<LevelArt>,
    raid_icons: Option<Vec<Gd<AtlasTexture>>>,
    classification_icons: Option<HashMap<&'static str, Gd<AtlasTexture>>>,
    /// Every plate's cast bar.
    pub(crate) casts: PlateCasts,
    layer: Option<Gd<CanvasLayer>>,
    plates: HashMap<u64, PlateNodes>,
    views: HashMap<u64, PlateView>,
}

impl Nameplates {
    pub fn new() -> Self {
        Self {
            cvars: NameplateCvars::default(),
            templates: None,
            art: None,
            skin: None,
            cast_art: None,
            level_art: None,
            raid_icons: None,
            classification_icons: None,
            casts: PlateCasts::default(),
            layer: None,
            plates: HashMap::new(),
            views: HashMap::new(),
        }
    }

    /// The screen rect of every shown plate by unit, for the unit tooltip's nameplate hover.
    pub(crate) fn plate_rects(&self) -> impl Iterator<Item = (u64, Rect2)> + '_ {
        self.plates
            .iter()
            .filter(|(_, plate)| plate.root.is_visible_in_tree())
            .map(|(&id, plate)| {
                let frame = plate.frame.get_global_rect();
                let name = plate.name.get_global_rect();
                let rect = if plate.frame.is_visible_in_tree() {
                    frame.merge(name)
                } else {
                    name
                };
                (id, rect)
            })
    }

    fn clear(&mut self) {
        self.views.clear();
        self.casts = PlateCasts::default();
        for (_, plate) in self.plates.drain() {
            plate.root.free();
        }
    }

    pub(crate) fn templates(
        &mut self,
        data_root: &Path,
    ) -> Result<&HashMap<u32, FactionTemplateEntry>, String> {
        self.templates
            .get_or_insert_with(|| {
                let path = data_root.join(FACTION_TEMPLATE_CSV);
                std::fs::read_to_string(&path)
                    .map_err(|error| format!("Read {}: {error}", path.display()))
                    .and_then(|text| parse_faction_template_csv(&text))
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn sync_nodes(
        &mut self,
        parent: &mut Gd<Node3D>,
        views: HashMap<u64, PlateView>,
        style: &NameplateStyle,
        show_health_bars: bool,
        (data_root, icons): (&Path, &HashMap<u32, u32>),
    ) -> Result<(), String> {
        let skin = atlas::thread_skin();
        if self.art.is_none() || self.skin != Some(skin) {
            self.art = Some(PlateArt::load(skin)?);
        }
        if self.skin != Some(skin) {
            // Plates hold the previous skin's textures and frame tree.
            for (_, plate) in self.plates.drain() {
                plate.root.free();
            }
            self.cast_art = Some(CastArt::load(data_root, skin)?);
            self.level_art = level_frame_atlases(skin)
                .map(|atlases| LevelArt::load(atlases, skin, data_root))
                .transpose()?;
            self.skin = Some(skin);
        }
        if self.raid_icons.is_none() && views.values().any(|view| view.raid_target.is_some()) {
            self.raid_icons = Some(raid_icon_textures(data_root)?);
        }
        if self.classification_icons.is_none()
            && views.values().any(|view| view.classification.is_some())
        {
            self.classification_icons = Some(classification_textures(data_root)?);
        }
        let art = self.art.as_ref().expect("art loaded above");
        let raid_icons = self.raid_icons.as_deref().unwrap_or_default();
        let classification_icons = self.classification_icons.as_ref();
        let cast_art = self.cast_art.as_mut().expect("cast art loaded above");
        let level_art = self.level_art.as_ref();
        let level_width = level_art.map_or(0.0, |_| LEVEL_INDICATOR_WIDTH);
        let textures = data_root.join("textures");
        let layer = self.layer.get_or_insert_with(|| {
            let mut layer = CanvasLayer::new_alloc();
            layer.set_name(LAYER_NAME);
            // Below the registry UI layers (1): plates never cover HUD frames.
            layer.set_layer(0);
            parent.add_child(&layer);
            layer
        });
        self.plates.retain(|id, plate| {
            let keep = views.contains_key(id);
            if !keep {
                plate.root.clone().free();
            }
            keep
        });
        for (id, view) in &views {
            let plate = self
                .plates
                .entry(*id)
                .or_insert_with(|| spawn_plate(layer, art, cast_art, level_art));
            apply_plate(plate, view, style, level_width, art, show_health_bars);
            apply_level(plate, view, style, show_health_bars);
            apply_raid_icon(plate, view, style, show_health_bars, raid_icons);
            apply_classification(plate, view, style, classification_icons);
            // `ShouldShowCastBar`: no cast bar on a name-only plate.
            let bar = self.casts.get(*id).filter(|_| show_health_bars);
            let icon = match bar.and_then(|bar| icons.get(&bar.spell_id)) {
                Some(&fdid) => cast_art.icon(fdid, &textures)?,
                None => None,
            };
            let layout = plate_layout(style, view.fraction, level_width);
            plate.cast.apply(
                bar,
                icon.as_ref(),
                (style, layout.span, layout.bottom),
                cast_art,
            );
            // Bevy: aura icons show with the health bars.
            let auras = if show_health_bars {
                view.auras.as_slice()
            } else {
                &[]
            };
            let aura_icons = auras
                .iter()
                .map(|aura| cast_art.icon(aura.icon_fdid, &textures))
                .collect::<Result<Vec<_>, _>>()?;
            apply_auras(plate, auras, &aura_icons, style, layout.top, &art.font);
        }
        self.views = views;
        Ok(())
    }
}

fn ignore_mouse(control: &mut Gd<impl Inherits<Control>>) {
    control.upcast_mut().set_mouse_filter(MouseFilter::IGNORE);
}

fn spawn_level(
    parent: &mut Gd<Control>,
    art: &LevelArt,
    font: &Gd<godot::classes::FontFile>,
) -> LevelNodes {
    let mut root = Control::new_alloc();
    root.set_name("PlayerLevelDiffFrame");
    ignore_mouse(&mut root);
    let texture_rect = |name: &str, texture: &Gd<AtlasTexture>| {
        let mut rect = TextureRect::new_alloc();
        rect.set_name(name);
        rect.set_texture(texture);
        rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
        rect.set_stretch_mode(StretchMode::SCALE);
        ignore_mouse(&mut rect);
        rect
    };
    let icon = texture_rect("playerLevelDiffIcon", &art.icon);
    let selected = texture_rect("selectedBorder", &art.selected);
    let skull = texture_rect("highLevelTexture", &art.skull);
    // `SystemFont_NamePlateLevel`: Friz Quadrata 10, outlined (GameFonts.xml:389-391).
    let mut text = Label::new_alloc();
    text.set_name("playerLevelDiffText");
    ignore_mouse(&mut text);
    text.add_theme_font_override("font", font);
    text.add_theme_font_size_override("font_size", LEVEL_FONT_HEIGHT);
    text.add_theme_color_override("font_color", Color::WHITE);
    text.add_theme_color_override("font_outline_color", Color::BLACK);
    text.add_theme_constant_override("outline_size", 1);
    // BACKGROUND icon under the OVERLAY text, border and skull.
    root.add_child(&icon);
    root.add_child(&text);
    root.add_child(&selected);
    root.add_child(&skull);
    parent.add_child(&root);
    LevelNodes {
        root,
        icon,
        selected,
        skull,
        text,
    }
}

/// `CompactUnitFrame_UpdatePlayerLevelDiff` (CompactUnitFrame.lua:1591-1620): the level,
/// or the skull for a level of 0 or less; `selectedBorder` on the target in white
/// `TARGET_BORDER_COLOR` (Camelot/Blizzard_NamePlateLevelFrame.lua:55-70). Hidden with the
/// health bars it hangs on.
fn apply_level(
    plate: &mut PlateNodes,
    view: &PlateView,
    style: &NameplateStyle,
    show_health_bars: bool,
) {
    let Some(nodes) = &mut plate.level else {
        return;
    };
    let Some(level) = view.level.filter(|_| show_health_bars) else {
        nodes.root.set_visible(false);
        return;
    };
    nodes.root.set_visible(true);
    let layout = level_layout(style);
    place(&mut nodes.icon, layout.frame);
    place(&mut nodes.selected, layout.selected);
    nodes.selected.set_visible(view.targeted);
    place(&mut nodes.skull, layout.skull);
    nodes.skull.set_visible(level == 0);
    nodes.text.set_visible(level > 0);
    let text = level.to_string();
    if nodes.text.get_text().to_string() != text {
        nodes.text.set_text(&text);
    }
    nodes.text.reset_size();
    let size = nodes.text.get_minimum_size();
    nodes.text.set_size(size);
    nodes.text.set_position(layout.frame.center() - size / 2.0);
}

fn spawn_plate(
    layer: &mut Gd<CanvasLayer>,
    art: &PlateArt,
    cast_art: &CastArt,
    level_art: Option<&LevelArt>,
) -> PlateNodes {
    let mut root = Control::new_alloc();
    ignore_mouse(&mut root);
    let texture_rect = || {
        let mut rect = TextureRect::new_alloc();
        rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
        rect.set_stretch_mode(StretchMode::SCALE);
        ignore_mouse(&mut rect);
        rect
    };
    let mut fill = texture_rect();
    let mut frame = NinePatchRect::new_alloc();
    ignore_mouse(&mut frame);
    // Forever's numeric hollow patch keeps a 1px edge at every authored bar size.
    // Modern uses zero patch margins, preserving its original stretched bitmap.
    let edge = i32::from(level_art.is_some());
    if level_art.is_some() {
        frame.set_texture_filter(godot::classes::canvas_item::TextureFilter::NEAREST);
        fill.set_texture_filter(godot::classes::canvas_item::TextureFilter::LINEAR);
    }
    for side in [Side::LEFT, Side::RIGHT, Side::TOP, Side::BOTTOM] {
        frame.set_patch_margin(side, edge);
    }
    let raid_icon = texture_rect();
    let classification = texture_rect();
    // White Friz with a black outline, as the reference's texts.
    let label = || {
        let mut label = Label::new_alloc();
        ignore_mouse(&mut label);
        label.add_theme_font_override("font", &art.font);
        label.add_theme_color_override("font_color", Color::WHITE);
        label.add_theme_color_override("font_outline_color", Color::BLACK);
        label.add_theme_constant_override("outline_size", TEXT_OUTLINE);
        label
    };
    let mut name = label();
    // A name longer than its room on the bar ends in an ellipsis, however little of it fits
    // (plain `TRIM_ELLIPSIS` drops the ellipsis under six remaining characters).
    name.set_text_overrun_behavior(OverrunBehavior::TRIM_ELLIPSIS_FORCE);
    let health = label();
    // Fill under the frame, whose interior is transparent.
    root.add_child(&fill);
    root.add_child(&frame);
    root.add_child(&name);
    root.add_child(&health);
    root.add_child(&raid_icon);
    root.add_child(&classification);
    let level = level_art.map(|level_art| spawn_level(&mut root, level_art, &art.font));
    let cast = CastNodes::spawn(&mut root, cast_art, &art.font);
    layer.add_child(&root);
    PlateNodes {
        root,
        frame,
        fill,
        name,
        health,
        raid_icon,
        classification,
        level,
        cast,
        auras: Vec::new(),
    }
}

/// Bevy `spawn_slot`: a black border square under the icon and its countdown, white 9px
/// Friz with a black shadow, on the icon's centre.
fn spawn_aura(root: &mut Gd<Control>, font: &Gd<godot::classes::FontFile>) -> AuraNodes {
    let mut border = ColorRect::new_alloc();
    border.set_color(Color::from_rgba(0.0, 0.0, 0.0, 0.9));
    ignore_mouse(&mut border);
    let mut icon = TextureRect::new_alloc();
    icon.set_expand_mode(ExpandMode::IGNORE_SIZE);
    icon.set_stretch_mode(StretchMode::SCALE);
    ignore_mouse(&mut icon);
    let mut timer = Label::new_alloc();
    ignore_mouse(&mut timer);
    timer.add_theme_font_override("font", font);
    timer.add_theme_font_size_override("font_size", nameplate_auras::TIMER_FONT_SIZE);
    timer.add_theme_color_override("font_color", Color::WHITE);
    timer.add_theme_color_override("font_shadow_color", Color::BLACK);
    timer.add_theme_constant_override("shadow_offset_x", 1);
    timer.add_theme_constant_override("shadow_offset_y", 1);
    root.add_child(&border);
    root.add_child(&icon);
    root.add_child(&timer);
    AuraNodes {
        border,
        icon,
        timer,
    }
}

/// One slot per aura in `auras`, the rest hidden.
fn apply_auras(
    plate: &mut PlateNodes,
    auras: &[PlateAura],
    icons: &[Option<Gd<godot::classes::Texture2D>>],
    style: &NameplateStyle,
    top: f32,
    font: &Gd<godot::classes::FontFile>,
) {
    while plate.auras.len() < auras.len() {
        let nodes = spawn_aura(&mut plate.root, font);
        plate.auras.push(nodes);
    }
    for (slot, nodes) in plate.auras.iter_mut().enumerate() {
        let Some(aura) = auras.get(slot) else {
            nodes.border.set_visible(false);
            nodes.icon.set_visible(false);
            nodes.timer.set_visible(false);
            continue;
        };
        let rect = aura_icon_rect(style, slot, top);
        let border = Vector2::splat(nameplate_auras::BORDER);
        nodes.border.set_position(rect.position - border);
        nodes.border.set_size(rect.size + border * 2.0);
        nodes.border.set_visible(true);
        place(&mut nodes.icon, rect);
        if let Some(Some(texture)) = icons.get(slot) {
            nodes.icon.set_texture(texture);
        }
        nodes
            .icon
            .set_visible(matches!(icons.get(slot), Some(Some(_))));
        if nodes.timer.get_text().to_string() != aura.timer {
            nodes.timer.set_text(&aura.timer);
        }
        nodes.timer.reset_size();
        let size = nodes.timer.get_minimum_size();
        nodes.timer.set_size(size);
        nodes.timer.set_position(rect.center() - size / 2.0);
        nodes.timer.set_visible(true);
    }
}

/// `UpdateClassificationIndicator`: the atlas while one applies, else hidden.
fn apply_classification(
    plate: &mut PlateNodes,
    view: &PlateView,
    style: &NameplateStyle,
    textures: Option<&HashMap<&'static str, Gd<AtlasTexture>>>,
) {
    let texture = view
        .classification
        .zip(textures)
        .and_then(|(name, textures)| textures.get(name));
    let Some(texture) = texture else {
        plate.classification.set_visible(false);
        return;
    };
    let rect = classification_rect(style);
    plate.classification.set_texture(texture);
    plate.classification.set_position(rect.position);
    plate.classification.set_size(rect.size);
    plate.classification.set_visible(true);
}

/// `NamePlateRaidTargetMixin:SetRaidTargetIndex`: shown while the unit has an icon, also
/// on name-only plates.
fn apply_raid_icon(
    plate: &mut PlateNodes,
    view: &PlateView,
    style: &NameplateStyle,
    show_health_bars: bool,
    textures: &[Gd<AtlasTexture>],
) {
    let Some(index) = view.raid_target else {
        plate.raid_icon.set_visible(false);
        return;
    };
    let name = Rect2::new(plate.name.get_position(), plate.name.get_size());
    let rect = raid_icon_rect(style, show_health_bars, name);
    plate
        .raid_icon
        .set_texture(&textures[usize::from(index) - 1]);
    plate.raid_icon.set_position(rect.position);
    plate.raid_icon.set_size(rect.size);
    plate.raid_icon.set_visible(true);
}

fn apply_plate(
    plate: &mut PlateNodes,
    view: &PlateView,
    style: &NameplateStyle,
    level_width: f32,
    art: &PlateArt,
    show_health_bars: bool,
) {
    let thick = thick_preset(style);
    let layout = plate_layout(style, view.fraction, level_width);
    plate.root.set_position(view.anchor);
    plate
        .root
        .set_modulate(Color::from_rgba(1.0, 1.0, 1.0, view.alpha));
    plate
        .frame
        .set_visible(show_health_bars && style.show_border);
    plate.frame.set_texture(if thick {
        &art.thick_frame
    } else {
        &art.thin_frame
    });
    plate.frame.set_position(layout.frame.position);
    plate.frame.set_size(layout.frame.size);
    plate
        .fill
        .set_visible(show_health_bars && view.fraction > 0.0);
    plate.fill.set_texture(if thick {
        &art.thick_fill
    } else {
        &art.thin_fill
    });
    plate.fill.set_self_modulate(view.color);
    plate.fill.set_position(layout.fill.position);
    plate.fill.set_size(layout.fill.size);
    let font_size = style.name_font_size.round() as i32;
    let text_size = |text: &str| {
        art.font
            .get_string_size_ex(text)
            .font_size(font_size)
            .done()
    };
    for (label, text) in [
        (&mut plate.name, &view.name),
        (&mut plate.health, &view.health_text),
    ] {
        if label.get_text().to_string() != *text {
            label.set_text(text);
        }
        label.add_theme_font_size_override("font_size", font_size);
    }
    plate
        .name
        .add_theme_color_override("font_color", view.name_color);
    let name = text_size(&view.name);
    let inside = match layout.text {
        PlateText::Inside {
            name_left,
            health_right,
        } if show_health_bars => {
            let health = text_size(&view.health_text);
            plate.health.set_size(health);
            plate
                .health
                .set_position(health_right - Vector2::new(health.x, health.y / 2.0));
            let width = name_width_inside(name.x, health.x, name_left.x, health_right.x);
            plate.name.set_size(Vector2::new(width, name.y));
            plate
                .name
                .set_position(name_left - Vector2::new(0.0, name.y / 2.0));
            true
        }
        PlateText::Above { name_bottom } if show_health_bars => {
            plate.name.set_size(name);
            plate
                .name
                .set_position(name_bottom - Vector2::new(name.x / 2.0, name.y));
            false
        }
        // Name-only plate: the name on the anchor.
        _ => {
            plate.name.set_size(name);
            plate.name.set_position(-name / 2.0);
            false
        }
    };
    plate.health.set_visible(inside);
}

fn unit_name(unit: ReplicatedUnit) -> String {
    unit.name().unwrap_or_default().to_owned()
}

fn health_fraction(unit: ReplicatedUnit) -> f32 {
    unit.get::<Health>()
        .filter(|health| health.max > 0.0)
        .map_or(1.0, |health| (health.current / health.max).clamp(0.0, 1.0))
}

/// Whether world geometry (terrain, WMO collision) lies between the camera and `point`.
fn occluded(camera: &Gd<Camera3D>, point: Vector3) -> bool {
    let Some(mut space) = camera
        .get_world_3d()
        .and_then(|world| world.get_direct_space_state())
    else {
        return false;
    };
    let Some(mut query) =
        PhysicsRayQueryParameters3D::create_ex(camera.get_global_position(), point)
            .collision_mask(OCCLUDER_MASK)
            .done()
    else {
        return false;
    };
    query.set_collide_with_areas(false);
    !space.intersect_ray(&query).is_empty()
}

/// The unit's plate anchor (Bevy `BAR_Y_OFFSET` above its origin) and occlusion probe
/// (the centre of its pick box), once its model has loaded.
fn unit_points(node: &Gd<Node3D>) -> Option<(Vector3, Vector3)> {
    let probe = unit_pick_shape(node)?.get_global_position();
    Some((plate_anchor(node), probe))
}

/// Where a unit's plate sits: Bevy `BAR_Y_OFFSET` above its origin.
pub(crate) fn plate_anchor(node: &Gd<Node3D>) -> Vector3 {
    node.get_global_transform() * Vector3::new(0.0, BAR_Y_OFFSET, 0.0)
}

/// The local player as the plate rules see it.
struct Viewer<'a> {
    id: u64,
    target: Option<u64>,
    position: Vector3,
    template: Option<&'a FactionTemplateEntry>,
}

fn build_viewer<'a>(
    world: &crate::world::WorldUnits,
    units: &Replica,
    target: Option<u64>,
    templates: &'a HashMap<u32, FactionTemplateEntry>,
) -> Option<Viewer<'a>> {
    let id = world.local_player_id()?;
    let node = world.local_player_node()?;
    Some(Viewer {
        id,
        target,
        position: node.get_global_position(),
        template: units
            .unit(id)
            .and_then(UnitFields::faction_template)
            .and_then(|template| templates.get(&template)),
    })
}

fn plate_rule_input(
    viewer: &Viewer,
    unit: ReplicatedUnit,
    template: Option<&FactionTemplateEntry>,
    distance: f32,
) -> PlateUnit {
    let flags = UnitFlags(unit.unit_flags().unwrap_or_default());
    let is_player = unit.has::<Player>();
    let attacker = Unit {
        template: viewer.template,
        is_player: true,
    };
    let defender = Unit {
        template,
        is_player,
    };
    PlateUnit {
        is_local_player: unit.server_id == viewer.id,
        selectable: flags.is_selectable(),
        alive: unit
            .get::<Health>()
            .is_none_or(|health| health.current > 0.0),
        is_player,
        enemy: flags.is_attackable() && can_attack(attacker, defender),
        targeted: viewer.target == Some(unit.server_id),
        in_combat_with_player: in_combat_with_player(
            unit.in_combat(),
            unit.unit_target(),
            unit.threat_list(),
            viewer.id,
        ),
        distance,
    }
}

/// A shown unit's plate on screen, or none while its head is off screen.
fn project_plate(
    camera: &Gd<Camera3D>,
    cvars: &NameplateCvars,
    unit: ReplicatedUnit,
    node: &Gd<Node3D>,
    (color, name_color): (Color, Color),
    fade_far: f32,
    selected: bool,
) -> Option<PlateView> {
    let (top, center) = unit_points(node)?;
    if !camera.is_position_in_frustum(top) {
        return None;
    }
    let fade = nameplate_alpha(camera.get_global_position().distance_to(top), fade_far);
    if fade <= 0.0 && !selected {
        return None;
    }
    let is_occluded = occluded(camera, center);
    Some(PlateView {
        name: unit_name(unit),
        alpha: plate_alpha(cvars, selected, fade, is_occluded),
        occluded: is_occluded,
        anchor: camera.unproject_position(top),
        fraction: health_fraction(unit),
        health_text: String::new(),
        color,
        name_color,
        raid_target: None,
        classification: None,
        level: None,
        targeted: selected,
        enemy: false,
        auras: Vec::new(),
    })
}

impl GameClient {
    /// Per frame after the camera moves: which units have plates, their alpha, and nodes.
    pub(super) fn update_nameplates(&mut self, delta: f32) -> Result<(), String> {
        let enabled = self.account.session.screen == SessionScreen::InWorld
            && self.client_options.hud.show_nameplates;
        let camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d());
        let (Some(camera), true) = (camera, enabled) else {
            self.nameplates.clear();
            return Ok(());
        };
        let mut views = self.nameplate_views(&camera)?;
        for (id, view) in &mut views {
            view.auras = plate_auras(&self.unit_auras(*id), view.enemy);
            for aura in &mut view.auras {
                aura.icon_fdid = self.drawable_fdid(aura.icon_fdid);
            }
        }
        for id in views.keys() {
            let cast = self
                .replica
                .unit(*id)
                .and_then(|unit| unit.get::<CastState>());
            self.nameplates.casts.observe(*id, cast);
        }
        self.nameplates
            .casts
            .advance(delta, |id| views.contains_key(&id));
        let icons = self.nameplate_cast_icons();
        let style = self.client_options.hud.nameplate_style;
        let show_health_bars = self.client_options.hud.show_health_bars;
        let mut parent = self.to_gd().upcast::<Node3D>();
        let data_root = self.data_root.clone();
        self.nameplates.sync_nodes(
            &mut parent,
            views,
            &style,
            show_health_bars,
            (&data_root, &icons),
        )
    }

    /// Spell ID to drawable icon FDID for every shown cast bar.
    fn nameplate_cast_icons(&mut self) -> HashMap<u32, u32> {
        let spells: Vec<u32> = self
            .nameplates
            .casts
            .iter()
            .map(|(_, bar)| bar.spell_id)
            .collect();
        spells
            .into_iter()
            .filter_map(|spell| {
                let fdid = self.spells.catalog()?.get(spell)?.icon_fdid;
                Some((spell, self.drawable_fdid(fdid)))
            })
            .collect()
    }

    /// `GetInterruptText`'s interrupter: its name, class-coloured for a player.
    pub(super) fn cast_interrupter(&self, unit: u64) -> Option<Interrupter> {
        let unit = self.replica.unit(unit)?;
        Some(Interrupter {
            name: unit.name()?.to_owned(),
            color: unit
                .get::<Player>()
                .map(|player| game_engine_ui_model::damage_meter_data::class_color(player.class)),
        })
    }

    fn nameplate_views(
        &mut self,
        camera: &Gd<Camera3D>,
    ) -> Result<HashMap<u64, PlateView>, String> {
        let target = self.targeting_target();
        let cvars = self.nameplates.cvars;
        let style = self.client_options.hud.nameplate_style;
        let fade_far = self.client_options.hud.nameplate_distance;
        let colorblind_mode = self.client_options.graphics.colorblind_mode;
        let show_health_bars = self.client_options.hud.show_health_bars;
        let viewer_level = self.player_level().map(|level| level as u8);
        let templates = self.nameplates.templates(&self.data_root)?;
        let Some(viewer) = build_viewer(&self.world, &self.replica, target, templates) else {
            return Ok(HashMap::new());
        };
        let group: Vec<_> = self
            .account
            .group
            .members
            .iter()
            .map(|member| member.character_id)
            .collect();
        let views = self
            .replica
            .units()
            .filter_map(|unit| {
                let node = self.world.unit_node(unit.server_id)?;
                let template = unit.faction_template().and_then(|id| templates.get(&id));
                let distance = node.get_global_position().distance_to(viewer.position);
                let rules = plate_rule_input(&viewer, unit, template, distance);
                if !node.is_visible_in_tree() || !plate_shown(&cvars, &rules) {
                    return None;
                }
                let reaction = reaction(template, viewer.template);
                let friendly = reaction == Reaction::Friendly;
                let color = if self
                    .account
                    .session
                    .selected_character_id
                    .is_some_and(|character| unit.tap_denied(character, &group))
                {
                    // CompactUnitFrame.lua:677-678: tap-denied grey, before reaction colour.
                    Color::from_rgb(0.5, 0.5, 0.5)
                } else if selection_in_combat_is_hostile(unit.threat_list(), viewer.id, friendly) {
                    // CompactUnitFrame_UpdateHealthColor: `r, g, b = 1.0, 0.0, 0.0`.
                    Color::from_rgb(1.0, 0.0, 0.0)
                } else {
                    reaction_color(&style, reaction)
                };
                let name_color = nameplate_text_color(unit.has::<Player>(), colorblind_mode);
                let mut view = project_plate(
                    camera,
                    &cvars,
                    unit,
                    &node,
                    (color, name_color),
                    fade_far,
                    rules.targeted,
                )?;
                view.health_text = unit
                    .get::<Health>()
                    .filter(|health| health.max > 0.0)
                    .map(|health| health_text(health, style.show_health_value))
                    .unwrap_or_default();
                view.level = displayed_plate_level(unit, viewer_level);
                view.enemy = rules.enemy;
                view.raid_target = self.account.raid_targets.icon_of(unit.server_id);
                view.classification = classification_atlas(
                    unit_classification(unit),
                    friendly,
                    view.raid_target,
                    !show_health_bars,
                );
                Some((unit.server_id, view))
            })
            .collect();
        Ok(views)
    }

    /// The plate rule inputs for one unit, for automation; empty when it is unknown.
    pub(super) fn nameplate_rule_state(&mut self, id: u64) -> Result<VarDictionary, String> {
        let mut state = VarDictionary::new();
        let target = self.targeting_target();
        let cvars = self.nameplates.cvars;
        let templates = self.nameplates.templates(&self.data_root)?;
        let (Some(viewer), Some(node), Some(unit)) = (
            build_viewer(&self.world, &self.replica, target, templates),
            self.world.unit_node(id),
            self.replica.unit(id),
        ) else {
            return Ok(state);
        };
        let template = unit.faction_template().and_then(|id| templates.get(&id));
        let distance = node.get_global_position().distance_to(viewer.position);
        let rules = plate_rule_input(&viewer, unit, template, distance);
        state.set("enemy", rules.enemy);
        state.set("selectable", rules.selectable);
        state.set("alive", rules.alive);
        state.set("targeted", rules.targeted);
        state.set("in_combat_with_player", rules.in_combat_with_player);
        state.set("distance", rules.distance);
        state.set("shown", plate_shown(&cvars, &rules));
        state.set(
            "faction_template",
            unit.faction_template().unwrap_or_default() as i64,
        );
        state.set(
            "reaction",
            format!("{:?}", reaction(template, viewer.template)).as_str(),
        );
        Ok(state)
    }

    /// Plates for automation: unit id to name, alpha, occlusion, screen anchor, fill.
    pub(super) fn nameplates_snapshot(&self) -> VarDictionary {
        let mut plates = VarDictionary::new();
        for (id, view) in &self.nameplates.views {
            let mut entry = VarDictionary::new();
            entry.set("name", view.name.as_str());
            entry.set("alpha", view.alpha);
            entry.set("occluded", view.occluded);
            entry.set("anchor", view.anchor);
            entry.set("fraction", view.fraction);
            entry.set("health_text", view.health_text.as_str());
            entry.set("color", view.color);
            if let Some(index) = view.raid_target {
                entry.set("raid_target", i64::from(index));
            }
            if let Some(plate) = self.nameplates.plates.get(id) {
                entry.set("frame_rect", plate.frame.get_global_rect());
                entry.set("name_rect", plate.name.get_global_rect());
                if plate.health.is_visible() {
                    entry.set("health_rect", plate.health.get_global_rect());
                }
                if plate.raid_icon.is_visible() {
                    entry.set("raid_icon_rect", plate.raid_icon.get_global_rect());
                }
                if let Some(atlas) = view
                    .classification
                    .filter(|_| plate.classification.is_visible())
                {
                    entry.set("classification", atlas);
                    entry.set(
                        "classification_rect",
                        plate.classification.get_global_rect(),
                    );
                }
            }
            if let Some(bar) = self.nameplates.casts.get(*id) {
                entry.set("cast", &cast_snapshot(bar, self.nameplates.plates.get(id)));
            }
            let auras: VarArray = view
                .auras
                .iter()
                .map(|aura| {
                    let mut entry = VarDictionary::new();
                    entry.set("icon_fdid", i64::from(aura.icon_fdid));
                    entry.set("timer", aura.timer.as_str());
                    entry.to_variant()
                })
                .collect();
            entry.set("auras", &auras);
            plates.set(*id as i64, &entry);
        }
        plates
    }
}

fn cast_snapshot(
    bar: &crate::nameplate_casts::CastBar,
    plate: Option<&PlateNodes>,
) -> VarDictionary {
    let mut cast = VarDictionary::new();
    cast.set("spell_id", i64::from(bar.spell_id));
    let bar_type = match bar.bar_type {
        BarType::Standard => "standard",
        BarType::Channel => "channel",
        BarType::Uninterruptable => "uninterruptable",
        BarType::Interrupted => "interrupted",
    };
    cast.set("bar_type", bar_type);
    cast.set("casting", bar.casting);
    cast.set("channeling", bar.channeling);
    cast.set("fraction", bar.fraction());
    cast.set("alpha", bar.alpha());
    cast.set("text", text_bbcode(bar).as_str());
    cast.set("icon_shown", bar.icon_shown);
    cast.set("shield_shown", bar.shield_shown);
    cast.set("spark", format!("{:?}", bar.spark).as_str());
    if let Some(plate) = plate {
        let nodes = &plate.cast;
        cast.set("visible", nodes.root.is_visible_in_tree());
        cast.set("icon_visible", nodes.icon.is_visible_in_tree());
        cast.set("shield_visible", nodes.shield.is_visible_in_tree());
        cast.set("text_rect", nodes.text.get_global_rect());
    }
    cast
}

/// Script access to the native occlusion ray and alpha, for physics-level fixtures.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
pub struct NameplateProbe {
    base: Base<RefCounted>,
}

#[godot_api]
impl NameplateProbe {
    /// Whether terrain or WMO collision hides `point` from `camera`.
    #[func]
    fn occluded(camera: Gd<Camera3D>, point: Vector3) -> bool {
        occluded(&camera, point)
    }

    /// The Retail-default plate alpha for an occluded or clear unit.
    #[func]
    fn alpha(is_occluded: bool) -> f32 {
        plate_alpha(&NameplateCvars::default(), false, 1.0, is_occluded)
    }
}

#[path = "nameplate_debug.rs"]
mod debug;

#[cfg(test)]
#[path = "nameplate_skin_tests.rs"]
mod skin_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::{CombatStatus, Npc, UnitFactionTemplate, UnitThreatList};

    #[test]
    fn nameplate_alpha_distances_preserve_selected_and_occlusion_overrides() {
        let cvars = NameplateCvars::default();
        // The authored HUD limit is 40 yd, not Retail's alpha-distance CVars.
        for (distance, expected_fade) in [
            (10.0, 1.0),
            (20.0, 1.0),
            (30.0, 0.5),
            (35.62, 0.219),
            (40.0, 0.0),
            (60.0, 0.0),
        ] {
            let fade = nameplate_alpha(distance, 40.0);
            assert!((fade - expected_fade).abs() < 1e-6, "{distance} yd");
            for (selected, occluded, expected) in [
                (false, false, expected_fade),
                (false, true, expected_fade * 0.4),
                (true, false, 1.0),
                (true, true, 0.4),
            ] {
                let alpha = plate_alpha(&cvars, selected, fade, occluded);
                assert!(
                    (alpha - expected).abs() < 1e-6,
                    "{distance} yd, selected={selected}, occluded={occluded}: {alpha}"
                );
            }
        }
    }

    const SHOT: u64 = 30;
    const GUARD: u64 = 41;
    const KOBOLD: u64 = 42;
    /// FactionTemplate.csv (build 12.1.0.69933) rows 1 (Human player), 11 (Stormwind
    /// guard, Faction 72) and 26 (Kobold Vermin's creature_template faction 25).
    const TEMPLATES: &str = "\
ID,Faction,Flags,FactionGroup,FriendGroup,EnemyGroup,Enemies_0,Enemies_1,Enemies_2,Enemies_3,Enemies_4,Enemies_5,Enemies_6,Enemies_7,Friend_0,Friend_1,Friend_2,Friend_3,Friend_4,Friend_5,Friend_6,Friend_7
1,1,72,3,2,12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
11,72,2081,3,2,12,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
26,25,1,8,0,1,0,0,0,0,0,0,0,0,25,0,0,0,0,0,0,0
";

    fn npc(replica: &mut Replica, id: u64, name: &str, template: u32) {
        replica.insert(
            id,
            Npc {
                template_id: 6,
                name: name.into(),
            },
        );
        replica.insert(id, UnitFactionTemplate(template));
        replica.insert(
            id,
            Health {
                current: 55.0,
                max: 55.0,
            },
        );
    }

    /// Whether unit `id`, 10 yd away, has a plate under the Retail default CVars while
    /// the Human player targets `target`.
    fn shown(replica: &Replica, id: u64, target: Option<u64>) -> bool {
        let templates = parse_faction_template_csv(TEMPLATES).unwrap();
        let viewer = Viewer {
            id: SHOT,
            target,
            position: Vector3::ZERO,
            template: templates.get(&1),
        };
        let unit = replica.unit(id).unwrap();
        let template = unit.faction_template().and_then(|id| templates.get(&id));
        let rules = plate_rule_input(&viewer, unit, template, 10.0);
        plate_shown(&NameplateCvars::default(), &rules)
    }

    /// `nameplateShowFriendlyNpcs = 0` (cvars.yaml:1004): the targeted Stormwind guard has
    /// no plate, as the Stormwind Army Registrar in shot3-2026-10-03/forever-target.webp.
    #[test]
    fn a_targeted_friendly_npc_has_no_plate_under_the_default_cvars() {
        let mut replica = Replica::for_tests();
        npc(&mut replica, GUARD, "Stormwind Guard", 11);
        assert!(!shown(&replica, GUARD, Some(GUARD)));
    }

    /// `nameplateShowEnemies = 1`, `nameplateShowAll = 0`: a Kobold Vermin has a plate
    /// while it is the target or has the player on its threat list, and none otherwise.
    #[test]
    fn a_hostile_npc_has_a_plate_while_targeted_or_fighting_the_player() {
        let mut replica = Replica::for_tests();
        npc(&mut replica, KOBOLD, "Kobold Vermin", 26);
        assert!(!shown(&replica, KOBOLD, None));
        assert!(shown(&replica, KOBOLD, Some(KOBOLD)));
        replica.insert(KOBOLD, CombatStatus(true));
        replica.insert(KOBOLD, UnitThreatList(vec![SHOT]));
        assert!(shown(&replica, KOBOLD, None));
    }

    #[test]
    fn colorblind_mode_changes_player_and_npc_labels_but_not_health_fill() {
        let style = NameplateStyle::default();
        let fill = reaction_color(&style, Reaction::Hostile);
        assert_eq!(nameplate_text_color(true, false), Color::WHITE);
        assert_eq!(nameplate_text_color(false, false), Color::WHITE);
        assert_eq!(
            nameplate_text_color(true, true),
            Color::from_rgb(0.45, 0.9, 1.0)
        );
        assert_eq!(
            nameplate_text_color(false, true),
            Color::from_rgb(1.0, 0.92, 0.35)
        );
        assert_eq!(reaction_color(&style, Reaction::Hostile), fill);
    }

    #[test]
    fn thick_plate_centres_its_body_on_the_anchor_with_the_texts_inside_its_ends() {
        let style = NameplateStyle::default();
        let layout = plate_layout(&style, 1.0, 0.0);
        // 188x20 body; the Thick frame adds 10x4 and sits 1px right, 0.5px up.
        assert_eq!(layout.frame.size, Vector2::new(198.0, 24.0));
        assert_eq!(layout.frame.position, Vector2::new(-98.0, -12.5));
        assert_eq!(layout.fill.size, Vector2::new(188.0, 19.0));
        assert_eq!(layout.fill.position, Vector2::new(-94.0, -9.5));
        // The reference: the name 2px inside the body's left end, the health text 2px
        // inside its right end, both on the bar's middle line.
        assert_eq!(
            layout.text,
            PlateText::Inside {
                name_left: Vector2::new(-92.0, 0.0),
                health_right: Vector2::new(92.0, 0.0),
            }
        );
    }

    /// The health and cast rows form one block: the cast track and its icon start on the
    /// health frame's drawn left edge and the track ends on the plate's right edge (the
    /// frame's drawn right edge, or the level frame's end under Forever), for the Thick
    /// and Thin presets, with and without the border.
    #[test]
    fn cast_row_is_flush_with_the_health_row_at_both_ends() {
        use crate::nameplate_cast_bar::cast_layout;
        use game_engine_core::nameplate_style_data::NameplateBarThickness::{Thick, Thin};
        for (health, cast) in [(Thick, Thick), (Thin, Thin), (Thick, Thin), (Thin, Thick)] {
            for show_border in [true, false] {
                let style = NameplateStyle {
                    show_border,
                    ..NameplateStyle::from_presets(health, cast)
                };
                for level_width in [0.0, LEVEL_INDICATOR_WIDTH] {
                    let case =
                        format!("{health:?}/{cast:?} border {show_border} level {level_width}");
                    let plate = plate_layout(&style, 1.0, level_width);
                    let row = cast_layout(&style, 1.0, plate.span, plate.bottom);
                    assert_eq!(row.background.position.x, plate.span.0, "{case}");
                    assert_eq!(row.background.end().x, plate.span.1, "{case}");
                    assert_eq!(row.icon.position.x, plate.span.0, "{case}");
                    // The left edge is the frame's border, drawn inside its bitmap and
                    // outside the fill; borderless, the fill's own edge.
                    assert!(plate.span.0 >= plate.frame.position.x, "{case}");
                    assert!(plate.span.0 <= plate.fill.position.x, "{case}");
                    if !show_border {
                        assert_eq!(plate.span.0, plate.fill.position.x, "{case}");
                    }
                    if level_width > 0.0 {
                        let level = level_layout(&style).frame;
                        assert_eq!(plate.span.1, level.end().x, "{case}");
                    } else {
                        assert!(plate.span.1 <= plate.frame.end().x, "{case}");
                        assert!(plate.span.1 >= plate.fill.end().x, "{case}");
                    }
                }
            }
        }
    }

    /// By default the plate's health text is the percent alone, which Retail rounds up
    /// (`math.ceil`).
    #[test]
    fn health_text_is_the_percent_alone_by_default() {
        let show_value = NameplateStyle::default().show_health_value;
        let text = |current, max| health_text(&Health { current, max }, show_value);
        assert_eq!(text(425_000.0, 425_000.0), "100%");
        assert_eq!(text(324_275.0, 425_000.0), "77%");
        assert_eq!(text(42.0, 55.0), "77%");
    }

    /// With the style's health value on, the reference's "425 K  100%":
    /// `AbbreviateLargeNumbers` then the percent.
    #[test]
    fn health_text_leads_with_the_abbreviated_value_when_the_style_shows_it() {
        let text = |current, max| health_text(&Health { current, max }, true);
        assert_eq!(text(425_000.0, 425_000.0), "425 K  100%");
        assert_eq!(text(42.0, 55.0), "42  77%");
        assert_eq!(text(12_345.0, 20_000.0), "12,345  62%");
        assert_eq!(text(3_500_000_000.0, 4_000_000_000.0), "3500 M  88%");
    }

    /// On the 188px bar the texts span -92..92: "Stormwind Army Registrar" (150px wide)
    /// next to a 70px health text keeps 184 - 70 - 6 = 108px; "Kobold Vermin" (80px)
    /// keeps its own width.
    #[test]
    fn a_long_name_is_trimmed_to_leave_the_health_text_its_room() {
        assert_eq!(name_width_inside(150.0, 70.0, -92.0, 92.0), 108.0);
        assert_eq!(name_width_inside(80.0, 70.0, -92.0, 92.0), 80.0);
    }

    #[test]
    fn fill_shrinks_from_the_right_with_health() {
        let style = NameplateStyle::default();
        let full = plate_layout(&style, 1.0, 0.0).fill;
        let half = plate_layout(&style, 0.5, 0.0).fill;
        assert_eq!(half.position, full.position);
        assert_eq!(half.size.x, full.size.x / 2.0);
    }

    #[test]
    fn borderless_thin_plate_names_sit_two_pixels_above_the_body() {
        let style = NameplateStyle {
            health_height: THIN_HEALTH_HEIGHT,
            show_border: false,
            ..NameplateStyle::default()
        };
        let layout = plate_layout(&style, 1.0, 0.0);
        assert!(!thick_preset(&style));
        // 10px body centred on the anchor, name 2px above its top.
        assert_eq!(
            layout.text,
            PlateText::Above {
                name_bottom: Vector2::new(0.0, -7.0)
            }
        );
        // The aura icons clear the 13px name line; on the Thick plate, the frame's top.
        assert_eq!(layout.top, -20.0);
        assert_eq!(
            plate_layout(&NameplateStyle::default(), 1.0, 0.0).top,
            -12.5
        );
    }

    /// Retail `RaidTargetFrame` (22×22) hangs RIGHT on the health bars' LEFT: the
    /// 188×20 body spans -94..94, so the icon spans -116..-94 × -11..11.
    #[test]
    fn raid_icon_sits_left_of_the_health_bar() {
        let style = NameplateStyle::default();
        let name = Rect2::new(Vector2::new(-30.0, -28.5), Vector2::new(60.0, 14.0));
        assert_eq!(
            raid_icon_rect(&style, true, name),
            Rect2::new(Vector2::new(-116.0, -11.0), Vector2::new(22.0, 22.0))
        );
    }

    /// Name-only plates (`IsShowOnlyName`) put the icon's BOTTOM 10px above the name's TOP.
    #[test]
    fn raid_icon_stands_above_a_name_only_plate() {
        let style = NameplateStyle::default();
        let name = Rect2::new(Vector2::new(-30.0, -7.0), Vector2::new(60.0, 14.0));
        assert_eq!(
            raid_icon_rect(&style, false, name),
            Rect2::new(Vector2::new(-11.0, -39.0), Vector2::new(22.0, 22.0))
        );
    }

    /// Timber (creature_template 1132, rank 4 rare): the `rare` branch's star.
    #[test]
    fn timber_rare_plate_shows_the_rare_star() {
        assert_eq!(
            classification_atlas(CreatureClassification::Rare, false, None, false),
            Some("UI-HUD-UnitFrame-Target-PortraitOn-Boss-Rare-Star")
        );
    }

    /// Hogger (creature_template 448, rank 1 elite): the gold elite dragon; world bosses
    /// share it and rare elites get the silver one.
    #[test]
    fn hogger_elite_plate_shows_the_gold_dragon() {
        assert_eq!(
            classification_atlas(CreatureClassification::Elite, false, None, false),
            Some("nameplates-icon-elite-gold")
        );
        assert_eq!(
            classification_atlas(CreatureClassification::WorldBoss, false, None, false),
            Some("nameplates-icon-elite-gold")
        );
        assert_eq!(
            classification_atlas(CreatureClassification::RareElite, false, None, false),
            Some("nameplates-icon-elite-silver")
        );
    }

    /// A normal Kobold Vermin (rank 0) shows nothing; friendly plates use
    /// `NamePlateFriendlyFrameOptions` (`showClassificationIndicator = false`); a raid
    /// icon or a name-only plate hides the frame.
    #[test]
    fn normal_friendly_marked_or_name_only_plates_show_no_classification() {
        use CreatureClassification::{Elite, Normal};
        assert_eq!(classification_atlas(Normal, false, None, false), None);
        assert_eq!(classification_atlas(Elite, true, None, false), None);
        assert_eq!(classification_atlas(Elite, false, Some(8), false), None);
        assert_eq!(classification_atlas(Elite, false, None, true), None);
    }

    /// The 20×20 `ClassificationFrame` hangs RIGHT on the 22×22 `RaidTargetFrame`'s LEFT,
    /// which hangs on the health bars' LEFT (-94): -136..-116 × -10..10, ending where the
    /// raid icon's -116..-94 begins.
    #[test]
    fn classification_sits_left_of_the_raid_icon_slot() {
        let style = NameplateStyle::default();
        let rect = classification_rect(&style);
        assert_eq!(
            rect,
            Rect2::new(Vector2::new(-136.0, -10.0), Vector2::new(20.0, 20.0))
        );
        let name = Rect2::new(Vector2::new(-30.0, -28.5), Vector2::new(60.0, 14.0));
        assert_eq!(rect.end().x, raid_icon_rect(&style, true, name).position.x);
    }
}
