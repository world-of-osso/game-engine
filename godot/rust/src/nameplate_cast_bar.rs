//! Nameplate cast bar nodes: Retail's `ui-castingbar-background`/`-filling-standard`
//! atlases under the health bar (docs/specs/nameplate-style.md), drawn from the active
//! skin's sheet, with the approved B soft leading-edge glow, Retail's interrupt shield,
//! spell icon and spell name row, driven by `nameplate_casts::CastBar`. The Thick bar
//! follows the user's reference of 2026-10-04: as wide as the health frame, the icon at
//! its left end and the name inside.

use std::collections::HashMap;
use std::path::Path;

use game_engine_core::nameplate_style_data::{NameplateBarThickness, NameplateStyle};
use game_engine_core::warband_scene_data::{AtlasArt, read_atlas_art};
use godot::{
    classes::{
        AtlasTexture, CanvasItemMaterial, Control, FontFile, ImageTexture, RichTextLabel,
        Texture2D, TextureRect, canvas_item_material::BlendMode, control::MouseFilter,
        texture_rect::ExpandMode, texture_rect::StretchMode,
    },
    prelude::*,
};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};

use crate::assets::material::{load_texture, shared_texture, texture_from_rgba};
use crate::nameplate_casts::{BarType, CastBar, Spark};

/// Bevy `NAMEPLATE_SCALE`: reference skins are unscaled screenshot pixels.
const NAMEPLATE_SCALE: f32 = 0.5;
/// Bevy Thin label row: its centre this far below the cast body's bottom.
const THIN_LABEL_DROP: f32 = 14.0 * NAMEPLATE_SCALE;
/// `NamePlateSetupOptions` at scale 1: `castIconWidth/Height`, `castBarShieldWidth/Height`.
const ICON_SIZE: f32 = 12.0;
const SHIELD_SIZE: Vector2 = Vector2::new(10.0, 12.0);
/// B's sigma ≈ 0.9 × 2.25 UI units. OBJFX_Glow's 64px radial alpha has a fitted
/// sigma ≈ 10.7px, so this extent preserves the approved narrow, soft falloff.
const SPARK_SIZE: Vector2 = Vector2::new(5.4, 13.5);
const SPARK_GOLD: [f32; 3] = [1.0, 0.92, 0.65];
/// `Text` anchors LEFT to `Icon` RIGHT, 2 px over.
const ICON_TEXT_GAP: f32 = 2.0;
/// `NamePlateCastingBarMixin` art (Blizzard_NamePlateCastingBar.lua:88,95,101) and the
/// standard filling (CastingBarFrame.lua:38): `uicastingbar` members under Modern,
/// Forever's set-1 `uicastingbarc60` members of the same names.
const BACKGROUND: &str = "ui-castingbar-background";
const FILL: &str = "ui-castingbar-filling-standard";
/// Blizzard's soft white radial alpha (FDID 959719), shared by both skins.
const GLOW: &str = "objfx_glow";
const SHIELD: &str = "nameplates-InterruptShield";
/// `CASTBAR_CLASSIC_RED`, the interrupted and failed fill.
const INTERRUPTED_COLOR: [f32; 3] = [1.0, 0.0, 0.0];

/// The sheet and crop atlas `name` draws under `skin`, from the `UiTextureAtlas*` tables.
pub(crate) fn skin_art(name: &str, skin: ActiveSkin) -> Result<AtlasArt, String> {
    let region = resolve_region(name, skin)
        .ok_or_else(|| format!("no UiTextureAtlas member for atlas {name} under {skin:?}"))?;
    let AtlasSource::FileDataId(fdid) = region.source else {
        return Err(format!("atlas {name} is not a UiTextureAtlas member"));
    };
    Ok(AtlasArt {
        fdid,
        tex_coords: [region.left, region.right, region.top, region.bottom],
    })
}

/// The cast bar's atlas crops under one skin.
#[derive(Debug, PartialEq)]
pub(crate) struct CastCrops {
    pub background: AtlasArt,
    pub fill: AtlasArt,
    pub shield: AtlasArt,
}

pub(crate) fn cast_crops(skin: ActiveSkin) -> Result<CastCrops, String> {
    Ok(CastCrops {
        background: skin_art(BACKGROUND, skin)?,
        fill: skin_art(FILL, skin)?,
        shield: skin_art(SHIELD, skin)?,
    })
}

/// Cast bar part rectangles relative to the plate anchor (the health body's centre),
/// y down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CastLayout {
    pub background: Rect2,
    pub fill: Rect2,
    pub spark: Rect2,
    pub icon: Rect2,
    pub shield: Rect2,
    /// Left middle of the spell name.
    pub text_left: Vector2,
}

/// `plate` is the left and right edge of the health row (`PlateLayout::span`): the cast
/// row, icon included, sits on the same span, wider or narrower by what the style's cast
/// width differs from its health width (nothing by default). Its top touches the
/// rendered health row's `health_bottom`.
pub(crate) fn cast_layout(
    style: &NameplateStyle,
    fraction: f32,
    plate: (f32, f32),
    health_bottom: f32,
) -> CastLayout {
    let (left, right) = plate;
    let body = Vector2::new(
        right - left + style.cast_width - style.health_width,
        style.cast_height,
    );
    let center = Vector2::new((left + right) / 2.0, health_bottom + body.y / 2.0);
    let left = center.x - body.x / 2.0;
    let background = Rect2::new(center - body / 2.0, body);
    let fill = Rect2::new(background.position, Vector2::new(body.x * fraction, body.y));
    let spark_size = Vector2::new(SPARK_SIZE.x, body.y.min(SPARK_SIZE.y));
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
    background: Gd<AtlasTexture>,
    /// The fill's sheet desaturated, so the style's cast colour tints its fill crop.
    fill_sheet: Gd<ImageTexture>,
    /// The fill crop on `fill_sheet`, in pixels.
    fill_rect: Rect2,
    pip: Gd<AtlasTexture>,
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
    Ok(atlas_region(&texture, pixel_rect(art, &texture)))
}

/// `art`'s crop of `texture`, in pixels.
fn pixel_rect(art: &AtlasArt, texture: &Gd<ImageTexture>) -> Rect2 {
    let (width, height) = (texture.get_width() as f32, texture.get_height() as f32);
    let [left, right, top, bottom] = art.tex_coords;
    Rect2::new(
        Vector2::new(left * width, top * height),
        Vector2::new((right - left) * width, (bottom - top) * height),
    )
}

impl CastArt {
    /// The art under atlas skin `active`.
    pub fn load(data_root: &Path, active: ActiveSkin) -> Result<Self, String> {
        let crops = cast_crops(active)?;
        ensure_texture(data_root, crops.fill.fdid)?;
        let textures = data_root.join("textures");
        let mut missing = PackedInt32Array::new();
        let (mut pixels, width, height) =
            load_texture(crops.fill.fdid, &textures, &mut missing)?
                .ok_or_else(|| format!("missing nameplate cast texture {}", crops.fill.fdid))?;
        // Bevy `desaturated_copy`: each pixel's HSV value.
        for pixel in pixels.chunks_exact_mut(4) {
            let value = pixel[0].max(pixel[1]).max(pixel[2]);
            pixel[..3].fill(value);
        }
        let fill_sheet = texture_from_rgba(&pixels, width, height)?;
        let atlases = read_atlas_art(data_root, &[GLOW])?;
        Ok(Self {
            background: atlas_art(&crops.background, data_root)?,
            fill_rect: pixel_rect(&crops.fill, &fill_sheet),
            fill_sheet,
            pip: atlas_art(&atlases[GLOW], data_root)?,
            shield: atlas_art(&crops.shield, data_root)?,
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
    background: Gd<TextureRect>,
    fill: Gd<TextureRect>,
    fill_region: Gd<AtlasTexture>,
    spark: Gd<TextureRect>,
    spark_region: Gd<AtlasTexture>,
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
        let fill_region = atlas_region(&art.fill_sheet, art.fill_rect);
        let mut fill = texture_rect("Fill");
        fill.set_texture(&fill_region);
        let mut spark = texture_rect("Spark");
        let mut spark_region = AtlasTexture::new_gd();
        spark_region.set_atlas(&art.pip);
        spark.set_texture(&spark_region);
        // Keep Retail's additive spark blend (Blizzard_NamePlateCastingBar.xml:52).
        // The radial alpha brightens the track without an opaque core.
        let mut additive = CanvasItemMaterial::new_gd();
        additive.set_blend_mode(BlendMode::ADD);
        spark.set_material(&additive);
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
        // Outlined like the plate's name (the user's reference of 2026-10-04).
        text.add_theme_color_override("font_outline_color", Color::BLACK);
        text.add_theme_constant_override("outline_size", 2);
        // Fill above its background, spark and icon row on top.
        for node in [
            background.clone().upcast::<Control>(),
            fill.clone().upcast(),
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
            background,
            fill,
            fill_region,
            spark,
            spark_region,
            icon,
            shield,
            text,
        }
    }

    /// Show `bar` (or nothing) under a plate drawn in `style` whose health row spans
    /// `plate`.
    pub fn apply(
        &mut self,
        bar: Option<&CastBar>,
        icon: Option<&Gd<Texture2D>>,
        (style, plate, health_bottom): (&NameplateStyle, (f32, f32), f32),
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
        let layout = cast_layout(style, fraction, plate, health_bottom);
        place(&mut self.background, layout.background);
        place(&mut self.fill, layout.fill);
        self.fill.set_visible(fraction > 0.0);
        // Bevy `cast_fill_crop`: the fill shows the left `fraction` of the crop.
        let Rect2 { position, size } = art.fill_rect;
        self.fill_region.set_region(Rect2::new(
            position,
            Vector2::new(size.x * fraction, size.y),
        ));
        self.fill.set_self_modulate(fill_color(bar, style));
        place(&mut self.spark, layout.spark);
        self.spark.set_visible(bar.spark.is_some());
        // Crop the shared glow vertically instead of squashing it on Thin bars.
        let source_size = Vector2::new(art.pip.get_width() as f32, art.pip.get_height() as f32);
        let crop_height = source_size.y * layout.spark.size.y / SPARK_SIZE.y;
        self.spark_region.set_region(Rect2::new(
            Vector2::new(0.0, (source_size.y - crop_height) / 2.0),
            Vector2::new(source_size.x, crop_height),
        ));
        if let Some(spark) = bar.spark {
            self.spark.set_self_modulate(spark_color(spark));
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

pub(crate) fn place(rect: &mut Gd<TextureRect>, at: Rect2) {
    rect.set_position(at.position);
    rect.set_size(at.size);
}

/// `ShowSpark` retains its normal/interrupted choice; B changes only the artwork.
fn spark_color(spark: Spark) -> Color {
    let [r, g, b] = match spark {
        Spark::Pip => SPARK_GOLD,
        Spark::PipRed => INTERRUPTED_COLOR,
    };
    Color::from_rgb(r, g, b)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nameplate_casts::PlateCasts;
    use shared::{casting::CastState, spell_data::CastFailReason};

    #[test]
    fn nameplate_interrupted_and_failed_glows_tint_red_instead_of_gold() {
        for reason in [CastFailReason::Interrupted, CastFailReason::OutOfRange] {
            let mut casts = PlateCasts::default();
            let mut cast = CastState::normal(133, 0, 2.0, true);
            cast.elapsed = 1.0;
            casts.observe(42, Some(&cast));
            let normal = spark_color(casts.get(42).unwrap().spark.unwrap());
            assert!(normal.r >= normal.g && normal.g > normal.b);
            casts.observe(42, None);
            casts.spell_failure(42, 133, reason, None);
            let interrupted = spark_color(casts.get(42).unwrap().spark.unwrap());
            assert!(interrupted.r > 0.9);
            assert!(interrupted.g < 0.05 && interrupted.b < 0.05);
        }
    }
}
