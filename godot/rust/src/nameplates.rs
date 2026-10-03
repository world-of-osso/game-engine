//! In-world nameplates with Retail visibility: the shared `nameplate_visibility_data`
//! rules decide which units get a plate (default CVars: only the target and units
//! fighting the player, enemies only), and a camera ray against terrain and WMO
//! collision dims plates of units behind world geometry (`nameplateOccludedAlphaMult`).
//! Look and layout follow docs/specs/nameplate-style.md: the reference-derived health
//! frame and fill skins of the Bevy client, the fill tinted by the unit's reaction to the
//! local player, and the white Friz name 2px above the plate.

use std::collections::HashMap;
use std::path::Path;

use game_engine_core::nameplate_style_data::{
    NameplateStyle, THICK_HEALTH_HEIGHT, THIN_HEALTH_HEIGHT,
};
use game_engine_core::nameplate_visibility_data::{
    NameplateCvars, PlateUnit, in_combat_with_player, nameplate_alpha, plate_alpha, plate_shown,
    selection_in_combat_is_hostile,
};
use game_engine_network::replica::{Replica, Unit as ReplicatedUnit};
use game_engine_session::SessionScreen;
use game_engine_ui_model::inworld_unit_frames_component::{
    RAID_TARGET_ICONS_FDID, raid_target_tex_coords,
};
use godot::{
    classes::{
        AtlasTexture, Camera3D, CanvasLayer, Control, Image, ImageTexture, Label,
        PhysicsRayQueryParameters3D, TextureRect, control::MouseFilter, texture_rect::ExpandMode,
        texture_rect::StretchMode,
    },
    prelude::*,
};
use shared::{
    casting::CastState,
    components::{Health, Player, UnitFlags},
    faction_reaction::{Unit, can_attack},
    protocol::RAID_TARGET_ICON_COUNT,
};

use crate::{
    GameClient,
    faction_reaction::{FactionTemplateEntry, Reaction, parse_faction_template_csv, reaction},
    nameplate_cast_bar::{CastArt, CastNodes, text_bbcode},
    nameplate_casts::{BarType, Interrupter, PlateCasts},
    replicated::UnitFields,
    targeting::unit_pick_shape,
    wmo::collision::{TERRAIN_LAYER, WMO_LAYER},
};

const LAYER_NAME: &str = "Nameplates";
const FACTION_TEMPLATE_CSV: &str = "db2/12.1.0.69933/FactionTemplate.csv";
/// Retail `HEALTH_BAR_TO_NAME_ABOVE_SPACING` (Blizzard_NamePlateConstants.lua:33).
const NAME_ABOVE_BAR_SPACING: f32 = 2.0;
/// Bevy `NAMEPLATE_SCALE`: the skins are unscaled reference-screenshot pixels.
const NAMEPLATE_SCALE: f32 = 0.5;
/// Bevy `BAR_Y_OFFSET` (health_bar.rs): the health body centre above the unit origin, in
/// the unit's space.
const BAR_Y_OFFSET: f32 = 2.5;
/// Retail `RaidTargetFrame` size (Blizzard_NamePlates.xml:185-193).
const RAID_ICON_SIZE: f32 = 22.0;
/// `PixelUtil.SetPoint(RaidTargetFrame, "BOTTOM", name, "TOP", 0, 10)` on name-only plates.
const RAID_ICON_ABOVE_NAME: f32 = 10.0;
/// Physics layers that hide a unit from the camera.
const OCCLUDER_MASK: u32 = TERRAIN_LAYER | WMO_LAYER;

/// Bevy `HealthSkin` (health_bar.rs), in reference pixels: the fill is shorter than the
/// body and the frame bitmap extends past it by a fixed margin.
struct HealthSkin {
    fill_inset: f32,
    fill_y: f32,
    frame_margin: Vector2,
    frame_offset: Vector2,
}

fn health_skin(thick: bool) -> HealthSkin {
    let (fill_inset, fill_y, margin, offset) = if thick {
        (2.0, 0.0, Vector2::new(20.0, 8.0), Vector2::new(2.0, -1.0))
    } else {
        (1.0, 0.5, Vector2::new(20.0, 10.0), Vector2::new(2.0, 0.0))
    };
    HealthSkin {
        fill_inset: fill_inset * NAMEPLATE_SCALE,
        fill_y: fill_y * NAMEPLATE_SCALE,
        frame_margin: margin * NAMEPLATE_SCALE,
        frame_offset: offset * NAMEPLATE_SCALE,
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
    /// Bottom centre of the name label.
    name_bottom: Vector2,
}

fn plate_layout(style: &NameplateStyle, fraction: f32) -> PlateLayout {
    let skin = health_skin(thick_preset(style));
    let body = Vector2::new(style.health_width, style.health_height);
    let frame_size = body + skin.frame_margin;
    // Bevy parity: the anchor is the health body's centre.
    let center = Vector2::ZERO;
    let frame = Rect2::new(center + skin.frame_offset - frame_size / 2.0, frame_size);
    let fill_size = Vector2::new(body.x * fraction, body.y - skin.fill_inset);
    let fill = Rect2::new(
        center + Vector2::new(-body.x / 2.0, skin.fill_y - fill_size.y / 2.0),
        fill_size,
    );
    let top = if style.show_border {
        frame_size.y / 2.0 - skin.frame_offset.y
    } else {
        body.y / 2.0
    };
    PlateLayout {
        frame,
        fill,
        name_bottom: center - Vector2::new(0.0, top + NAME_ABOVE_BAR_SPACING),
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
    fn load() -> Result<Self, String> {
        let _span = crate::profile::span(|| "nameplates.art".to_owned());
        let skin = |bytes: &[u8]| png_texture(bytes, false);
        let fill = |bytes: &[u8]| png_texture(bytes, true);
        Ok(Self {
            thick_frame: skin(include_bytes!(
                "rendering/ui/nameplate_skins/health-thick.png"
            ))?,
            thin_frame: skin(include_bytes!(
                "rendering/ui/nameplate_skins/health-thin.png"
            ))?,
            thick_fill: fill(include_bytes!(
                "rendering/ui/nameplate_skins/health-fill-thick.png"
            ))?,
            thin_fill: fill(include_bytes!(
                "rendering/ui/nameplate_skins/health-fill.png"
            ))?,
            font: crate::ui::assets::load_font(
                ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            )?,
        })
    }
}

struct PlateNodes {
    root: Gd<Control>,
    frame: Gd<TextureRect>,
    fill: Gd<TextureRect>,
    name: Gd<Label>,
    raid_icon: Gd<TextureRect>,
    cast: CastNodes,
}

/// One unit's plate this frame, also reported to automation.
#[derive(Clone, Debug, PartialEq)]
struct PlateView {
    name: String,
    alpha: f32,
    occluded: bool,
    anchor: Vector2,
    fraction: f32,
    color: Color,
    name_color: Color,
    /// `GetRaidTargetIndex(unit)`.
    raid_target: Option<u8>,
}

pub(crate) struct Nameplates {
    cvars: NameplateCvars,
    templates: Option<Result<HashMap<u32, FactionTemplateEntry>, String>>,
    art: Option<PlateArt>,
    cast_art: Option<CastArt>,
    raid_icons: Option<Vec<Gd<AtlasTexture>>>,
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
            cast_art: None,
            raid_icons: None,
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
        if self.art.is_none() {
            self.art = Some(PlateArt::load()?);
        }
        if self.cast_art.is_none() {
            self.cast_art = Some(CastArt::load(data_root, |bytes| png_texture(bytes, false))?);
        }
        if self.raid_icons.is_none() && views.values().any(|view| view.raid_target.is_some()) {
            self.raid_icons = Some(raid_icon_textures(data_root)?);
        }
        let art = self.art.as_ref().expect("art loaded above");
        let raid_icons = self.raid_icons.as_deref().unwrap_or_default();
        let cast_art = self.cast_art.as_mut().expect("cast art loaded above");
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
        let thick = thick_preset(style);
        for (id, view) in &views {
            let plate = self
                .plates
                .entry(*id)
                .or_insert_with(|| spawn_plate(layer, art, cast_art));
            apply_plate(plate, view, style, thick, art, show_health_bars);
            apply_raid_icon(plate, view, style, show_health_bars, raid_icons);
            // `ShouldShowCastBar`: no cast bar on a name-only plate.
            let bar = self.casts.get(*id).filter(|_| show_health_bars);
            let icon = match bar.and_then(|bar| icons.get(&bar.spell_id)) {
                Some(&fdid) => cast_art.icon(fdid, &textures)?,
                None => None,
            };
            plate.cast.apply(bar, icon.as_ref(), style, cast_art);
        }
        self.views = views;
        Ok(())
    }
}

fn ignore_mouse(control: &mut Gd<impl Inherits<Control>>) {
    control.upcast_mut().set_mouse_filter(MouseFilter::IGNORE);
}

fn spawn_plate(layer: &mut Gd<CanvasLayer>, art: &PlateArt, cast_art: &CastArt) -> PlateNodes {
    let mut root = Control::new_alloc();
    ignore_mouse(&mut root);
    let texture_rect = || {
        let mut rect = TextureRect::new_alloc();
        rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
        rect.set_stretch_mode(StretchMode::SCALE);
        ignore_mouse(&mut rect);
        rect
    };
    let fill = texture_rect();
    let frame = texture_rect();
    let raid_icon = texture_rect();
    let mut name = Label::new_alloc();
    ignore_mouse(&mut name);
    name.add_theme_font_override("font", &art.font);
    name.add_theme_color_override("font_color", Color::WHITE);
    name.add_theme_color_override("font_shadow_color", Color::BLACK);
    name.add_theme_constant_override("shadow_offset_x", 1);
    name.add_theme_constant_override("shadow_offset_y", 1);
    // Fill under the frame, whose interior is transparent.
    root.add_child(&fill);
    root.add_child(&frame);
    root.add_child(&name);
    root.add_child(&raid_icon);
    let cast = CastNodes::spawn(&mut root, cast_art, &art.font);
    layer.add_child(&root);
    PlateNodes {
        root,
        frame,
        fill,
        name,
        raid_icon,
        cast,
    }
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
    thick: bool,
    art: &PlateArt,
    show_health_bars: bool,
) {
    let layout = plate_layout(style, view.fraction);
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
    if plate.name.get_text().to_string() != view.name {
        plate.name.set_text(&view.name);
    }
    plate
        .name
        .add_theme_color_override("font_color", view.name_color);
    plate
        .name
        .add_theme_font_size_override("font_size", style.name_font_size.round() as i32);
    plate.name.reset_size();
    let size = plate.name.get_minimum_size();
    plate.name.set_size(size);
    plate.name.set_position(if show_health_bars {
        layout.name_bottom - Vector2::new(size.x / 2.0, size.y)
    } else {
        -size / 2.0
    });
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
    node: &Gd<Node3D>,
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
        distance: node.get_global_position().distance_to(viewer.position),
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
        color,
        name_color,
        raid_target: None,
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
        let views = self.nameplate_views(&camera)?;
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
        let templates = self.nameplates.templates(&self.data_root)?;
        let Some(viewer) = build_viewer(&self.world, &self.replica, target, templates) else {
            return Ok(HashMap::new());
        };
        let views = self
            .replica
            .units()
            .filter_map(|unit| {
                let node = self.world.unit_node(unit.server_id)?;
                let template = unit.faction_template().and_then(|id| templates.get(&id));
                let rules = plate_rule_input(&viewer, unit, template, &node);
                if !node.is_visible_in_tree() || !plate_shown(&cvars, &rules) {
                    return None;
                }
                let reaction = reaction(template, viewer.template);
                let friendly = reaction == Reaction::Friendly;
                let color =
                    if selection_in_combat_is_hostile(unit.threat_list(), viewer.id, friendly) {
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
                view.raid_target = self.account.raid_targets.icon_of(unit.server_id);
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
        let rules = plate_rule_input(&viewer, unit, template, &node);
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
            entry.set("color", view.color);
            if let Some(index) = view.raid_target {
                entry.set("raid_target", i64::from(index));
            }
            if let Some(plate) = self.nameplates.plates.get(id) {
                entry.set("frame_rect", plate.frame.get_global_rect());
                entry.set("name_rect", plate.name.get_global_rect());
                if plate.raid_icon.is_visible() {
                    entry.set("raid_icon_rect", plate.raid_icon.get_global_rect());
                }
            }
            if let Some(bar) = self.nameplates.casts.get(*id) {
                entry.set("cast", &cast_snapshot(bar, self.nameplates.plates.get(id)));
            }
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
mod tests {
    use super::*;

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
    fn thick_plate_centres_its_body_on_the_anchor_with_the_name_two_pixels_above_the_frame() {
        let style = NameplateStyle::default();
        let layout = plate_layout(&style, 1.0);
        // 188x20 body; the Thick frame adds 10x4 and sits 1px right, 0.5px up.
        assert_eq!(layout.frame.size, Vector2::new(198.0, 24.0));
        assert_eq!(layout.frame.position, Vector2::new(-98.0, -12.5));
        assert_eq!(layout.fill.size, Vector2::new(188.0, 19.0));
        assert_eq!(layout.fill.position, Vector2::new(-94.0, -9.5));
        assert_eq!(layout.name_bottom, Vector2::new(0.0, -14.5));
    }

    #[test]
    fn fill_shrinks_from_the_right_with_health() {
        let style = NameplateStyle::default();
        let full = plate_layout(&style, 1.0).fill;
        let half = plate_layout(&style, 0.5).fill;
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
        let layout = plate_layout(&style, 1.0);
        assert!(!thick_preset(&style));
        // 10px body centred on the anchor, name 2px above its top.
        assert_eq!(layout.name_bottom.y, -7.0);
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
}
