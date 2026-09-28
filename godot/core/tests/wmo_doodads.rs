use game_engine_core::wmo::{self, WmoDoodadModel};
use glam::{Quat, Vec3};

/// `sw_magicdistrict` root; group 59 (`Jail01`, FDID 456882) is the Stockade entrance.
const MAGIC_DISTRICT_ROOT_FDID: u32 = 321999;
const JAIL_GROUP_FDID: u32 = 456882;
/// `world/generic/activedoodads/instanceportal/instanceportal.m2`.
const INSTANCE_PORTAL_FDID: u32 = 197007;

fn read_model(fdid: u32) -> Vec<u8> {
    let path = format!(
        "{}/../../data/models/{fdid}.wmo",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// The Stockade instance portal is MODD 1112, referenced by `Jail01`'s MODR and inside
/// `Set_$DefaultGlobal` (MODS 0: doodads 0..1177). The root has MODI and no MODN, so
/// MODD `name_offset` 106 indexes MODI (FDID 197007). Authored WMO-local position
/// (-323.41, -136.29, -16.11), identity rotation, scale 1.3597, colour BGRA 32 22 1E FF.
#[test]
fn magic_district_jail_places_the_instance_portal_at_its_authored_transform() {
    let root = wmo::parse_root(&read_model(MAGIC_DISTRICT_ROOT_FDID)).expect("parse root");
    let jail = wmo::parse_group(&read_model(JAIL_GROUP_FDID)).expect("parse Jail01");

    let doodads = wmo::placed_doodads(&root, [(59, jail.geometry.doodad_refs.as_slice())], 0);

    assert_eq!(
        doodads.len(),
        18,
        "every Jail01 MODR entry is in the default set"
    );
    let portal = doodads
        .iter()
        .find(|doodad| doodad.index == 1112)
        .expect("MODD 1112");
    assert_eq!(portal.model, WmoDoodadModel::FileId(INSTANCE_PORTAL_FDID));
    assert_eq!(
        portal.groups,
        [59],
        "drawn with the group that references it"
    );
    // WMO-local (x, y, z) in engine axes is (x, z, -y).
    let expected = Vec3::new(-323.412_26, -16.113_346, 136.293_42);
    assert!(
        portal.translation.distance(expected) < 1e-3,
        "{}",
        portal.translation
    );
    assert!(portal.rotation.angle_between(Quat::IDENTITY) < 1e-5);
    assert!((portal.scale - 1.359_748_6).abs() < 1e-6);
    assert_eq!(
        portal.color,
        [30.0 / 255.0, 34.0 / 255.0, 50.0 / 255.0, 1.0]
    );
}

/// A doodad referenced by several groups is placed once and belongs to each of them.
#[test]
fn doodads_shared_by_groups_are_placed_once() {
    let root = wmo::parse_root(&read_model(MAGIC_DISTRICT_ROOT_FDID)).expect("parse root");
    let jail = wmo::parse_group(&read_model(JAIL_GROUP_FDID)).expect("parse Jail01");
    let refs = jail.geometry.doodad_refs.as_slice();

    let doodads = wmo::placed_doodads(&root, [(3, refs), (7, refs)], 0);

    assert_eq!(doodads.len(), 18);
    assert!(doodads.iter().all(|doodad| doodad.groups == [3, 7]));
}

fn doodad(name_offset: u32, rotation: [f32; 4]) -> wmo::WmoDoodadDef {
    wmo::WmoDoodadDef {
        name_offset,
        flags: 0,
        position: [1.0, 2.0, 3.0],
        rotation,
        scale: 1.0,
        color: [1.0; 4],
    }
}

fn set(start_doodad: u32, n_doodads: u32) -> wmo::WmoDoodadSet {
    wmo::WmoDoodadSet {
        name: String::new(),
        start_doodad,
        n_doodads,
    }
}

/// Three sets over MODD 0..6; MODN names, no MODI.
fn synthetic_root() -> wmo::WmoRootData {
    wmo::WmoRootData {
        n_groups: 1,
        flags: Default::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: vec![],
        lights: vec![],
        doodad_sets: vec![set(0, 2), set(2, 2), set(4, 2)],
        group_names: vec![],
        doodad_names: vec![
            wmo::WmoDoodadName {
                offset: 0,
                name: "world/a.m2".into(),
            },
            wmo::WmoDoodadName {
                offset: 11,
                name: "world/b.m2".into(),
            },
        ],
        doodad_file_ids: vec![],
        doodad_defs: (0..6)
            .map(|index| doodad(index % 2 * 11, [0.0, 0.0, 0.0, 1.0]))
            .collect(),
        fogs: vec![],
        visible_block_vertices: vec![],
        visible_blocks: vec![],
        convex_volume_planes: vec![],
        group_file_data_ids: vec![],
        global_ambient_volumes: vec![],
        ambient_volumes: vec![],
        baked_ambient_box_volumes: vec![],
        dynamic_lights: vec![],
        portals: vec![],
        portal_refs: vec![],
        group_infos: vec![],
        skybox_wow_path: None,
    }
}

fn indices(doodads: &[wmo::WmoDoodad]) -> Vec<u16> {
    doodads.iter().map(|doodad| doodad.index).collect()
}

/// Set 0 (`$DefaultGlobal`) is always active, plus the placement's MODF doodad set
/// (WebWowViewerCpp `WmoObject` constructor `m_activeDoodadSets.set(0)` and
/// `setLoadingParam` `m_activeDoodadSets.set(mapObjDef.doodadSet)`).
#[test]
fn default_set_and_placement_set_are_active() {
    let root = synthetic_root();
    let refs: &[u16] = &[0, 1, 2, 3, 4, 5];

    assert_eq!(indices(&wmo::placed_doodads(&root, [(0, refs)], 0)), [0, 1]);
    assert_eq!(
        indices(&wmo::placed_doodads(&root, [(0, refs)], 2)),
        [0, 1, 4, 5]
    );
    assert_eq!(indices(&wmo::placed_doodads(&root, [(0, refs)], 9)), [0, 1]);
}

/// Only doodads a group references through MODR are placed.
#[test]
fn unreferenced_doodads_are_not_placed() {
    let root = synthetic_root();

    assert_eq!(
        indices(&wmo::placed_doodads(&root, [(0, &[1u16][..])], 1)),
        [1]
    );
}

/// Without MODI, `name_offset` is a byte offset into MODN.
#[test]
fn modn_names_resolve_by_byte_offset() {
    let root = synthetic_root();

    let doodads = wmo::placed_doodads(&root, [(0, &[0u16, 1][..])], 0);

    assert_eq!(doodads[0].model, WmoDoodadModel::Path("world/a.m2".into()));
    assert_eq!(doodads[1].model, WmoDoodadModel::Path("world/b.m2".into()));
}

/// A WoW-local quarter turn about +Z (up) is a quarter turn about engine +Y, and the
/// authored position (x, y, z) sits at engine (x, z, -y).
#[test]
fn rotation_and_position_convert_to_engine_axes() {
    let mut root = synthetic_root();
    let half = std::f32::consts::FRAC_1_SQRT_2;
    root.doodad_defs[0] = doodad(0, [0.0, 0.0, half, half]);

    let placed = &wmo::placed_doodads(&root, [(0, &[0u16][..])], 0)[0];

    assert!(placed.translation.distance(Vec3::new(1.0, 3.0, -2.0)) < 1e-6);
    // WoW +X turned a quarter about +Z is WoW +Y, engine -Z.
    assert!((placed.rotation * Vec3::X).distance(Vec3::NEG_Z) < 1e-6);
}
