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
    let row = ITEM_H + Y_PADDING;
    let mut views: Vec<Vec<Placement>> = vec![Vec::new()];
    let mut y = 0.0;
    for (group_index, group) in groups.iter().enumerate() {
        if group.items.is_empty() {
            continue;
        }
        if y > 0.0 {
            y += SPACER;
        }
        if y + HEADER_H + ITEM_H > VIEW_H {
            views.push(Vec::new());
            y = 0.0;
        }
        views.last_mut().expect("a view").push(Placement::Header {
            group: group_index,
            y,
        });
        y += HEADER_H + Y_PADDING;
        let mut placed = 0;
        while placed < group.items.len() {
            let remaining = group.items.len() - placed;
            let rows_fit = ((VIEW_H - y + Y_PADDING) / row).floor().max(0.0) as usize;
            if rows_fit == 0 {
                views.push(Vec::new());
                y = 0.0;
                continue;
            }
            let rows = remaining.div_ceil(COLUMNS).min(rows_fit);
            let count = remaining.min(rows * COLUMNS);
            let view = views.last_mut().expect("a view");
            for offset in 0..count {
                let (column, row_index) = (offset / rows, offset % rows);
                view.push(Placement::Item {
                    group: group_index,
                    item: placed + offset,
                    x: column as f32 * (ITEM_W + X_PADDING),
                    y: y + row_index as f32 * row,
                });
            }
            placed += count;
            y += rows as f32 * row;
            if placed < group.items.len() {
                views.push(Vec::new());
                y = 0.0;
            }
        }
    }
    views
}
