use crate::ui::anchor::{Anchor, AnchorPoint, anchor_position, frame_position_from_anchor};

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Return the fractional (0..1) position along the X and Y axes for an anchor point.
fn point_to_edge_offsets(point: AnchorPoint) -> (f32, f32) {
    match point {
        AnchorPoint::TopLeft => (0.0, 0.0),
        AnchorPoint::Top => (0.5, 0.0),
        AnchorPoint::TopRight => (1.0, 0.0),
        AnchorPoint::Left => (0.0, 0.5),
        AnchorPoint::Center => (0.5, 0.5),
        AnchorPoint::Right => (1.0, 0.5),
        AnchorPoint::BottomLeft => (0.0, 1.0),
        AnchorPoint::Bottom => (0.5, 1.0),
        AnchorPoint::BottomRight => (1.0, 1.0),
    }
}

fn resolve_target(anchor: &Anchor, parent: &LayoutRect) -> (f32, f32) {
    let (ax, ay) = anchor_position(
        anchor.relative_point,
        parent.x,
        parent.y,
        parent.width,
        parent.height,
    );
    (ax + anchor.x_offset, ay - anchor.y_offset)
}

/// Resolve a frame's layout rectangle from its anchors, explicit size, and parent rect.
///
/// Follows WoW anchor semantics: no anchors places at parent top-left, one anchor
/// positions the frame, two anchors can stretch the frame along axes where the
/// anchor points differ.
pub fn resolve_anchors(
    anchors: &[Anchor],
    width: f32,
    height: f32,
    parent_rect: &LayoutRect,
) -> LayoutRect {
    match anchors.len() {
        0 => LayoutRect {
            x: parent_rect.x,
            y: parent_rect.y,
            width,
            height,
        },
        1 => resolve_single_anchor(&anchors[0], width, height, parent_rect),
        _ => resolve_two_anchors(&anchors[0], &anchors[1], width, height, parent_rect),
    }
}

fn resolve_single_anchor(
    anchor: &Anchor,
    width: f32,
    height: f32,
    parent: &LayoutRect,
) -> LayoutRect {
    let (tx, ty) = resolve_target(anchor, parent);
    let (fx, fy) = frame_position_from_anchor(anchor.point, tx, ty, width, height);
    LayoutRect {
        x: fx,
        y: fy,
        width,
        height,
    }
}

fn resolve_two_anchors(
    a: &Anchor,
    b: &Anchor,
    width: f32,
    height: f32,
    parent: &LayoutRect,
) -> LayoutRect {
    let (t1x, t1y) = resolve_target(a, parent);
    let (t2x, t2y) = resolve_target(b, parent);

    let (frac1x, frac1y) = point_to_edge_offsets(a.point);
    let (frac2x, frac2y) = point_to_edge_offsets(b.point);

    let (final_x, final_w) = if (frac1x - frac2x).abs() > f32::EPSILON {
        stretch_axis(t1x, frac1x, t2x, frac2x)
    } else {
        let fx = t1x - frac1x * width;
        (fx, width)
    };

    let (final_y, final_h) = if (frac1y - frac2y).abs() > f32::EPSILON {
        stretch_axis(t1y, frac1y, t2y, frac2y)
    } else {
        let fy = t1y - frac1y * height;
        (fy, height)
    };

    LayoutRect {
        x: final_x,
        y: final_y,
        width: final_w,
        height: final_h,
    }
}

/// Given two target positions along an axis and their fractional offsets,
/// compute the origin and size of the frame along that axis.
///
/// frac represents where the anchor sits within the frame (0=start, 1=end).
/// target = origin + frac * size, so with two equations we solve for origin and size.
fn stretch_axis(t1: f32, frac1: f32, t2: f32, frac2: f32) -> (f32, f32) {
    let size = (t2 - t1) / (frac2 - frac1);
    let origin = t1 - frac1 * size;
    (origin, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent() -> LayoutRect {
        LayoutRect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }

    fn anchor(
        point: AnchorPoint,
        relative_point: AnchorPoint,
        x_offset: f32,
        y_offset: f32,
    ) -> Anchor {
        Anchor {
            point,
            relative_to: None,
            relative_point,
            x_offset,
            y_offset,
        }
    }

    fn approx_eq(a: &LayoutRect, b: &LayoutRect) -> bool {
        (a.x - b.x).abs() < 0.01
            && (a.y - b.y).abs() < 0.01
            && (a.width - b.width).abs() < 0.01
            && (a.height - b.height).abs() < 0.01
    }

    #[test]
    fn no_anchors_at_parent_topleft() {
        let result = resolve_anchors(&[], 100.0, 50.0, &parent());
        assert_eq!(
            result,
            LayoutRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            }
        );
    }

    #[test]
    fn single_center_to_center() {
        let anchors = [anchor(AnchorPoint::Center, AnchorPoint::Center, 0.0, 0.0)];
        let result = resolve_anchors(&anchors, 200.0, 100.0, &parent());
        // Frame centered: (800-200)/2=300, (600-100)/2=250
        assert_eq!(
            result,
            LayoutRect {
                x: 300.0,
                y: 250.0,
                width: 200.0,
                height: 100.0,
            }
        );
    }

    #[test]
    fn single_topleft_with_offset() {
        // x_offset=10, y_offset=-5 means target_y = parent_y - (-5) = 5
        let anchors = [anchor(
            AnchorPoint::TopLeft,
            AnchorPoint::TopLeft,
            10.0,
            -5.0,
        )];
        let result = resolve_anchors(&anchors, 100.0, 50.0, &parent());
        assert_eq!(
            result,
            LayoutRect {
                x: 10.0,
                y: 5.0,
                width: 100.0,
                height: 50.0,
            }
        );
    }

    #[test]
    fn two_anchors_horizontal_stretch() {
        // LEFT-to-LEFT with 20px inset, RIGHT-to-RIGHT with -20px inset
        let anchors = [
            anchor(AnchorPoint::Left, AnchorPoint::Left, 20.0, 0.0),
            anchor(AnchorPoint::Right, AnchorPoint::Right, -20.0, 0.0),
        ];
        let result = resolve_anchors(&anchors, 100.0, 50.0, &parent());
        // Horizontal: LEFT target = (0+20, 300), RIGHT target = (800-20, 300)
        // frac1x=0, frac2x=1 → size = (780-20)/(1-0) = 760, origin = 20
        // Vertical: same frac (0.5, 0.5) → use first anchor, fy = 300 - 0.5*50 = 275
        let expected = LayoutRect {
            x: 20.0,
            y: 275.0,
            width: 760.0,
            height: 50.0,
        };
        assert!(
            approx_eq(&result, &expected),
            "got {result:?}, expected {expected:?}"
        );
    }

    #[test]
    fn two_anchors_vertical_stretch() {
        // TOP-to-TOP and BOTTOM-to-BOTTOM with 10px insets
        let anchors = [
            anchor(AnchorPoint::Top, AnchorPoint::Top, 0.0, -10.0),
            anchor(AnchorPoint::Bottom, AnchorPoint::Bottom, 0.0, 10.0),
        ];
        let result = resolve_anchors(&anchors, 200.0, 100.0, &parent());
        // TOP target = (400, 0 - (-10)) = (400, 10)
        // BOTTOM target = (400, 600 - 10) = (400, 590)
        // X fracs both 0.5 → use first anchor: fx = 400 - 0.5*200 = 300
        // Y fracs 0.0 and 1.0 → size = (590-10)/(1-0) = 580, origin = 10
        let expected = LayoutRect {
            x: 300.0,
            y: 10.0,
            width: 200.0,
            height: 580.0,
        };
        assert!(
            approx_eq(&result, &expected),
            "got {result:?}, expected {expected:?}"
        );
    }

    #[test]
    fn single_bottomright_anchor() {
        let anchors = [anchor(
            AnchorPoint::BottomRight,
            AnchorPoint::BottomRight,
            0.0,
            0.0,
        )];
        let result = resolve_anchors(&anchors, 100.0, 50.0, &parent());
        // Target = parent bottom-right = (800, 600)
        // frame_position_from_anchor(BottomRight, 800, 600, 100, 50) = (700, 550)
        assert_eq!(
            result,
            LayoutRect {
                x: 700.0,
                y: 550.0,
                width: 100.0,
                height: 50.0,
            }
        );
    }
}
