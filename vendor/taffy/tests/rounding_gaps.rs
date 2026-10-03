//! Rounded layout must not open gaps between abutting boxes inside a parent that sits
//! at a fractional position. Values are game-engine's auction house Buy tab at UI scale
//! 2/3 in physical pixels: tab at x 386.333, left cap (-1, 35 wide), middle (34, 7 wide)
//! and right cap (41, 37 wide) in UI units.

use taffy::prelude::*;

fn absolute(left: f32, width: f32) -> Style {
    Style {
        position: Position::Absolute,
        inset: Rect {
            left: LengthPercentageAuto::length(left),
            right: LengthPercentageAuto::auto(),
            top: LengthPercentageAuto::length(0.0),
            bottom: LengthPercentageAuto::auto(),
        },
        size: Size { width: Dimension::length(width), height: Dimension::length(28.0) },
        ..Style::default()
    }
}

#[test]
fn abutting_children_of_fractional_parent_share_rounded_edges() {
    let mut tree: TaffyTree<()> = TaffyTree::new();
    let left = tree.new_leaf(absolute(-2.0 / 3.0, 35.0 * 2.0 / 3.0)).unwrap();
    let middle = tree.new_leaf(absolute(34.0 * 2.0 / 3.0, 7.0 * 2.0 / 3.0)).unwrap();
    let right = tree.new_leaf(absolute(41.0 * 2.0 / 3.0, 37.0 * 2.0 / 3.0)).unwrap();
    let tab = tree.new_with_children(absolute(386.0 + 1.0 / 3.0, 70.0 * 2.0 / 3.0), &[left, middle, right]).unwrap();
    let root = tree
        .new_with_children(
            Style { size: Size { width: Dimension::length(1280.0), height: Dimension::length(720.0) }, ..Style::default() },
            &[tab],
        )
        .unwrap();
    tree.compute_layout(root, Size::MAX_CONTENT).unwrap();

    let edges = |node| {
        let layout = tree.layout(node).unwrap();
        (layout.location.x, layout.location.x + layout.size.width)
    };
    let (left_start, left_end) = edges(left);
    let (middle_start, middle_end) = edges(middle);
    let (right_start, _) = edges(right);

    assert_eq!(left_end, middle_start, "gap between left cap and middle");
    assert_eq!(middle_end, right_start, "gap between middle and right cap");
    // Absolute rounded left edge: tab at round(386.333) = 386, cap at round(385.667) = 386.
    assert_eq!(tree.layout(tab).unwrap().location.x + left_start, 386.0);
}
