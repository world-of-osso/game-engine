/// Default geoset visibility for initial model display.
pub fn default_geoset_visible(mesh_part_id: u16) -> bool {
    let group = mesh_part_id / 100;
    let variant = mesh_part_id % 100;
    match group {
        0 => matches!(mesh_part_id, 0 | 1 | 5 | 16 | 17 | 27..=33),
        1..=3 => variant == 2,
        // WMVx CharacterDefaultsGeosetModifier: ears 702 (solarity BASE_GEOSETS too).
        7 => variant == 2,
        15 => false,
        17 => false,
        // WMVx ModernCharCustomGeosetModifier forces the face to 3202; one face only.
        32 => variant == 2,
        _ => variant == 1,
    }
}

pub fn is_geoset_visible(
    mesh_part_id: u16,
    active_geosets: &[(u16, u16)],
    active_types: &[u16],
) -> bool {
    let group = mesh_part_id / 100;
    let variant = mesh_part_id % 100;
    if !active_types.contains(&group) {
        return default_geoset_visible(mesh_part_id);
    }
    if group == 0 {
        let selected_variant = active_geosets
            .iter()
            .find(|(t, _)| *t == 0)
            .map(|(_, id)| *id)
            .unwrap_or(0);
        return group_zero_visible(mesh_part_id, selected_variant);
    }
    active_geosets
        .iter()
        .any(|(t, id)| *t == group && *id == variant)
}

/// A selected hairstyle shows the body (0) and its own mesh only: WebWowViewerCpp
/// m2Object meshIds, WMVx GeosetState::setVisibility, solarityclient geoset.rs
/// base_geosets. Meshes 1 and 27+ are other hairstyles (orc female 1/27/28 are hair).
fn group_zero_visible(mesh_part_id: u16, selected_variant: u16) -> bool {
    mesh_part_id == 0 || mesh_part_id == selected_variant
}

pub fn apply_exact_geoset_overrides(
    mesh_part_id: u16,
    base_visible: bool,
    overrides: &[(u16, u16)],
) -> bool {
    let group = mesh_part_id / 100;
    let mut visible = base_visible;
    for &(override_group, value) in overrides {
        if override_group == group {
            if value == 0 {
                // Exact hide: only affects mesh group*100+0
                if mesh_part_id == group * 100 {
                    visible = false;
                }
            } else {
                // Group-level switch: show only the target variant, hide others
                visible = mesh_part_id == group * 100 + value;
            }
        }
    }
    visible
}

#[cfg(test)]
mod tests {
    use super::{apply_exact_geoset_overrides, default_geoset_visible, is_geoset_visible};

    #[test]
    fn default_group_zero_uses_initial_mesh_set() {
        for id in [0, 1, 5, 16, 17, 27, 28, 29, 30, 31, 32, 33] {
            assert!(default_geoset_visible(id), "{id}");
        }
        for id in [2, 3, 4, 6, 18, 26, 34] {
            assert!(!default_geoset_visible(id), "{id}");
        }
    }

    #[test]
    fn default_nonzero_groups_keep_their_distinct_variants() {
        for id in [102, 202, 302, 702, 401, 3202] {
            assert!(default_geoset_visible(id), "{id}");
        }
        for id in [
            100, 101, 201, 301, 700, 701, 703, 1501, 1701, 3200, 3201, 402,
        ] {
            assert!(!default_geoset_visible(id), "{id}");
        }
    }

    #[test]
    fn selected_group_zero_shows_only_the_body_and_selected_hair() {
        let selected = [(0, 5)];
        for id in [0, 5] {
            assert!(is_geoset_visible(id, &selected, &[0]), "{id}");
        }
        for id in [1, 2, 16, 17, 27, 28, 33, 34] {
            assert!(!is_geoset_visible(id, &selected, &[0]), "{id}");
        }
        assert!(is_geoset_visible(16, &[(0, 16)], &[0]));
        assert!(!is_geoset_visible(5, &[(0, 16)], &[0]));
    }

    #[test]
    fn selected_group_without_pair_hides_variants_and_inactive_group_uses_default() {
        assert!(!is_geoset_visible(402, &[], &[4]));
        assert!(is_geoset_visible(402, &[(4, 2)], &[4]));
        assert!(!is_geoset_visible(401, &[(4, 2)], &[4]));
        assert!(is_geoset_visible(401, &[(4, 2)], &[]));
        assert!(is_geoset_visible(0, &[], &[0]));
        assert!(!is_geoset_visible(5, &[], &[0]));
    }

    #[test]
    fn exact_zero_hides_only_group_base_mesh() {
        assert!(!apply_exact_geoset_overrides(400, true, &[(4, 0)]));
        assert!(apply_exact_geoset_overrides(401, true, &[(4, 0)]));
        assert!(!apply_exact_geoset_overrides(401, false, &[(4, 0)]));
        assert!(apply_exact_geoset_overrides(501, true, &[(4, 0)]));
    }

    #[test]
    fn exact_nonzero_selects_variant_and_order_is_last_matching_override() {
        assert!(apply_exact_geoset_overrides(402, false, &[(4, 2)]));
        assert!(!apply_exact_geoset_overrides(401, true, &[(4, 2)]));
        assert!(!apply_exact_geoset_overrides(402, false, &[(4, 2), (4, 1)]));
        assert!(apply_exact_geoset_overrides(401, false, &[(4, 2), (4, 1)]));
        assert!(apply_exact_geoset_overrides(402, false, &[(4, 2), (4, 0)]));
        assert!(!apply_exact_geoset_overrides(400, false, &[(4, 2), (4, 0)]));
    }
}
