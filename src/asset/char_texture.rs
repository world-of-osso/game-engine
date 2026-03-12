//! Character body texture compositor.
//!
//! Composites BLP textures onto a body texture using ChrModelTextureLayer
//! and CharComponentTextureSections data.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;

use super::blp;
use super::casc_resolver;

/// A texture layer definition from ChrModelTextureLayer.csv.
#[derive(Debug, Clone)]
struct TextureLayer {
    layer: u32,
    blend_mode: u32,
    section_bitmask: i64,
    target_id: u16,
    layout_id: u32,
}

/// A texture section from CharComponentTextureSections.csv.
#[derive(Debug, Clone, Copy)]
struct TextureSection {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Texture layout dimensions from CharComponentTextureLayouts.csv.
#[derive(Debug, Clone, Copy)]
struct TextureLayout {
    width: u32,
    height: u32,
}

/// Loaded compositor data (parsed once at startup).
#[derive(Resource, Default, Debug)]
pub struct CharTextureData {
    /// ChrModelTextureTargetID -> Vec<TextureLayer>, sorted by layer order
    layers: Vec<TextureLayer>,
    /// (layout_id, section_type) -> TextureSection
    sections: HashMap<(u32, u32), TextureSection>,
    /// layout_id -> (width, height)
    layouts: HashMap<u32, TextureLayout>,
}

impl CharTextureData {
    pub fn load(data_dir: &Path) -> Self {
        match Self::try_load(data_dir) {
            Ok(d) => {
                info!(
                    "CharTextureData loaded: {} layers, {} sections",
                    d.layers.len(),
                    d.sections.len()
                );
                d
            }
            Err(e) => {
                warn!("Failed to load char texture data: {e}");
                Self::default()
            }
        }
    }

    fn try_load(data_dir: &Path) -> Result<Self, String> {
        let layers = parse_texture_layers(&data_dir.join("ChrModelTextureLayer.csv"))?;
        let sections = parse_texture_sections(&data_dir.join("CharComponentTextureSections.csv"))?;
        let layouts = parse_texture_layouts(&data_dir.join("CharComponentTextureLayouts.csv"))?;
        Ok(Self {
            layers,
            sections,
            layouts,
        })
    }

    /// Composite a character body texture from material assignments.
    /// `materials`: list of (ChrModelTextureTargetID, FDID) from customization choices.
    /// `layout_id`: CharComponentTextureLayoutID from ChrModel.
    pub fn composite(
        &self,
        materials: &[(u16, u32)],
        layout_id: u32,
    ) -> Option<(Vec<u8>, u32, u32)> {
        let layout = self.layouts.get(&layout_id)?;
        let (w, h) = (layout.width, layout.height);
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        // Group materials by target ID for lookup
        let mat_by_target: HashMap<u16, u32> = materials.iter().copied().collect();

        // Get layers for this layout, sorted by layer order
        let mut active_layers: Vec<_> = self
            .layers
            .iter()
            .filter(|l| l.layout_id == layout_id)
            .collect();
        active_layers.sort_by_key(|l| l.layer);

        for layer in &active_layers {
            let Some(&fdid) = mat_by_target.get(&layer.target_id) else {
                continue;
            };
            let texture_rgba = load_texture_rgba(fdid);
            let Some((tex_pixels, tex_w, tex_h)) = texture_rgba else {
                continue;
            };
            self.blit_layer(&mut pixels, w, &tex_pixels, tex_w, tex_h, layer, layout_id);
        }

        Some((pixels, w, h))
    }

    #[allow(clippy::too_many_arguments)]
    fn blit_layer(
        &self,
        pixels: &mut [u8],
        canvas_w: u32,
        tex: &[u8],
        tex_w: u32,
        tex_h: u32,
        layer: &TextureLayer,
        layout_id: u32,
    ) {
        if layer.section_bitmask == -1 {
            // Full texture: blit at (0,0), scaled to fill
            blit_scaled(pixels, canvas_w, tex, tex_w, tex_h, 0, 0, canvas_w, layer);
            return;
        }
        // Blit into each matching section
        for bit in 0..32u32 {
            if layer.section_bitmask & (1i64 << bit) == 0 {
                continue;
            }
            let Some(section) = self.sections.get(&(layout_id, bit)) else {
                continue;
            };
            blit_section(pixels, canvas_w, tex, tex_w, tex_h, section, layer);
        }
    }
}

fn load_texture_rgba(fdid: u32) -> Option<(Vec<u8>, u32, u32)> {
    let path = casc_resolver::ensure_texture(fdid)
        .unwrap_or_else(|| Path::new("data/textures").join(format!("{fdid}.blp")));
    blp::load_blp_rgba(&path).ok()
}

fn blit_section(
    pixels: &mut [u8],
    canvas_w: u32,
    tex: &[u8],
    tex_w: u32,
    tex_h: u32,
    section: &TextureSection,
    layer: &TextureLayer,
) {
    // Scale texture to section dimensions
    let (scaled, sw, sh) = scale_to(tex, tex_w, tex_h, section.width, section.height);
    let is_alpha_blend = layer.blend_mode == 15;
    for row in 0..sh.min(section.height) {
        for col in 0..sw.min(section.width) {
            let si = ((row * sw + col) * 4) as usize;
            let dx = section.x + col;
            let dy = section.y + row;
            let di = ((dy * canvas_w + dx) * 4) as usize;
            if di + 3 >= pixels.len() || si + 3 >= scaled.len() {
                continue;
            }
            blend_pixel(pixels, di, &scaled, si, is_alpha_blend);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_scaled(
    pixels: &mut [u8],
    canvas_w: u32,
    tex: &[u8],
    tex_w: u32,
    tex_h: u32,
    dx: u32,
    dy: u32,
    target_w: u32,
    layer: &TextureLayer,
) {
    let is_alpha_blend = layer.blend_mode == 15;
    for row in 0..tex_h {
        for col in 0..tex_w {
            let si = ((row * tex_w + col) * 4) as usize;
            let px = dx + col * target_w / tex_w;
            let py = dy + row;
            let di = ((py * canvas_w + px) * 4) as usize;
            if di + 3 >= pixels.len() || si + 3 >= tex.len() {
                continue;
            }
            blend_pixel(pixels, di, tex, si, is_alpha_blend);
        }
    }
}

fn blend_pixel(dst: &mut [u8], di: usize, src: &[u8], si: usize, alpha_blend: bool) {
    let alpha = src[si + 3] as u16;
    if alpha == 0 {
        return;
    }
    if !alpha_blend || alpha == 255 {
        dst[di] = src[si];
        dst[di + 1] = src[si + 1];
        dst[di + 2] = src[si + 2];
        dst[di + 3] = 255;
    } else {
        let inv = 255 - alpha;
        dst[di] = ((alpha * src[si] as u16 + inv * dst[di] as u16) / 255) as u8;
        dst[di + 1] = ((alpha * src[si + 1] as u16 + inv * dst[di + 1] as u16) / 255) as u8;
        dst[di + 2] = ((alpha * src[si + 2] as u16 + inv * dst[di + 2] as u16) / 255) as u8;
        dst[di + 3] = dst[di + 3].max(src[si + 3]);
    }
}

/// Nearest-neighbor scale to target dimensions.
fn scale_to(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (Vec<u8>, u32, u32) {
    if src_w == dst_w && src_h == dst_h {
        return (src.to_vec(), dst_w, dst_h);
    }
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    for y in 0..dst_h {
        for x in 0..dst_w {
            let sx = (x * src_w / dst_w).min(src_w - 1);
            let sy = (y * src_h / dst_h).min(src_h - 1);
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((y * dst_w + x) * 4) as usize;
            if si + 3 < src.len() && di + 3 < out.len() {
                out[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }
    (out, dst_w, dst_h)
}

// --- CSV parsing ---

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut lines = data.lines();
    let header_line = lines.next().ok_or("empty CSV")?;
    let headers = parse_csv_line(header_line);
    let rows: Vec<_> = lines.map(parse_csv_line).collect();
    Ok((headers, rows))
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == ',' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn parse_texture_layers(path: &Path) -> Result<Vec<TextureLayer>, String> {
    let (h, rows) = read_csv(path)?;
    let layer_col = col(&h, "Layer")?;
    let blend_col = col(&h, "BlendMode")?;
    let mask_col = col(&h, "TextureSectionTypeBitMask")?;
    let target_col = col(&h, "ChrModelTextureTargetID_0")?;
    let layout_col = col(&h, "CharComponentTextureLayoutsID")?;
    Ok(rows
        .iter()
        .map(|r| TextureLayer {
            layer: field_u32(r, layer_col),
            blend_mode: field_u32(r, blend_col),
            section_bitmask: field_i64(r, mask_col),
            target_id: field_u32(r, target_col) as u16,
            layout_id: field_u32(r, layout_col),
        })
        .collect())
}

fn parse_texture_sections(path: &Path) -> Result<HashMap<(u32, u32), TextureSection>, String> {
    let (h, rows) = read_csv(path)?;
    let layout_col = col(&h, "CharComponentTextureLayoutID")?;
    let section_col = col(&h, "SectionType")?;
    let x_col = col(&h, "X")?;
    let y_col = col(&h, "Y")?;
    let w_col = col(&h, "Width")?;
    let h_col = col(&h, "Height")?;
    Ok(rows
        .iter()
        .map(|r| {
            let key = (field_u32(r, layout_col), field_u32(r, section_col));
            let val = TextureSection {
                x: field_u32(r, x_col),
                y: field_u32(r, y_col),
                width: field_u32(r, w_col),
                height: field_u32(r, h_col),
            };
            (key, val)
        })
        .collect())
}

fn parse_texture_layouts(path: &Path) -> Result<HashMap<u32, TextureLayout>, String> {
    let (h, rows) = read_csv(path)?;
    let id_col = col(&h, "ID")?;
    let w_col = col(&h, "Width")?;
    let h_col = col(&h, "Height")?;
    Ok(rows
        .iter()
        .map(|r| {
            (
                field_u32(r, id_col),
                TextureLayout {
                    width: field_u32(r, w_col),
                    height: field_u32(r, h_col),
                },
            )
        })
        .collect())
}

fn col(headers: &[String], name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| format!("Column '{name}' not found"))
}

fn field_u32(row: &[String], col: usize) -> u32 {
    row.get(col)
        .and_then(|s| {
            s.parse::<u32>()
                .ok()
                .or_else(|| s.parse::<i32>().ok().map(|v| v as u32))
        })
        .unwrap_or(0)
}

fn field_i64(row: &[String], col: usize) -> i64 {
    row.get(col)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}
