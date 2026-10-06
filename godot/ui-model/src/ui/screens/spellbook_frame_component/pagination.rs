use super::*;

/// A header or item placed in a page view, in unscaled view coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    Header {
        group: usize,
        y: f32,
    },
    Item {
        group: usize,
        item: usize,
        x: f32,
        y: f32,
    },
}

/// `PagedCondensedVerticalGridContentFrameMixin`: each group's header takes a full row,
/// its items fill `ceil(n / 3)` rows column-first; a view that runs out of height
/// continues the items (without the header) in the next view. Groups after the first in a
/// view start after a 20 px spacer.
pub fn paginate(groups: &[SpellbookGroup]) -> Vec<Vec<Placement>> {
    let mut views = vec![Vec::new()];
    let mut y = 0.0;
    for (group_index, group) in groups.iter().enumerate() {
        if group.items.is_empty() {
            continue;
        }
        place_header(&mut views, &mut y, group_index);
        place_group_items(&mut views, &mut y, group_index, group.items.len());
    }
    views
}

fn start_view(views: &mut Vec<Vec<Placement>>, y: &mut f32) {
    views.push(Vec::new());
    *y = 0.0;
}

fn place_header(views: &mut Vec<Vec<Placement>>, y: &mut f32, group: usize) {
    if *y > 0.0 {
        *y += SPACER;
    }
    if *y + HEADER_H + ITEM_H > VIEW_H {
        start_view(views, y);
    }
    views
        .last_mut()
        .expect("a view")
        .push(Placement::Header { group, y: *y });
    *y += HEADER_H + Y_PADDING;
}

fn place_group_items(views: &mut Vec<Vec<Placement>>, y: &mut f32, group: usize, total: usize) {
    let row = ITEM_H + Y_PADDING;
    let mut placed = 0;
    while placed < total {
        let remaining = total - placed;
        let rows_fit = ((VIEW_H - *y + Y_PADDING) / row).floor().max(0.0) as usize;
        if rows_fit == 0 {
            start_view(views, y);
            continue;
        }
        let rows = remaining.div_ceil(COLUMNS).min(rows_fit);
        let count = remaining.min(rows * COLUMNS);
        let view = views.last_mut().expect("a view");
        place_columns(view, group, placed..placed + count, rows, *y);
        placed += count;
        *y += rows as f32 * row;
        if placed < total {
            start_view(views, y);
        }
    }
}

fn place_columns(
    view: &mut Vec<Placement>,
    group: usize,
    items: std::ops::Range<usize>,
    rows: usize,
    y: f32,
) {
    for (offset, item) in items.enumerate() {
        let (column, row_index) = (offset / rows, offset % rows);
        view.push(Placement::Item {
            group,
            item,
            x: column as f32 * (ITEM_W + X_PADDING),
            y: y + row_index as f32 * (ITEM_H + Y_PADDING),
        });
    }
}
