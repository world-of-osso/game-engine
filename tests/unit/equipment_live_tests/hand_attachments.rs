use super::*;

#[test]
fn live_weapons_follow_authored_palms_and_shield_wrist() {
    let character = Path::new("data/models/humanmale_hd.m2")
        .canonicalize()
        .unwrap();
    let sword = Path::new("data/item-models/item/objectcomponents/weapon/sword_1h_short_a_02.m2");
    let shield = Path::new("data/item-models/item/objectcomponents/shield/shield_round_a_01.m2");
    assert!(
        sword.exists() && shield.exists(),
        "starter weapon fixtures must exist"
    );
    let mut app = game_engine::test_harness::headless_app_with(configure_live_test_app);
    let spawned = spawn_live_character(&mut app, &character);
    // WMVx AttachmentPosition: right palm=1, left palm=2, shield wrist=0.
    for (slot, path, attachment_id) in [
        (EquipmentSlot::MainHand, sword, 1),
        (EquipmentSlot::OffHand, sword, 2),
        (EquipmentSlot::OffHand, shield, 0),
    ] {
        {
            let mut equipment = app
                .world_mut()
                .get_mut::<Equipment>(spawned.model_root)
                .unwrap();
            equipment.slots.clear();
            equipment.slots.insert(slot, path.to_path_buf());
        }
        app.world_mut().trigger(EquipmentChanged {
            entity: spawned.model_root,
        });
        app.update();
        app.update();
        assert_item_follows_attachment(&mut app, spawned.model_root, slot, attachment_id);
    }
}

fn assert_item_follows_attachment(
    app: &mut App,
    owner: Entity,
    slot: EquipmentSlot,
    attachment_id: u32,
) {
    let points = app.world().get::<AttachmentPoints>(owner).unwrap();
    let &(bone, offset) = points.points.get(&attachment_id).unwrap();
    let joint = app.world().get::<M2AnimData>(owner).unwrap().joint_entities[bone as usize];
    let expected = app
        .world()
        .get::<GlobalTransform>(joint)
        .unwrap()
        .mul_transform(Transform::from_translation(offset));
    let item = app
        .world_mut()
        .query::<(Entity, &EquipmentItem)>()
        .iter(app.world())
        .find(|(_, item)| item._slot == slot)
        .unwrap()
        .0;
    let actual = app.world().get::<GlobalTransform>(item).unwrap();
    assert!(
        actual.translation().distance(expected.translation()) < 0.0001,
        "{slot:?} model origin must follow attachment {attachment_id}; actual={:?}, expected={:?}",
        actual.translation(),
        expected.translation()
    );
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let actual_axis = actual.affine().transform_vector3(axis);
        let expected_axis = expected.affine().transform_vector3(axis);
        assert!(
            actual_axis.distance(expected_axis) < 0.0001,
            "{slot:?} authored model axis {axis:?} must follow attachment {attachment_id}: {actual_axis:?} != {expected_axis:?}"
        );
    }
}
