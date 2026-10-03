//! Original bag-column layout in logical UI units, independent of the renderer.

const CONTAINER_RIGHT: f32 = 16.0;
/// Clears the micro menu and bags bar at the bottom-right.
const CONTAINER_BOTTOM: f32 = 96.0;
const CONTAINER_GAP: f32 = 8.0;
const WINDOW_TOP: f32 = 104.0;

/// Bags, supplied in ascending ID order, stack upward then leftward in columns.
/// Each input is `(bag_id, [width, height])`; outputs are clamped top-left positions.
pub fn container_positions(bags: &[(usize, [f32; 2])], screen: [f32; 2]) -> Vec<(usize, [f32; 2])> {
    let mut placed = Vec::with_capacity(bags.len());
    let mut column_right = screen[0] - CONTAINER_RIGHT;
    let mut column_width: f32 = 0.0;
    let mut bottom = screen[1] - CONTAINER_BOTTOM;
    for &(id, [width, height]) in bags {
        if bottom - height < WINDOW_TOP && column_width > 0.0 {
            column_right -= column_width + CONTAINER_GAP;
            column_width = 0.0;
            bottom = screen[1] - CONTAINER_BOTTOM;
        }
        let position = [
            (column_right - width).clamp(0.0, (screen[0] - width).max(0.0)),
            (bottom - height).clamp(0.0, (screen[1] - height).max(0.0)),
        ];
        placed.push((id, position));
        bottom -= height + CONTAINER_GAP;
        column_width = column_width.max(width);
    }
    placed
}
