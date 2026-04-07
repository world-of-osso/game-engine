use super::parser::WmoBspNode;

/// BSP node axis flags.
const BSP_AXIS_Y: u16 = 0x02;
const BSP_LEAF: u16 = 0x04;

impl WmoBspNode {
    /// Whether this is a leaf node (contains faces, no children).
    pub fn is_leaf(&self) -> bool {
        (self.flags & BSP_LEAF) != 0
    }

    /// Split axis: 0 = X, 1 = Y. Only meaningful for non-leaf nodes.
    pub fn split_axis(&self) -> usize {
        if (self.flags & BSP_AXIS_Y) != 0 { 1 } else { 0 }
    }
}

/// Collect all face indices referenced by BSP leaf nodes that a point falls into.
/// `point` is in group-local coordinates; axes 0=X, 1=Y are used for splitting.
pub fn bsp_query_point(nodes: &[WmoBspNode], face_refs: &[u16], point: [f32; 2]) -> Vec<u16> {
    let mut result = Vec::new();
    if nodes.is_empty() {
        return result;
    }
    bsp_traverse(nodes, face_refs, 0, point, &mut result);
    result
}

fn bsp_traverse(
    nodes: &[WmoBspNode],
    face_refs: &[u16],
    index: usize,
    point: [f32; 2],
    result: &mut Vec<u16>,
) {
    let node = &nodes[index];

    if node.is_leaf() {
        let start = node.face_start as usize;
        let end = start + node.face_count as usize;
        if end <= face_refs.len() {
            result.extend_from_slice(&face_refs[start..end]);
        }
        return;
    }

    let axis = node.split_axis();
    let dist = node.plane_dist;

    if point[axis] <= dist && node.neg_child >= 0 {
        bsp_traverse(nodes, face_refs, node.neg_child as usize, point, result);
    }
    if point[axis] >= dist && node.pos_child >= 0 {
        bsp_traverse(nodes, face_refs, node.pos_child as usize, point, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(face_start: u32, face_count: u16) -> WmoBspNode {
        WmoBspNode {
            flags: BSP_LEAF,
            neg_child: -1,
            pos_child: -1,
            face_count,
            face_start,
            plane_dist: 0.0,
        }
    }

    fn split_x(dist: f32, neg: i16, pos: i16) -> WmoBspNode {
        WmoBspNode {
            flags: 0x01, // X-axis split
            neg_child: neg,
            pos_child: pos,
            face_count: 0,
            face_start: 0,
            plane_dist: dist,
        }
    }

    fn split_y(dist: f32, neg: i16, pos: i16) -> WmoBspNode {
        WmoBspNode {
            flags: BSP_AXIS_Y,
            neg_child: neg,
            pos_child: pos,
            face_count: 0,
            face_start: 0,
            plane_dist: dist,
        }
    }

    // --- Node properties ---

    #[test]
    fn leaf_node_detected() {
        assert!(leaf(0, 3).is_leaf());
        assert!(!split_x(5.0, 1, 2).is_leaf());
    }

    #[test]
    fn split_axis_x() {
        assert_eq!(split_x(5.0, -1, -1).split_axis(), 0);
    }

    #[test]
    fn split_axis_y() {
        assert_eq!(split_y(5.0, -1, -1).split_axis(), 1);
    }

    // --- Single leaf ---

    #[test]
    fn query_single_leaf() {
        let nodes = vec![leaf(0, 3)];
        let face_refs = vec![10, 20, 30];
        let result = bsp_query_point(&nodes, &face_refs, [0.0, 0.0]);
        assert_eq!(result, vec![10, 20, 30]);
    }

    // --- Simple tree: root splits X at 5.0, two leaves ---

    #[test]
    fn query_left_of_split() {
        //  [0] split X at 5.0, neg=1, pos=2
        //  [1] leaf faces 0..2
        //  [2] leaf faces 2..4
        let nodes = vec![split_x(5.0, 1, 2), leaf(0, 2), leaf(2, 2)];
        let face_refs = vec![10, 11, 20, 21];
        let result = bsp_query_point(&nodes, &face_refs, [3.0, 0.0]);
        assert_eq!(result, vec![10, 11]);
    }

    #[test]
    fn query_right_of_split() {
        let nodes = vec![split_x(5.0, 1, 2), leaf(0, 2), leaf(2, 2)];
        let face_refs = vec![10, 11, 20, 21];
        let result = bsp_query_point(&nodes, &face_refs, [7.0, 0.0]);
        assert_eq!(result, vec![20, 21]);
    }

    #[test]
    fn query_on_split_plane_returns_both() {
        let nodes = vec![split_x(5.0, 1, 2), leaf(0, 2), leaf(2, 2)];
        let face_refs = vec![10, 11, 20, 21];
        let result = bsp_query_point(&nodes, &face_refs, [5.0, 0.0]);
        assert_eq!(result, vec![10, 11, 20, 21]);
    }

    // --- Y-axis split ---

    #[test]
    fn query_y_split() {
        let nodes = vec![split_y(3.0, 1, 2), leaf(0, 1), leaf(1, 1)];
        let face_refs = vec![100, 200];
        let below = bsp_query_point(&nodes, &face_refs, [0.0, 1.0]);
        assert_eq!(below, vec![100]);
        let above = bsp_query_point(&nodes, &face_refs, [0.0, 5.0]);
        assert_eq!(above, vec![200]);
    }

    // --- Deeper tree ---

    #[test]
    fn query_two_level_tree() {
        //  [0] split X at 10.0, neg=1, pos=2
        //  [1] split Y at 5.0,  neg=3, pos=4
        //  [2] leaf faces 4..6
        //  [3] leaf faces 0..2
        //  [4] leaf faces 2..4
        let nodes = vec![
            split_x(10.0, 1, 2),
            split_y(5.0, 3, 4),
            leaf(4, 2),
            leaf(0, 2),
            leaf(2, 2),
        ];
        let face_refs = vec![1, 2, 3, 4, 5, 6];
        // x < 10, y < 5 → node 3
        assert_eq!(bsp_query_point(&nodes, &face_refs, [3.0, 2.0]), vec![1, 2]);
        // x < 10, y > 5 → node 4
        assert_eq!(bsp_query_point(&nodes, &face_refs, [3.0, 8.0]), vec![3, 4]);
        // x > 10 → node 2
        assert_eq!(bsp_query_point(&nodes, &face_refs, [15.0, 0.0]), vec![5, 6]);
    }

    // --- Edge cases ---

    #[test]
    fn query_empty_tree() {
        let result = bsp_query_point(&[], &[], [0.0, 0.0]);
        assert!(result.is_empty());
    }

    #[test]
    fn query_leaf_with_no_faces() {
        let nodes = vec![leaf(0, 0)];
        let result = bsp_query_point(&nodes, &[], [0.0, 0.0]);
        assert!(result.is_empty());
    }

    #[test]
    fn query_node_with_no_children() {
        // Split node but both children are -1
        let nodes = vec![split_x(5.0, -1, -1)];
        let result = bsp_query_point(&nodes, &[], [3.0, 0.0]);
        assert!(result.is_empty());
    }
}
