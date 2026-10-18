//! Native FontString adapter for the toolkit's historical raster allocation.
//! Private per-size resources keep button/edit fonts and shared assets unchanged.
use std::collections::HashMap;

use godot::classes::FontFile;
use godot::prelude::*;
use skrifa::raw::TableProvider;
use ui_toolkit::font_line_metrics::{FontLineMetrics, FontLineRequest, resolve_font_lines};
use ui_toolkit::widgets::font_string::{FontStringData, GameFont};

#[derive(Default)]
pub(super) struct FontLineCache {
    fonts: HashMap<(GameFont, u32, u32), (Gd<FontFile>, FontLineRequest)>,
}

impl FontLineCache {
    pub fn resolve(
        &mut self,
        source: &Gd<FontFile>,
        data: &FontStringData,
        pixels_per_ui: f32,
        line_count: usize,
    ) -> Result<(Gd<FontFile>, FontLineMetrics), String> {
        let key = (data.font, data.font_size.to_bits(), pixels_per_ui.to_bits());
        if let std::collections::hash_map::Entry::Vacant(entry) = self.fonts.entry(key) {
            entry.insert(create_raster_font(source, data, pixels_per_ui)?);
        }
        let (resource, request) = &self.fonts[&key];
        let metrics = resolve_font_lines(FontLineRequest {
            spacing: data.line_spacing.unwrap_or(0) as f32,
            line_count,
            ..*request
        })?;
        Ok((resource.clone(), metrics))
    }
}

fn create_raster_font(
    source: &Gd<FontFile>,
    data: &FontStringData,
    pixels_per_ui: f32,
) -> Result<(Gd<FontFile>, FontLineRequest), String> {
    let nominal_height = data.font_size;
    let bytes = source.get_data();
    let (face_ascent, face_descent) = read_face_metrics(bytes.as_slice(), data.font)?;
    let request = FontLineRequest {
        nominal_height,
        pixels_per_ui,
        spacing: 0.0,
        line_count: 0,
        face_ascent,
        face_descent,
    };
    let metrics = resolve_font_lines(request)?;
    let mut resource = FontFile::new_gd();
    resource.set_data(&bytes);
    // Query first to create the ordinary cache, then replace only its metrics.
    resource
        .get_ascent_ex()
        .font_size(nominal_height as i32)
        .done();
    resource.set_cache_ascent(0, nominal_height as i32, metrics.baseline_ui);
    resource.set_cache_descent(
        0,
        nominal_height as i32,
        metrics.line_height_ui - metrics.baseline_ui,
    );
    Ok((resource, request))
}

fn read_face_metrics(bytes: &[u8], font: GameFont) -> Result<(f32, f32), String> {
    let face = skrifa::FontRef::new(bytes)
        .map_err(|error| format!("Cannot read FontString face {font:?}: {error}"))?;
    let hhea = face
        .hhea()
        .map_err(|error| format!("Cannot read FontString ascender {font:?}: {error}"))?;
    Ok((f32::from(hhea.ascender()), f32::from(hhea.descender())))
}
