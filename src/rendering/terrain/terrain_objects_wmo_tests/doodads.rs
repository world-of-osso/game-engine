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
fn resolve_wmo_doodad_fdid_indexes_modi_by_name_offset() {
    let root = wmo::WmoRootData {
        doodad_file_ids: vec![1001, 2002],
        ..minimal_root()
    };
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), Some(1001));
    assert_eq!(resolve_wmo_doodad_fdid(&root, 1), Some(2002));
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

const MAGIC_DISTRICT_ROOT_FDID: u32 = 321999;
/// `sw_magicdistrict` group 59, `Jail01`: the Stockade entrance tunnel.
const MAGIC_DISTRICT_JAIL_GROUP: usize = 59;
/// `world/generic/activedoodads/instanceportal/instanceportal.m2`.
const INSTANCE_PORTAL_FDID: u32 = 197007;

/// `sw_magicdistrict` has MODI but no MODN, so MODD `name_offset` indexes MODI directly
/// (WebWowViewerCpp `wmoObject.cpp`: `doodadFileDataIds[doodadDef->name_offset]`). Its
/// `Jail01` group carries the Stockade instance portal (MODD 1112) in the doorway of area
/// trigger 101 at WoW (-8761.85, 848.56, 87.81).
#[test]
fn stockade_entrance_group_places_the_instance_portal_at_the_area_trigger() {
    let obj0 =
        std::fs::read("data/terrain/777628.adt").expect("azeroth_30_48_obj0 in data/terrain");
    let objects = adt_obj::load_adt_obj0(&obj0).expect("parse azeroth_30_48_obj0");
    let placement = objects
        .wmos
        .into_iter()
        .find(|placement| resolve_wmo_fdid(placement) == Some(MAGIC_DISTRICT_ROOT_FDID))
        .expect("sw_magicdistrict MODF placement");
    let root_data = std::fs::read(format!("data/models/{MAGIC_DISTRICT_ROOT_FDID}.wmo"))
        .expect("sw_magicdistrict root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse sw_magicdistrict root");
    let group_fdid = root.group_file_data_ids[MAGIC_DISTRICT_JAIL_GROUP];
    let group_data = std::fs::read(format!("data/models/{group_fdid}.wmo"))
        .expect("sw_magicdistrict Jail01 group in data/models");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("parse Jail01");

    let doodads = collect_group_doodads(&root, &group, placement.doodad_set);

    let portal = doodads
        .iter()
        .find(|doodad| doodad.model_fdid == INSTANCE_PORTAL_FDID)
        .expect("Jail01 spawns instanceportal.m2");
    let placement_transform = super::super::super::wmo_transform(&placement, 30, 48);
    let world = placement_transform.transform_point(portal.transform.translation);
    let trigger = Vec3::new(-8761.85, 87.81, -848.56);
    assert!(
        world.distance(trigger) < 5.0,
        "portal at {world}, trigger at {trigger}"
    );
}

/// The Stockade instance portal, a WMO doodad of `sw_magicdistrict` `Jail01`, authors six
/// particle emitters: the swirl Retail draws in the doorway.
#[test]
fn stockade_instance_portal_doodad_spawns_its_six_particle_emitters() {
    let root_data = std::fs::read(format!("data/models/{MAGIC_DISTRICT_ROOT_FDID}.wmo"))
        .expect("sw_magicdistrict root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse sw_magicdistrict root");
    let group_fdid = root.group_file_data_ids[MAGIC_DISTRICT_JAIL_GROUP];
    let group_data = std::fs::read(format!("data/models/{group_fdid}.wmo"))
        .expect("sw_magicdistrict Jail01 group in data/models");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("parse Jail01");
    let mut app = App::new();
    app.world_mut().init_resource::<Assets<Mesh>>();
    app.world_mut().init_resource::<Assets<M2Material>>();
    app.world_mut().init_resource::<Assets<WaterMaterial>>();
    app.world_mut().init_resource::<Assets<Image>>();
    app.world_mut().init_resource::<Assets<M2EffectMaterial>>();
    app.world_mut()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    let group_entity = app.world_mut().spawn(Transform::default()).id();

    app.world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<M2Material>>,
                  mut water_materials: ResMut<Assets<WaterMaterial>>,
                  mut images: ResMut<Assets<Image>>,
                  mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
                  mut inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>| {
                let mut assets = WmoAssets {
                    meshes: &mut meshes,
                    materials: &mut materials,
                    water_materials: &mut water_materials,
                    images: &mut images,
                    effect_materials: &mut effect_materials,
                    inverse_bindposes: &mut inverse_bindposes,
                };
                spawn_wmo_group_doodads(&mut commands, &mut assets, &root, &group, group_entity, 0);
            },
        )
        .expect("spawn Jail01 doodads");
    app.update();

    let portal = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == INSTANCE_PORTAL_FDID.to_string())
        .map(|(entity, _)| entity)
        .expect("instance portal doodad spawned");
    let emitters: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<crate::particle::ParticleEmitterComp>>()
        .iter(app.world())
        .filter(|&emitter| descends_from(app.world(), emitter, portal))
        .collect();
    assert_eq!(emitters.len(), 6);
}

fn descends_from(world: &World, mut entity: Entity, ancestor: Entity) -> bool {
    while let Some(child_of) = world.get::<ChildOf>(entity) {
        entity = child_of.parent();
        if entity == ancestor {
            return true;
        }
    }
    false
}
