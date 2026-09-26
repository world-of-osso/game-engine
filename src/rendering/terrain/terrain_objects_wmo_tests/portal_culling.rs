use super::*;
use bevy::camera::{CameraProjection, PerspectiveProjection};
use bevy::state::app::StatesPlugin;
use game_engine::culling::{CullingPlugin, Wmo};
use game_engine::game_state_enum::GameState;

pub(super) const TRADE_DISTRICT_ROOT_FDID: u32 = 322057;
/// `azeroth_30_48_obj0.adt`: its MODF places `sw_tradedistrict`.
const TRADE_DISTRICT_OBJ0: &str = "data/terrain/777628.adt";
const GROUP_ANTIPORTAL: u16 = 31;
const GROUP_TRADE_NW: u16 = 36;
const GROUP_TRADE_NE: u16 = 37;
const GROUP_AH_INTERIOR: u16 = 53;
const GROUP_AH_EXTERIOR: u16 = 60;

/// The user's camera near Auctioneer Fitch (WoW -8818, 645.7, 100.9), looking at the player.
/// It lies inside the bounding boxes of the district antiportal (31), exterior groups 35, 36,
/// 37, 42, 60 and the auction house interior (53), but over group 36's exterior paving and
/// outside the auction house walls. Retail picks the group with a floor below the camera, sees
/// it is exterior and draws every exterior group; the engine hid the whole district, so the
/// ADT terrain under the plaza showed instead.
#[test]
fn trade_district_exterior_groups_are_drawn_from_the_auction_house_plaza() {
    let camera = Transform::from_xyz(-8818.0, 100.9, -645.7)
        .looking_at(Vec3::new(-8818.0, 96.5, -660.0), Vec3::Y);
    let mut app = trade_district_culling_app(camera);

    app.update();

    let visibility = group_visibility(app.world_mut());
    for group in [GROUP_TRADE_NW, GROUP_TRADE_NE, GROUP_AH_EXTERIOR] {
        assert_eq!(
            visibility.get(&group),
            Some(&Visibility::Visible),
            "exterior group {group}"
        );
    }
    assert_eq!(visibility.get(&GROUP_ANTIPORTAL), Some(&Visibility::Hidden));
}

/// Standing on the auction house floor (WoW -8818, 660, 98.06) the camera is inside interior
/// group 53, so it is drawn from there.
#[test]
fn trade_district_auction_house_interior_is_drawn_from_inside() {
    let camera = Transform::from_xyz(-8818.0, 100.0, -660.0)
        .looking_at(Vec3::new(-8818.0, 99.0, -670.0), Vec3::Y);
    let mut app = trade_district_culling_app(camera);

    app.update();

    let visibility = group_visibility(app.world_mut());
    assert_eq!(
        visibility.get(&GROUP_AH_INTERIOR),
        Some(&Visibility::Visible)
    );
}

fn trade_district_culling_app(camera: Transform) -> App {
    wmo_culling_app(load_trade_district(), camera)
}

fn wmo_culling_app(
    (placement_transform, root, groups): (Transform, wmo::WmoRootData, Vec<wmo::WmoGroupData>),
    camera: Transform,
) -> App {
    let mut app = App::new();
    app.add_plugins(StatesPlugin)
        .insert_state(GameState::InWorld)
        .add_plugins(CullingPlugin);
    let projection = PerspectiveProjection {
        fov: 60f32.to_radians(),
        aspect_ratio: 16.0 / 9.0,
        ..default()
    };
    let camera_gtf = GlobalTransform::from(camera);
    app.world_mut().spawn((
        Camera3d::default(),
        camera,
        camera_gtf,
        projection.compute_frustum(&camera_gtf),
    ));
    let root_entity = app
        .world_mut()
        .spawn((
            Wmo,
            GlobalTransform::from(placement_transform),
            build_portal_graph(&root),
        ))
        .id();
    let mut commands = app.world_mut().commands();
    for (index, group) in groups.iter().enumerate() {
        let group_entity = spawn_wmo_group_entity(&mut commands, &root, group, index as u16);
        commands.entity(root_entity).add_child(group_entity);
    }
    app.world_mut().flush();
    app
}

pub(super) fn trade_district_placement() -> adt_obj::WmoPlacement {
    let obj0 = std::fs::read(TRADE_DISTRICT_OBJ0).expect("azeroth_30_48_obj0 in data/terrain");
    let objects = adt_obj::load_adt_obj0(&obj0).expect("parse azeroth_30_48_obj0");
    objects
        .wmos
        .into_iter()
        .find(|placement| resolve_wmo_fdid(placement) == Some(TRADE_DISTRICT_ROOT_FDID))
        .expect("sw_tradedistrict MODF placement")
}

pub(super) fn load_trade_district() -> (Transform, wmo::WmoRootData, Vec<wmo::WmoGroupData>) {
    load_tile_30_48_wmo(TRADE_DISTRICT_ROOT_FDID)
}

/// The MODF placement, root and groups of a WMO that `azeroth_30_48_obj0` places.
pub(super) fn load_tile_30_48_wmo(
    root_fdid: u32,
) -> (Transform, wmo::WmoRootData, Vec<wmo::WmoGroupData>) {
    let obj0 = std::fs::read(TRADE_DISTRICT_OBJ0).expect("azeroth_30_48_obj0 in data/terrain");
    let objects = adt_obj::load_adt_obj0(&obj0).expect("parse azeroth_30_48_obj0");
    let placement = objects
        .wmos
        .into_iter()
        .find(|placement| resolve_wmo_fdid(placement) == Some(root_fdid))
        .expect("MODF placement");
    let placement_transform = super::super::super::wmo_transform(&placement, 30, 48);
    let root_data =
        std::fs::read(format!("data/models/{root_fdid}.wmo")).expect("root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse WMO root");
    let groups = (0..root.n_groups as usize)
        .map(|index| {
            let fdid = root.group_file_data_ids[index];
            let data =
                std::fs::read(format!("data/models/{fdid}.wmo")).expect("WMO group in data/models");
            wmo::load_wmo_group_with_root(&data, Some(&root)).expect("parse group")
        })
        .collect();
    (placement_transform, root, groups)
}

fn group_visibility(world: &mut World) -> std::collections::HashMap<u16, Visibility> {
    world
        .query::<(&game_engine::culling::WmoGroup, &Visibility)>()
        .iter(world)
        .map(|(group, visibility)| (group.group_index, *visibility))
        .collect()
}
