//! Nameplate cast bar nodes: the Bevy client's reference cast frame and authored
//! `4505182` background/fill under the health bar (docs/specs/nameplate-style.md), with
//! Retail's `NamePlateCastingBarTemplate` spark pip, interrupt shield, spell icon and
//! spell name row (`ApplyStyleAndAnchoring`, non-classic, name below the bar), driven by
//! `nameplate_casts::CastBar`.

use std::collections::HashMap;
use std::path::Path;

use game_engine_core::nameplate_style_data::{NameplateBarThickness, NameplateStyle};
use game_engine_core::warband_scene_data::{AtlasArt, read_atlas_art};
use godot::{
    classes::{
        AtlasTexture, Control, FontFile, ImageTexture, RichTextLabel, Texture2D, TextureRect,
        control::MouseFilter, texture_rect::ExpandMode, texture_rect::StretchMode,
    },
    prelude::*,
};

use crate::assets::material::{load_texture, shared_texture, texture_from_rgba};
use crate::nameplate_casts::{BarType, CastBar, Spark};

/// Bevy `NAMEPLATE_SCALE`: reference skins are unscaled screenshot pixels.
const NAMEPLATE_SCALE: f32 = 0.5;
/// Bevy `HEALTH_CAST_GAP`: between the health body and the cast body.
const HEALTH_CAST_GAP: f32 = 4.0 * NAMEPLATE_SCALE;
/// Bevy Thin label row: its centre this far below the cast body's bottom.
const THIN_LABEL_DROP: f32 = 14.0 * NAMEPLATE_SCALE;
/// `NamePlateSetupOptions` at scale 1: `castIconWidth/Height`, `castBarShieldWidth/Height`.
const ICON_SIZE: f32 = 12.0;
const SHIELD_SIZE: Vector2 = Vector2::new(10.0, 12.0);
/// `Spark:SetSize(4, 12)` then `castBarHeight + CAST_BAR_SPARK_EXTRA_HEIGHT` (8).
const SPARK_WIDTH: f32 = 4.0;
const SPARK_EXTRA_HEIGHT: f32 = 8.0;
/// `Text` anchors LEFT to `Icon` RIGHT, 2 px over.
const ICON_TEXT_GAP: f32 = 2.0;
/// `interface/castingbar/uicastingbar.blp` and the Bevy client's crops of it
/// (`ui-castingbar-background`, `ui-castingbar-filling-standard`).
const CASTING_BAR_FDID: u32 = 4505182;
/// `[x, y, width, height]` in pixels.
const BACKGROUND_RECT: [f32; 4] = [57.0, 85.0, 209.0, 11.0];
const FILL_RECT: [f32; 4] = [268.0, 124.0, 209.0, 11.0];

fn rect([x, y, width, height]: [f32; 4]) -> Rect2 {
    Rect2::new(Vector2::new(x, y), Vector2::new(width, height))
}
/// `CASTBAR_CLASSIC_RED`, the interrupted and failed fill.
const INTERRUPTED_COLOR: [f32; 3] = [1.0, 0.0, 0.0];
const PIP: &str = "ui-castingbar-pip";
const PIP_RED: &str = "ui-castingbar-pip-red";
const SHIELD: &str = "nameplates-interruptshield";

/// Reference cast frame bitmap extent past the cast body and its offset (raw pixels),
/// Bevy `cast_frame_margin`.
fn frame_margin(preset: NameplateBarThickness) -> (Vector2, Vector2) {
    let (offset, margin) = match preset {
        NameplateBarThickness::Thick => (Vector2::new(1.0, -1.5), Vector2::new(58.0, 9.0)),
        NameplateBarThickness::Thin => (Vector2::new(2.0, 0.0), Vector2::new(56.0, 10.0)),
    };
    (offset * NAMEPLATE_SCALE, margin * NAMEPLATE_SCALE)
}

/// Cast bar part rectangles relative to the plate anchor (the health body's centre),
/// y down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CastLayout {
    pub frame: Rect2,
    pub background: Rect2,
    pub fill: Rect2,
    pub spark: Rect2,
    pub icon: Rect2,
    pub shield: Rect2,
    /// Left middle of the spell name.
    pub text_left: Vector2,
}

pub(crate) fn cast_layout(style: &NameplateStyle, fraction: f32) -> CastLayout {
    let body = Vector2::new(style.cast_width, style.cast_height);
    let center = Vector2::new(
        0.0,
        style.health_height / 2.0 + HEALTH_CAST_GAP + body.y / 2.0,
    );
    let left = center.x - body.x / 2.0;
    let (offset, margin) = frame_margin(style.cast_preset());
    let frame_size = body + margin;
    let background = Rect2::new(center - body / 2.0, body);
    let fill = Rect2::new(background.position, Vector2::new(body.x * fraction, body.y));
    let spark_size = Vector2::new(SPARK_WIDTH, body.y + SPARK_EXTRA_HEIGHT);
    let spark = Rect2::new(
        Vector2::new(left + body.x * fraction, center.y) - spark_size / 2.0,
        spark_size,
    );
    let row = match style.cast_preset() {
        NameplateBarThickness::Thick => center.y,
        NameplateBarThickness::Thin => center.y + body.y / 2.0 + THIN_LABEL_DROP,
    };
    let icon = Rect2::new(
        Vector2::new(left, row - ICON_SIZE / 2.0),
        Vector2::splat(ICON_SIZE),
    );
    // `BorderShield` anchors RIGHT to `Icon` RIGHT.
    let shield = Rect2::new(
        Vector2::new(icon.end().x - SHIELD_SIZE.x, row - SHIELD_SIZE.y / 2.0),
        SHIELD_SIZE,
    );
    CastLayout {
        frame: Rect2::new(center + offset - frame_size / 2.0, frame_size),
        background,
        fill,
        spark,
        icon,
        shield,
        text_left: Vector2::new(icon.end().x + ICON_TEXT_GAP, row),
    }
}

/// Cast bar artwork shared by every plate.
pub(crate) struct CastArt {
    thick_frame: Gd<ImageTexture>,
    thin_frame: Gd<ImageTexture>,
    background: Gd<AtlasTexture>,
    /// `4505182` desaturated, so the style's cast colour tints its fill crop.
    fill_sheet: Gd<ImageTexture>,
    pip: Gd<AtlasTexture>,
    pip_red: Gd<AtlasTexture>,
    shield: Gd<AtlasTexture>,
    icons: HashMap<u32, Option<Gd<Texture2D>>>,
}

fn atlas_region(texture: &Gd<ImageTexture>, region: Rect2) -> Gd<AtlasTexture> {
    let mut atlas = AtlasTexture::new_gd();
    atlas.set_atlas(texture);
    atlas.set_region(region);
    atlas
}

/// Texture `fdid` under `data/textures`, copied from local CASC when absent.
fn ensure_texture(data_root: &Path, fdid: u32) -> Result<(), String> {
    let path = data_root.join("textures").join(format!("{fdid}.blp"));
    if path.exists()
        || crate::assets::creature::local_resolver(data_root)
            .ensure_cached(fdid, &path)
            .is_some()
    {
        Ok(())
    } else {
        Err(format!("nameplate texture {fdid} is not in local CASC"))
    }
}

pub(crate) fn atlas_art(art: &AtlasArt, data_root: &Path) -> Result<Gd<AtlasTexture>, String> {
    ensure_texture(data_root, art.fdid)?;
    let textures = data_root.join("textures");
    let textures = textures.as_path();
    let mut missing = PackedInt32Array::new();
    let texture = shared_texture(art.fdid, textures, &mut missing)?
        .ok_or_else(|| format!("missing nameplate texture {}", art.fdid))?;
    let (width, height) = (texture.get_width() as f32, texture.get_height() as f32);
    let [left, right, top, bottom] = art.tex_coords;
    let region = Rect2::new(
        Vector2::new(left * width, top * height),
        Vector2::new((right - left) * width, (bottom - top) * height),
    );
    Ok(atlas_region(&texture, region))
}

impl CastArt {
    pub fn load(
        data_root: &Path,
        skin: impl Fn(&[u8]) -> Result<Gd<ImageTexture>, String>,
    ) -> Result<Self, String> {
        ensure_texture(data_root, CASTING_BAR_FDID)?;
        let textures = data_root.join("textures");
        let mut missing = PackedInt32Array::new();
        let sheet = shared_texture(CASTING_BAR_FDID, &textures, &mut missing)?
            .ok_or_else(|| format!("missing nameplate cast texture {CASTING_BAR_FDID}"))?;
        let (mut pixels, width, height) = load_texture(CASTING_BAR_FDID, &textures, &mut missing)?
            .ok_or_else(|| format!("missing nameplate cast texture {CASTING_BAR_FDID}"))?;
        // Bevy `desaturated_copy`: each pixel's HSV value.
        for pixel in pixels.chunks_exact_mut(4) {
            let value = pixel[0].max(pixel[1]).max(pixel[2]);
            pixel[..3].fill(value);
        }
        let atlases = read_atlas_art(data_root, &[PIP, PIP_RED, SHIELD])?;
        Ok(Self {
            thick_frame: skin(include_bytes!(
                "rendering/ui/nameplate_skins/cast-thick.png"
            ))?,
            thin_frame: skin(include_bytes!("rendering/ui/nameplate_skins/cast-thin.png"))?,
            background: atlas_region(&sheet, rect(BACKGROUND_RECT)),
            fill_sheet: texture_from_rgba(&pixels, width, height)?,
            pip: atlas_art(&atlases[PIP], data_root)?,
            pip_red: atlas_art(&atlases[PIP_RED], data_root)?,
            shield: atlas_art(&atlases[SHIELD], data_root)?,
            icons: HashMap::new(),
        })
    }

    /// The spell's `SpellMisc` icon, once per FDID; `None` when it has none.
    pub fn icon(&mut self, fdid: u32, textures: &Path) -> Result<Option<Gd<Texture2D>>, String> {
        if let Some(icon) = self.icons.get(&fdid) {
            return Ok(icon.clone());
        }
        let mut missing = PackedInt32Array::new();
        let icon = if fdid == 0 {
            None
        } else {
            shared_texture(fdid, textures, &mut missing)?.map(|texture| texture.upcast())
        };
        self.icons.insert(fdid, icon.clone());
        Ok(icon)
    }
}

/// One plate's cast bar nodes, under the plate root.
pub(crate) struct CastNodes {
    pub root: Gd<Control>,
    frame: Gd<TextureRect>,
    background: Gd<TextureRect>,
    fill: Gd<TextureRect>,
    fill_region: Gd<AtlasTexture>,
    spark: Gd<TextureRect>,
    pub icon: Gd<TextureRect>,
    pub shield: Gd<TextureRect>,
    pub text: Gd<RichTextLabel>,
}

fn texture_rect(name: &str) -> Gd<TextureRect> {
    let mut rect = TextureRect::new_alloc();
    rect.set_name(name);
    rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
    rect.set_stretch_mode(StretchMode::SCALE);
    rect.set_mouse_filter(MouseFilter::IGNORE);
    rect
}

impl CastNodes {
    pub fn spawn(parent: &mut Gd<Control>, art: &CastArt, font: &Gd<FontFile>) -> Self {
        let mut root = Control::new_alloc();
        root.set_name("CastBar");
        root.set_mouse_filter(MouseFilter::IGNORE);
        let mut background = texture_rect("Background");
        background.set_texture(&art.background);
        let fill_region = atlas_region(&art.fill_sheet, rect(FILL_RECT));
        let mut fill = texture_rect("Fill");
        fill.set_texture(&fill_region);
        let frame = texture_rect("Border");
        let spark = texture_rect("Spark");
        let icon = texture_rect("Icon");
        let mut shield = texture_rect("BorderShield");
        shield.set_texture(&art.shield);
        let mut text = RichTextLabel::new_alloc();
        text.set_name("Text");
        text.set_mouse_filter(MouseFilter::IGNORE);
        text.set_use_bbcode(true);
        text.set_fit_content(true);
        text.set_autowrap_mode(godot::classes::text_server::AutowrapMode::OFF);
        text.set_scroll_active(false);
        text.add_theme_font_override("normal_font", font);
        text.add_theme_color_override("default_color", Color::WHITE);
        text.add_theme_color_override("font_shadow_color", Color::from_rgba(0.0, 0.0, 0.0, 0.85));
        text.add_theme_constant_override("shadow_offset_x", 1);
        text.add_theme_constant_override("shadow_offset_y", 1);
        // Fill above its background, frame over both, spark and icon row on top.
        for node in [
            background.clone().upcast::<Control>(),
            fill.clone().upcast(),
            frame.clone().upcast(),
            spark.clone().upcast(),
            icon.clone().upcast(),
            shield.clone().upcast(),
            text.clone().upcast(),
        ] {
            root.add_child(&node);
        }
        parent.add_child(&root);
        Self {
            root,
            frame,
            background,
            fill,
            fill_region,
            spark,
            icon,
            shield,
            text,
        }
    }

    /// Show `bar` (or nothing) under a plate drawn in `style`.
    pub fn apply(
        &mut self,
        bar: Option<&CastBar>,
        icon: Option<&Gd<Texture2D>>,
        style: &NameplateStyle,
        art: &CastArt,
    ) {
        let Some(bar) = bar else {
            self.root.set_visible(false);
            return;
        };
        self.root.set_visible(true);
        self.root
            .set_modulate(Color::from_rgba(1.0, 1.0, 1.0, bar.alpha()));
        let fraction = bar.fraction();
        let layout = cast_layout(style, fraction);
        place(&mut self.frame, layout.frame);
        self.frame.set_visible(style.show_border);
        self.frame.set_texture(match style.cast_preset() {
            NameplateBarThickness::Thick => &art.thick_frame,
            NameplateBarThickness::Thin => &art.thin_frame,
        });
        place(&mut self.background, layout.background);
        place(&mut self.fill, layout.fill);
        self.fill.set_visible(fraction > 0.0);
        // Bevy `cast_fill_crop`: the fill shows the left `fraction` of the crop.
        let [x, y, width, height] = FILL_RECT;
        self.fill_region
            .set_region(rect([x, y, width * fraction, height]));
        self.fill.set_self_modulate(fill_color(bar, style));
        place(&mut self.spark, layout.spark);
        self.spark.set_visible(bar.spark.is_some());
        if let Some(spark) = bar.spark {
            self.spark.set_texture(match spark {
                Spark::Pip => &art.pip,
                Spark::PipRed => &art.pip_red,
            });
        }
        place(&mut self.icon, layout.icon);
        self.icon.set_visible(bar.icon_shown && icon.is_some());
        if let Some(icon) = icon {
            self.icon.set_texture(icon);
        }
        place(&mut self.shield, layout.shield);
        self.shield.set_visible(bar.shield_shown);
        self.apply_text(bar, style, layout.text_left);
    }

    fn apply_text(&mut self, bar: &CastBar, style: &NameplateStyle, left_middle: Vector2) {
        let bbcode = text_bbcode(bar);
        if self.text.get_text().to_string() != bbcode {
            self.text.set_text(&bbcode);
        }
        let size = style.cast_font_size.round() as i32;
        self.text
            .add_theme_font_size_override("normal_font_size", size);
        let height = self.text.get_content_height() as f32;
        let width = self.text.get_content_width() as f32;
        self.text
            .set_size(Vector2::new(width.max(1.0), height.max(1.0)));
        self.text
            .set_position(left_middle - Vector2::new(0.0, height / 2.0));
    }
}

fn place(rect: &mut Gd<TextureRect>, at: Rect2) {
    rect.set_position(at.position);
    rect.set_size(at.size);
}

/// The style's cast colours by bar type (Retail `CastingBar` classic fill colours by
/// default), red once interrupted or failed.
fn fill_color(bar: &CastBar, style: &NameplateStyle) -> Color {
    let [r, g, b] = match bar.bar_type {
        BarType::Standard => style.cast_colors.normal,
        BarType::Channel => style.cast_colors.channel,
        BarType::Uninterruptable => style.cast_colors.uninterruptible,
        BarType::Interrupted => INTERRUPTED_COLOR,
    };
    Color::from_rgb(r, g, b)
}

/// The bar's text with the interrupter's name in its class colour
/// (`RAID_CLASS_COLORS:WrapTextInColorCode`).
pub(crate) fn text_bbcode(bar: &CastBar) -> String {
    let mut text = escape(&bar.text.text);
    if let Some(name) = &bar.text.name {
        match name.color {
            Some([r, g, b]) => {
                let color = Color::from_rgb(r, g, b).to_html_without_alpha();
                text.push_str(&format!("[color=#{color}]{}[/color]", escape(&name.name)));
            }
            None => text.push_str(&escape(&name.name)),
        }
    }
    text
}

fn escape(text: &str) -> String {
    text.replace('[', "[lb]")
}
