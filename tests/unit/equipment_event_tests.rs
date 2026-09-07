use super::tests::{configure_equipment_test_app, spawn_head_equipment_owner};
use super::*;

fn helm_fixture() -> &'static Path {
    let path = Path::new("data/item-models/item/objectcomponents/head/helm_plate_d_02_hum.m2");
    assert!(
        path.exists(),
        "equipment event tests require the existing helmet fixture"
    );
    path
}

fn empty_animation(joint: Entity) -> M2AnimData {
    M2AnimData {
        bones: vec![],
        spherical_billboards: vec![],
        sequences: vec![],
        bone_tracks: vec![],
        joint_entities: vec![joint],
    }
}

#[test]
fn equipment_is_not_polled_on_unchanged_frames() {
    let mut app = App::new();
    configure_equipment_test_app(&mut app);
    let owner =
        spawn_head_equipment_owner(&mut app, helm_fixture(), Vec3::ZERO, Vec3::ZERO, [0; 3]);
    app.update();
    let item =
        app.world().get::<RenderedEquipment>(owner).unwrap().slots[&EquipmentSlot::Head].entity;
    app.world_mut()
        .remove_resource::<Assets<StandardMaterial>>();
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world().get::<RenderedEquipment>(owner).unwrap().slots[&EquipmentSlot::Head].entity,
        item
    );
    assert!(app.world().get_entity(item).is_ok());
}

#[test]
fn equipment_waits_for_model_and_rebinds_after_model_replacement() {
    let mut app = App::new();
    configure_equipment_test_app(&mut app);
    let first_joint = app
        .world_mut()
        .spawn((Transform::default(), GlobalTransform::default()))
        .id();
    let owner = app
        .world_mut()
        .spawn((
            Equipment {
                slots: HashMap::from([(EquipmentSlot::Head, helm_fixture().to_path_buf())]),
                ..default()
            },
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .get::<RenderedEquipment>(owner)
            .unwrap()
            .slots
            .is_empty()
    );
    app.world_mut().entity_mut(owner).insert(AttachmentPoints {
        points: HashMap::from([(11, (0, Vec3::ZERO))]),
    });
    app.update();
    assert!(
        app.world()
            .get::<RenderedEquipment>(owner)
            .unwrap()
            .slots
            .is_empty()
    );
    app.world_mut()
        .entity_mut(owner)
        .insert(empty_animation(first_joint));
    app.update();
    let first_item =
        app.world().get::<RenderedEquipment>(owner).unwrap().slots[&EquipmentSlot::Head].entity;
    assert_eq!(
        app.world().get::<ChildOf>(first_item).unwrap().parent(),
        first_joint
    );

    let replacement_joint = app
        .world_mut()
        .spawn((Transform::default(), GlobalTransform::default()))
        .id();
    app.world_mut()
        .entity_mut(owner)
        .insert(empty_animation(replacement_joint));
    app.update();
    let replacement_item =
        app.world().get::<RenderedEquipment>(owner).unwrap().slots[&EquipmentSlot::Head].entity;
    assert_ne!(replacement_item, first_item);
    assert!(app.world().get_entity(first_item).is_err());
    assert_eq!(
        app.world()
            .get::<ChildOf>(replacement_item)
            .unwrap()
            .parent(),
        replacement_joint
    );
}

#[test]
fn empty_stage_equipment_notifications_do_not_require_render_assets() {
    let mut app = App::new();
    app.insert_resource(crate::game::inworld_scene_stage::InWorldSceneStage::Empty);
    register_equipment_observers(&mut app);
    let joint = app.world_mut().spawn_empty().id();
    let owner = app
        .world_mut()
        .spawn((
            Equipment::default(),
            AttachmentPoints {
                points: HashMap::new(),
            },
            empty_animation(joint),
        ))
        .id();
    app.world_mut().trigger(EquipmentChanged { entity: owner });
    app.update();
    assert!(
        app.world()
            .get::<RenderedEquipment>(owner)
            .unwrap()
            .slots
            .is_empty()
    );
    assert!(!app.world().contains_resource::<Assets<Mesh>>());
}
