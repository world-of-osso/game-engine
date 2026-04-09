use super::*;

#[test]
fn resolve_wmo_group_fdids_uses_gfid_when_available() {
    let gfid = vec![100, 200, 300];
    let result = resolve_wmo_group_fdids(999, 3, &gfid);
    assert_eq!(result, vec![Some(100), Some(200), Some(300)]);
}

#[test]
fn resolve_wmo_group_fdids_treats_zero_gfid_as_none() {
    let gfid = vec![100, 0, 300];
    let result = resolve_wmo_group_fdids(999, 3, &gfid);
    assert_eq!(result, vec![Some(100), None, Some(300)]);
}

#[test]
fn resolve_wmo_group_fdids_truncates_gfid_to_n_groups() {
    let gfid = vec![100, 200, 300, 400];
    let result = resolve_wmo_group_fdids(999, 2, &gfid);
    assert_eq!(result, vec![Some(100), Some(200)]);
}

#[test]
fn resolve_wmo_doodad_fdid_prefers_modi_over_modn() {
    let root = wmo::WmoRootData {
        doodad_names: vec![
            wmo::WmoDoodadName {
                offset: 0,
                name: "torch.m2".into(),
            },
            wmo::WmoDoodadName {
                offset: 9,
                name: "barrel.m2".into(),
            },
        ],
        doodad_file_ids: vec![1001, 2002],
        ..minimal_root()
    };
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), Some(1001));
    assert_eq!(resolve_wmo_doodad_fdid(&root, 9), Some(2002));
}

#[test]
fn resolve_wmo_doodad_fdid_skips_zero_modi_entry() {
    let root = wmo::WmoRootData {
        doodad_names: vec![wmo::WmoDoodadName {
            offset: 0,
            name: "torch.m2".into(),
        }],
        doodad_file_ids: vec![0],
        ..minimal_root()
    };
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), None);
}

#[test]
fn resolve_wmo_doodad_fdid_returns_none_for_unknown_offset() {
    let root = wmo::WmoRootData {
        doodad_names: vec![wmo::WmoDoodadName {
            offset: 0,
            name: "torch.m2".into(),
        }],
        doodad_file_ids: vec![1001],
        ..minimal_root()
    };
    assert_eq!(resolve_wmo_doodad_fdid(&root, 99), None);
}

#[test]
fn wmo_doodad_transform_applies_position_and_scale() {
    let def = wmo::WmoDoodadDef {
        name_offset: 0,
        flags: 0,
        position: [100.0, 200.0, 50.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: 2.0,
        color: [1.0; 4],
    };
    let transform = wmo_doodad_transform(&def);
    assert!((transform.scale.x - 2.0).abs() < 0.01);
    assert!((transform.scale.y - 2.0).abs() < 0.01);
    assert!((transform.scale.z - 2.0).abs() < 0.01);
    let pos = transform.translation;
    assert!(
        pos.x.abs() + pos.y.abs() + pos.z.abs() > 0.0,
        "position should be nonzero"
    );
}

#[test]
fn wmo_doodad_transform_unit_scale() {
    let def = wmo::WmoDoodadDef {
        name_offset: 0,
        flags: 0,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: 1.0,
        color: [1.0; 4],
    };
    let transform = wmo_doodad_transform(&def);
    assert!((transform.scale.x - 1.0).abs() < 0.01);
}

#[test]
fn doodad_set_range_selects_defs() {
    let sets = [
        wmo::WmoDoodadSet {
            name: "Set0".into(),
            start_doodad: 0,
            n_doodads: 2,
        },
        wmo::WmoDoodadSet {
            name: "Set1".into(),
            start_doodad: 2,
            n_doodads: 1,
        },
    ];
    let defs = [
        wmo::WmoDoodadDef {
            name_offset: 0,
            flags: 0,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: 1.0,
            color: [1.0; 4],
        },
        wmo::WmoDoodadDef {
            name_offset: 0,
            flags: 0,
            position: [1.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: 1.0,
            color: [1.0; 4],
        },
        wmo::WmoDoodadDef {
            name_offset: 0,
            flags: 0,
            position: [2.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: 1.0,
            color: [1.0; 4],
        },
    ];

    let set0_range =
        sets[0].start_doodad as usize..(sets[0].start_doodad + sets[0].n_doodads) as usize;
    let set0_defs = &defs[set0_range];
    assert_eq!(set0_defs.len(), 2);
    assert_eq!(set0_defs[0].position, [0.0; 3]);
    assert_eq!(set0_defs[1].position, [1.0; 3]);

    let set1_range =
        sets[1].start_doodad as usize..(sets[1].start_doodad + sets[1].n_doodads) as usize;
    let set1_defs = &defs[set1_range];
    assert_eq!(set1_defs.len(), 1);
    assert_eq!(set1_defs[0].position, [2.0; 3]);
}

#[test]
fn resolve_doodad_fdid_empty_modi() {
    let root = wmo::WmoRootData {
        doodad_names: vec![wmo::WmoDoodadName {
            offset: 0,
            name: "torch.m2".into(),
        }],
        doodad_file_ids: vec![],
        ..minimal_root()
    };
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), None);
}

#[test]
fn resolve_doodad_fdid_empty_modn() {
    let root = wmo::WmoRootData {
        doodad_names: vec![],
        doodad_file_ids: vec![1001],
        ..minimal_root()
    };
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), None);
}

#[test]
fn resolve_doodad_fdid_modi_index_beyond_range() {
    let root = wmo::WmoRootData {
        doodad_names: vec![
            wmo::WmoDoodadName {
                offset: 0,
                name: "a.m2".into(),
            },
            wmo::WmoDoodadName {
                offset: 5,
                name: "b.m2".into(),
            },
        ],
        doodad_file_ids: vec![1001],
        ..minimal_root()
    };
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), Some(1001));
    assert_eq!(resolve_wmo_doodad_fdid(&root, 5), None);
}
