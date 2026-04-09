use super::*;

#[test]
fn collect_group_doodads_filters_to_default_and_selected_set_refs() {
    let group = wmo::WmoGroupData {
        doodad_refs: vec![0, 2, 3, 4],
        ..minimal_group()
    };
    let root = wmo::WmoRootData {
        n_groups: 1,
        doodad_sets: vec![
            wmo::WmoDoodadSet {
                name: "$DefaultGlobal".into(),
                start_doodad: 0,
                n_doodads: 2,
            },
            wmo::WmoDoodadSet {
                name: "InnProps".into(),
                start_doodad: 2,
                n_doodads: 2,
            },
        ],
        doodad_names: [
            (0, "world/generic/passive_doodad_0.m2"),
            (1, "world/generic/passive_doodad_1.m2"),
            (2, "world/generic/selected_doodad_2.m2"),
            (3, "world/generic/selected_doodad_3.m2"),
            (4, "world/generic/unused_doodad_4.m2"),
        ]
        .into_iter()
        .map(|(offset, name)| wmo::WmoDoodadName {
            offset,
            name: name.into(),
        })
        .collect(),
        doodad_file_ids: vec![100, 101, 102, 103, 104],
        doodad_defs: vec![
            wmo::WmoDoodadDef {
                name_offset: 0,
                flags: 0,
                position: [1.0, 2.0, 3.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 1.0,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 1,
                flags: 0,
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 1.0,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 2,
                flags: 0,
                position: [4.0, 5.0, 6.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 0.5,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 3,
                flags: 0,
                position: [7.0, 8.0, 9.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 2.0,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 4,
                flags: 0,
                position: [10.0, 11.0, 12.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 3.0,
                color: [1.0; 4],
            },
        ],
        ..minimal_root()
    };

    let doodads = collect_group_doodads(&root, &group, 1);
    assert_eq!(doodads.len(), 3);
    assert_eq!(
        doodads
            .iter()
            .map(|doodad| doodad.model_fdid)
            .collect::<Vec<_>>(),
        vec![100, 102, 103]
    );
    assert_eq!(doodads[0].transform.translation, Vec3::new(-1.0, 3.0, 2.0));
    assert_eq!(doodads[1].transform.scale, Vec3::splat(0.5));
    assert_eq!(doodads[2].transform.scale, Vec3::splat(2.0));
}

#[test]
fn collect_group_lights_filters_to_group_light_refs() {
    let group = wmo::WmoGroupData {
        light_refs: vec![0, 2, 9],
        ..minimal_group()
    };
    let root = wmo::WmoRootData {
        n_groups: 1,
        lights: vec![
            wmo::WmoLight {
                light_type: wmo::WmoLightType::Omni,
                use_attenuation: true,
                color: [1.0, 0.0, 0.0, 1.0],
                position: [1.0, 2.0, 3.0],
                intensity: 4.0,
                rotation: [0.0, 0.0, 0.0, 1.0],
                attenuation_start: 1.0,
                attenuation_end: 5.0,
            },
            wmo::WmoLight {
                light_type: wmo::WmoLightType::Ambient,
                use_attenuation: false,
                color: [0.0, 1.0, 0.0, 1.0],
                position: [4.0, 5.0, 6.0],
                intensity: 2.0,
                rotation: [0.0, 0.0, 0.0, 1.0],
                attenuation_start: 0.0,
                attenuation_end: 0.0,
            },
            wmo::WmoLight {
                light_type: wmo::WmoLightType::Spot,
                use_attenuation: true,
                color: [0.0, 0.0, 1.0, 1.0],
                position: [7.0, 8.0, 9.0],
                intensity: 6.0,
                rotation: [0.0, 0.0, 0.0, 1.0],
                attenuation_start: 2.0,
                attenuation_end: 10.0,
            },
        ],
        ..minimal_root()
    };

    let lights = collect_group_lights(&root, &group);
    assert_eq!(lights.len(), 2);
    assert_eq!(lights[0].0, 0);
    assert_eq!(lights[1].0, 2);
    assert_eq!(lights[0].1.position, [1.0, 2.0, 3.0]);
    assert_eq!(lights[1].1.position, [7.0, 8.0, 9.0]);
}

#[test]
fn collect_group_fogs_filters_to_valid_unique_group_fog_ids() {
    let group = wmo::WmoGroupData {
        header: wmo::WmoGroupHeader {
            fog_ids: [1, 3, 1, 9],
            ..minimal_group_header()
        },
        ..minimal_group()
    };
    let root = wmo::WmoRootData {
        n_groups: 1,
        fogs: vec![
            wmo::WmoFog {
                flags: 0,
                position: [1.0, 2.0, 3.0],
                smaller_radius: 4.0,
                larger_radius: 5.0,
                fog_end: 6.0,
                fog_start_multiplier: 0.2,
                color_1: [0.1, 0.2, 0.3, 1.0],
                underwater_fog_end: 7.0,
                underwater_fog_start_multiplier: 0.3,
                color_2: [0.4, 0.5, 0.6, 1.0],
            },
            wmo::WmoFog {
                flags: 1,
                position: [10.0, 20.0, 30.0],
                smaller_radius: 40.0,
                larger_radius: 50.0,
                fog_end: 60.0,
                fog_start_multiplier: 0.4,
                color_1: [0.7, 0.2, 0.3, 1.0],
                underwater_fog_end: 70.0,
                underwater_fog_start_multiplier: 0.5,
                color_2: [0.4, 0.8, 0.6, 1.0],
            },
            wmo::WmoFog {
                flags: 2,
                position: [100.0, 200.0, 300.0],
                smaller_radius: 400.0,
                larger_radius: 500.0,
                fog_end: 600.0,
                fog_start_multiplier: 0.6,
                color_1: [0.1, 0.9, 0.3, 1.0],
                underwater_fog_end: 700.0,
                underwater_fog_start_multiplier: 0.7,
                color_2: [0.4, 0.5, 0.9, 1.0],
            },
            wmo::WmoFog {
                flags: 3,
                position: [11.0, 22.0, 33.0],
                smaller_radius: 44.0,
                larger_radius: 55.0,
                fog_end: 66.0,
                fog_start_multiplier: 0.8,
                color_1: [0.8, 0.2, 0.3, 1.0],
                underwater_fog_end: 77.0,
                underwater_fog_start_multiplier: 0.9,
                color_2: [0.4, 0.8, 0.9, 1.0],
            },
        ],
        ..minimal_root()
    };

    let fogs = collect_group_fogs(&root, &group);
    assert_eq!(fogs.len(), 2);
    assert_eq!(fogs[0].0, 1);
    assert_eq!(fogs[1].0, 3);
    assert_eq!(fogs[0].1.position, [10.0, 20.0, 30.0]);
    assert_eq!(fogs[1].1.position, [11.0, 22.0, 33.0]);
}

#[test]
fn group_bbox_marks_antiportal_groups_from_authored_name_offsets() {
    let root = wmo::WmoRootData {
        n_groups: 1,
        group_names: vec![
            crate::asset::wmo_format::parser::WmoGroupName {
                offset: 0,
                name: "EntryHall".into(),
                is_antiportal: false,
            },
            crate::asset::wmo_format::parser::WmoGroupName {
                offset: 24,
                name: "antiportal01".into(),
                is_antiportal: true,
            },
        ],
        group_infos: vec![wmo::WmoGroupInfo {
            flags: 0,
            bbox_min: [1.0, 2.0, 3.0],
            bbox_max: [4.0, 5.0, 6.0],
        }],
        ..minimal_root()
    };
    let group_header = wmo::WmoGroupHeader {
        descriptive_group_name_offset: 24,
        ..minimal_group_header()
    };

    let bbox = group_bbox(&root, 0, &group_header);

    assert!(bbox.is_antiportal);
    assert_eq!(bbox.bbox_min, Vec3::new(-4.0, 3.0, 2.0));
    assert_eq!(bbox.bbox_max, Vec3::new(-1.0, 6.0, 5.0));
}

#[test]
fn spawn_wmo_group_fog_preserves_authored_fog_fields() {
    let mut app = App::new();
    let fog = wmo::WmoFog {
        flags: 0,
        position: [10.0, 20.0, 30.0],
        smaller_radius: 4.0,
        larger_radius: 12.0,
        fog_end: 80.0,
        fog_start_multiplier: 0.25,
        color_1: [0.1, 0.2, 0.3, 1.0],
        underwater_fog_end: 90.0,
        underwater_fog_start_multiplier: 0.5,
        color_2: [0.7, 0.8, 0.9, 1.0],
    };

    let entity = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| spawn_wmo_group_fog(&mut commands, 2, &fog))
        .expect("fog entity should spawn");
    app.update();

    let component = app
        .world()
        .get::<WmoGroupFogVolume>(entity)
        .copied()
        .expect("fog component");
    assert_eq!(
        component,
        WmoGroupFogVolume {
            fog_index: 2,
            smaller_radius: 4.0,
            larger_radius: 12.0,
            fog_end: 80.0,
            fog_start_multiplier: 0.25,
            color_1: [0.1, 0.2, 0.3, 1.0],
            underwater_fog_end: 90.0,
            underwater_fog_start_multiplier: 0.5,
            color_2: [0.7, 0.8, 0.9, 1.0],
        }
    );
    let transform = app.world().get::<Transform>(entity).expect("fog transform");
    assert_eq!(transform.translation, Vec3::new(-10.0, 30.0, 20.0));
    assert_eq!(transform.scale, Vec3::splat(12.0));
}
