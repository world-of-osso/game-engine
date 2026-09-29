//! Legacy Options placement policy in logical viewport coordinates.

use godot::prelude::*;

const OPTIONS_W: f32 = 860.0;
const OPTIONS_H: f32 = 580.0;

pub(crate) struct OptionsDrag {
    cursor_offset: Vector2,
}

impl OptionsDrag {
    pub(crate) fn begin(cursor: Vector2, position: [f32; 2], viewport: Vector2) -> Self {
        Self {
            cursor_offset: cursor - top_left(position, viewport),
        }
    }

    pub(crate) fn position(&self, cursor: Vector2, viewport: Vector2) -> [f32; 2] {
        clamp_top_left(cursor - self.cursor_offset, viewport)
    }
}

pub(crate) fn initial_position(
    offset: Option<[f32; 2]>,
    legacy_top_left: Option<[f32; 2]>,
    viewport: Vector2,
) -> [f32; 2] {
    let position = offset
        .or_else(|| legacy_top_left.map(|[x, y]| top_left_to_offset(Vector2::new(x, y), viewport)));
    let [x, y] = position.unwrap_or([0.0, 0.0]);
    clamp_top_left(top_left([x, y], viewport), viewport)
}

fn top_left(offset: [f32; 2], viewport: Vector2) -> Vector2 {
    Vector2::new(
        viewport.x * 0.5 + offset[0] - OPTIONS_W * 0.5,
        viewport.y * 0.5 - offset[1] - OPTIONS_H * 0.5,
    )
}

fn clamp_top_left(position: Vector2, viewport: Vector2) -> [f32; 2] {
    top_left_to_offset(
        Vector2::new(
            position.x.clamp(0.0, (viewport.x - OPTIONS_W).max(0.0)),
            position.y.clamp(0.0, (viewport.y - OPTIONS_H).max(0.0)),
        ),
        viewport,
    )
}

fn top_left_to_offset(position: Vector2, viewport: Vector2) -> [f32; 2] {
    [
        position.x - viewport.x * 0.5 + OPTIONS_W * 0.5,
        viewport.y * 0.5 - position.y - OPTIONS_H * 0.5,
    ]
}
