//! Geometry from the local Retail XML and SpaceToFitLayoutFrame.lua.

pub(super) const NAV_WIDTH: f32 = 250.0;
pub(super) const NAV_HEIGHT: f32 = 66.0;
pub(super) const NAV_SIDE: f32 = 46.0;
pub(super) const NAV_BOTTOM: f32 = 28.0;
pub(super) const OPTION_WIDTH: f32 = 223.0;
pub(super) const OPTION_HEIGHT: f32 = 38.0;
pub(super) const OPTION_RIGHT: f32 = 33.0;
pub(super) const CATEGORY_HEIGHT: f32 = 105.0;
pub(super) const CATEGORY_WIDTH: f32 = 104.0;
pub(super) const CATEGORY_TOP: f32 = 166.0;
pub(super) const CHOICE_WIDTH: f32 = 172.0;
pub(super) const CHOICE_HEIGHT: f32 = 20.0;

pub(super) fn fit_spacing(space: f32, count: usize, size: f32, base: f32) -> f32 {
    if count == 0 {
        return base;
    }
    let left = space - count as f32 * size;
    if left < base * count as f32 {
        (left / count as f32).floor()
    } else {
        base
    }
}

pub(super) struct OptionsLayout {
    pub top: f32,
    pub step: f32,
    pub height: f32,
}

pub(super) fn options_layout(height: u32, count: usize) -> OptionsLayout {
    let bottom = height as f32 - NAV_HEIGHT - NAV_BOTTOM - 20.0;
    let spacing = fit_spacing(bottom - 297.0, count, OPTION_HEIGHT, 32.0);
    let top = if spacing < 32.0 { 267.0 } else { 297.0 };
    let spacing = fit_spacing(bottom - top, count, OPTION_HEIGHT, 32.0);
    let step = OPTION_HEIGHT + spacing;
    OptionsLayout {
        top,
        step,
        height: count as f32 * OPTION_HEIGHT + count.saturating_sub(1) as f32 * spacing,
    }
}

pub(super) struct ClassLayout {
    pub columns: usize,
    pub size: f32,
    pub gap: f32,
    pub width: f32,
    pub height: f32,
}

pub(super) fn class_layout(viewport_width: u32, count: usize) -> ClassLayout {
    let available = (viewport_width as f32 - 2.0 * (NAV_WIDTH + NAV_SIDE + 10.0)).max(1.0);
    let total = count as f32 * 67.0 + count.saturating_sub(1) as f32 * 15.0;
    let rows = (total / available).ceil().clamp(1.0, 2.0) as usize;
    let columns = count.div_ceil(rows).max(1);
    let row_width = columns as f32 * 67.0 + columns.saturating_sub(1) as f32 * 15.0;
    let scale = if row_width > available {
        1.0 / (1.0 + (row_width - available) / columns as f32 / 67.0)
    } else {
        1.0
    };
    ClassLayout {
        columns,
        size: 67.0 * scale,
        gap: 15.0 * scale,
        width: row_width * scale,
        height: (rows as f32 * 67.0 + rows.saturating_sub(1) as f32 * 47.0) * scale,
    }
}

pub(super) struct PopupLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rows: usize,
}

pub(super) fn popup_layout(viewport: [u32; 2], count: usize, anchor_bottom: f32) -> PopupLayout {
    // Retail's vertical auto-grid reserves 100px before introducing another column.
    let rows_available = ((viewport[1] as f32 - 100.0) / CHOICE_HEIGHT)
        .floor()
        .max(1.0) as usize;
    let columns = count.div_ceil(rows_available).max(1);
    let rows = count.div_ceil(columns).max(1);
    let width = columns as f32 * CHOICE_WIDTH;
    let height = rows as f32 * CHOICE_HEIGHT;
    let right = viewport[0] as f32 - OPTION_RIGHT - 36.5;
    PopupLayout {
        x: (right - width).max(30.0),
        y: anchor_bottom
            .min(viewport[1] as f32 - height - 30.0)
            .max(30.0),
        width,
        height,
        rows,
    }
}
