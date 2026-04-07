use super::parser::{WmoPortal, WmoPortalRef};

/// Determine which side of a portal the camera is on.
/// Returns positive if camera is on the normal side, negative on the back side.
pub fn portal_camera_side(portal: &WmoPortal, camera_pos: [f32; 3]) -> f32 {
    if portal.vertices.is_empty() {
        return 0.0;
    }
    let center = portal_center(portal);
    let dx = camera_pos[0] - center[0];
    let dy = camera_pos[1] - center[1];
    let dz = camera_pos[2] - center[2];
    dx * portal.normal[0] + dy * portal.normal[1] + dz * portal.normal[2]
}

fn portal_center(portal: &WmoPortal) -> [f32; 3] {
    let n = portal.vertices.len() as f32;
    let mut c = [0.0f32; 3];
    for v in &portal.vertices {
        c[0] += v[0];
        c[1] += v[1];
        c[2] += v[2];
    }
    c[0] /= n;
    c[1] /= n;
    c[2] /= n;
    c
}

/// Propagate visibility from a starting group through portals.
/// Returns a set of visible group indices.
pub fn propagate_visibility(
    start_group: u16,
    portals: &[WmoPortal],
    portal_refs: &[WmoPortalRef],
    group_portal_ranges: &[(u16, u16)],
    camera_pos: [f32; 3],
) -> Vec<u16> {
    let mut visible = Vec::new();
    let mut visited = Vec::new();
    visit_group(
        start_group,
        portals,
        portal_refs,
        group_portal_ranges,
        camera_pos,
        &mut visible,
        &mut visited,
    );
    visible
}

fn visit_group(
    group: u16,
    portals: &[WmoPortal],
    portal_refs: &[WmoPortalRef],
    group_portal_ranges: &[(u16, u16)],
    camera_pos: [f32; 3],
    visible: &mut Vec<u16>,
    visited: &mut Vec<u16>,
) {
    if visited.contains(&group) {
        return;
    }
    visited.push(group);
    visible.push(group);

    let Some(&(start, count)) = group_portal_ranges.get(group as usize) else {
        return;
    };
    for i in start..(start + count) {
        let Some(pref) = portal_refs.get(i as usize) else {
            continue;
        };
        let Some(portal) = portals.get(pref.portal_index as usize) else {
            continue;
        };
        let dot = portal_camera_side(portal, camera_pos);
        let camera_on_front = (dot >= 0.0 && pref.side >= 0) || (dot < 0.0 && pref.side < 0);
        if camera_on_front {
            visit_group(
                pref.group_index,
                portals,
                portal_refs,
                group_portal_ranges,
                camera_pos,
                visible,
                visited,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn portal_xy(normal: [f32; 3]) -> WmoPortal {
        WmoPortal {
            vertices: vec![
                [0.0, -1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 2.0],
                [0.0, -1.0, 2.0],
            ],
            normal,
        }
    }

    // --- Camera-side detection ---

    #[test]
    fn camera_on_normal_side() {
        let portal = portal_xy([1.0, 0.0, 0.0]);
        let dot = portal_camera_side(&portal, [5.0, 0.0, 1.0]);
        assert!(dot > 0.0, "camera should be on normal side, dot={dot}");
    }

    #[test]
    fn camera_on_back_side() {
        let portal = portal_xy([1.0, 0.0, 0.0]);
        let dot = portal_camera_side(&portal, [-5.0, 0.0, 1.0]);
        assert!(dot < 0.0, "camera should be on back side, dot={dot}");
    }

    #[test]
    fn camera_on_portal_plane() {
        let portal = portal_xy([1.0, 0.0, 0.0]);
        let dot = portal_camera_side(&portal, [0.0, 0.0, 1.0]);
        assert!(
            dot.abs() < 0.01,
            "camera on plane should have ~0 dot, got {dot}"
        );
    }

    #[test]
    fn camera_side_empty_portal() {
        let portal = WmoPortal {
            vertices: vec![],
            normal: [1.0, 0.0, 0.0],
        };
        assert_eq!(portal_camera_side(&portal, [5.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn camera_side_z_axis_normal() {
        let portal = WmoPortal {
            vertices: vec![[0.0, 0.0, 5.0]],
            normal: [0.0, 0.0, 1.0],
        };
        assert!(portal_camera_side(&portal, [0.0, 0.0, 10.0]) > 0.0);
        assert!(portal_camera_side(&portal, [0.0, 0.0, 0.0]) < 0.0);
    }

    // --- Visibility propagation ---

    // Setup: 3 groups connected linearly: 0 --portal0--> 1 --portal1--> 2
    fn linear_portals() -> (Vec<WmoPortal>, Vec<WmoPortalRef>, Vec<(u16, u16)>) {
        let portals = vec![
            // Portal 0: at x=5, normal pointing +X (from group 0 to group 1)
            WmoPortal {
                vertices: vec![[5.0, 0.0, 0.0]],
                normal: [1.0, 0.0, 0.0],
            },
            // Portal 1: at x=10, normal pointing +X (from group 1 to group 2)
            WmoPortal {
                vertices: vec![[10.0, 0.0, 0.0]],
                normal: [1.0, 0.0, 0.0],
            },
        ];
        let portal_refs = vec![
            // Group 0 (x<5) portal ref → portal 0, connects to group 1, group 0 on neg side
            WmoPortalRef {
                portal_index: 0,
                group_index: 1,
                side: -1,
            },
            // Group 1 (5<x<10) looking back through portal 0 (pos side of normal)
            WmoPortalRef {
                portal_index: 0,
                group_index: 0,
                side: 1,
            },
            // Group 1 looking forward through portal 1 (neg side)
            WmoPortalRef {
                portal_index: 1,
                group_index: 2,
                side: -1,
            },
            // Group 2 (x>10) looking back through portal 1 (pos side)
            WmoPortalRef {
                portal_index: 1,
                group_index: 1,
                side: 1,
            },
        ];
        // (start_portal_ref_index, count) for each group
        let group_ranges = vec![
            (0, 1), // group 0: ref[0]
            (1, 2), // group 1: ref[1..3]
            (3, 1), // group 2: ref[3]
        ];
        (portals, portal_refs, group_ranges)
    }

    #[test]
    fn propagate_from_group_0_camera_in_group_0() {
        let (portals, refs, ranges) = linear_portals();
        let camera = [2.0, 0.0, 0.0]; // in group 0 (x < 5)
        let visible = propagate_visibility(0, &portals, &refs, &ranges, camera);
        // Group 0 always visible. Camera is on the normal side of portal 0 → group 1 visible.
        // From group 1, camera is on the normal side of portal 1 → group 2 visible.
        assert!(visible.contains(&0));
        assert!(visible.contains(&1));
    }

    #[test]
    fn propagate_from_group_2_camera_in_group_2() {
        let (portals, refs, ranges) = linear_portals();
        let camera = [15.0, 0.0, 0.0]; // in group 2 (x > 10)
        let visible = propagate_visibility(2, &portals, &refs, &ranges, camera);
        assert!(visible.contains(&2));
        // Group 2 portal ref side=1, camera dot > 0 → matches → propagates to group 1
        assert!(visible.contains(&1));
    }

    #[test]
    fn propagate_single_group_no_portals() {
        let visible = propagate_visibility(0, &[], &[], &[(0, 0)], [0.0, 0.0, 0.0]);
        assert_eq!(visible, vec![0]);
    }

    #[test]
    fn propagate_empty_ranges() {
        let visible = propagate_visibility(0, &[], &[], &[], [0.0, 0.0, 0.0]);
        assert_eq!(visible, vec![0]);
    }

    #[test]
    fn propagate_no_cycles() {
        // Even if portals form a loop, visited tracking prevents infinite recursion
        let portals = vec![WmoPortal {
            vertices: vec![[5.0, 0.0, 0.0]],
            normal: [1.0, 0.0, 0.0],
        }];
        let refs = vec![
            WmoPortalRef {
                portal_index: 0,
                group_index: 1,
                side: 1,
            },
            WmoPortalRef {
                portal_index: 0,
                group_index: 0,
                side: 1,
            },
        ];
        let ranges = vec![(0, 1), (1, 1)];
        let camera = [10.0, 0.0, 0.0];
        let visible = propagate_visibility(0, &portals, &refs, &ranges, camera);
        // Should visit 0 and 1 without infinite loop
        assert!(visible.contains(&0));
        assert!(visible.contains(&1));
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn portal_center_computed_correctly() {
        let portal = WmoPortal {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                [4.0, 4.0, 0.0],
                [0.0, 4.0, 0.0],
            ],
            normal: [0.0, 0.0, 1.0],
        };
        let center = super::portal_center(&portal);
        assert!((center[0] - 2.0).abs() < 0.01);
        assert!((center[1] - 2.0).abs() < 0.01);
        assert!((center[2] - 0.0).abs() < 0.01);
    }
}
