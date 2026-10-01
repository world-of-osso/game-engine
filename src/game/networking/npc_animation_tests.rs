use super::*;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::ecs::system::RunSystemOnce;
use bevy::state::app::StatesPlugin;
use shared::components::{SheathState, StandState, UnitPose};
use std::time::Duration;

fn animated_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        StatesPlugin,
        bevy::transform::TransformPlugin,
    ));
    app.insert_state(crate::game_state::GameState::M2Debug);
    app.add_plugins(crate::animation::AnimationPlugin);
    app.init_asset::<Mesh>()
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<crate::retail_m2_material::M2Material>()
        .init_asset::<M2EffectMaterial>()
        .init_asset::<SkinnedMeshInverseBindposes>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    app
}

fn spawn_display(app: &mut App, display_id: u32, scale: f32) -> (Entity, Entity) {
    app.world_mut()
        .run_system_once(move |mut commands: Commands, mut assets: NpcSpawnAssets| {
            let npc = commands.spawn(Transform::default()).id();
            let root = spawn_npc_visual_root(&mut commands, npc, scale);
            let mut spawn_assets = crate::m2_spawn::SpawnAssets {
                meshes: &mut assets.meshes,
                materials: &mut assets.materials,
                effect_materials: &mut assets.effect_materials,
                skybox_materials: None,
                images: &mut assets.images,
                inverse_bindposes: &mut assets.inv_bp,
            };
            assert!(try_spawn_npc_model(
                &mut commands,
                &mut spawn_assets,
                root,
                npc,
                Some(&ModelDisplay { display_id }),
                Some(&CreatureDisplayMap),
                scale,
            ));
            (npc, root)
        })
        .unwrap()
}

fn assert_display_idle_moves(app: &mut App, display_id: u32) {
    let (npc, root) = spawn_display(app, display_id, 0.6);
    let owner = assert_animated_hierarchy(app, npc, root);
    assert_idle_bone_motion(app, owner);
}

fn assert_animated_hierarchy(app: &mut App, npc: Entity, root: Entity) -> Entity {
    let owners: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<crate::animation::M2AnimData>>()
        .iter(app.world())
        .collect();
    assert_eq!(
        owners.len(),
        1,
        "NPC spawn must attach its animation runtime"
    );
    let owner = owners[0];
    assert_eq!(app.world().get::<ChildOf>(owner).unwrap().parent(), root);
    assert_eq!(app.world().get::<ChildOf>(root).unwrap().parent(), npc);
    assert!(app.world().get::<crate::camera::Player>(owner).is_none());
    assert!(app.world().get::<crate::camera::Player>(root).is_none());
    assert!(
        app.world()
            .get::<crate::equipment::Equipment>(owner)
            .is_none_or(|equipment| equipment.slots.is_empty())
    );
    let root_transform = app.world().get::<Transform>(root).unwrap();
    assert_eq!(root_transform.scale, Vec3::splat(0.6));
    assert!((root_transform.rotation * Vec3::X - Vec3::Z).length() < 0.0001);
    owner
}

fn assert_idle_bone_motion(app: &mut App, owner: Entity) {
    for _ in 0..4 {
        app.update();
    }
    let joints = app
        .world()
        .get::<crate::animation::M2AnimData>(owner)
        .unwrap()
        .joint_entities
        .clone();
    let before: Vec<_> = joints
        .iter()
        .map(|joint| *app.world().get::<Transform>(*joint).unwrap())
        .collect();
    for _ in 0..3 {
        app.update();
    }
    assert!(
        joints
            .iter()
            .zip(before)
            .any(|(joint, before)| { *app.world().get::<Transform>(*joint).unwrap() != before }),
        "actual Bevy playback must change at least one idle bone pose"
    );
}

#[test]
fn npc_animated_spawn_advances_real_idle_bones_and_preserves_display_skin() {
    let mut app = animated_app();
    // Northshire sheep: nonzero explicit display texture, distinct from humanoid defaults.
    let display_id = 503;
    assert_display_idle_moves(&mut app, display_id);
    let skin = CreatureDisplayMap.get_skin_fdids(display_id).unwrap();
    let model_path =
        crate::asset::asset_cache::model(CreatureDisplayMap.get_fdid(display_id).unwrap()).unwrap();
    let expected = crate::asset::m2::load_m2(&model_path, &skin).unwrap();
    assert!(
        expected
            .batches
            .iter()
            .any(|batch| batch.texture_fdid == Some(skin[0]))
    );
    // An uncomposited M2 texture uploads block-compressed as authored (570d707d).
    let expected_image = crate::asset::blp::load_blp_gpu_material_image(
        &crate::asset::asset_cache::texture(skin[0]).unwrap(),
    )
    .unwrap();
    let images = app.world().resource::<Assets<Image>>();
    assert!(
        images.iter().any(|(_, image)| image.texture_descriptor.size
            == expected_image.texture_descriptor.size
            && image.data == expected_image.data),
        "explicit creature-display skin must reach spawned image assets"
    );
}

#[test]
fn npc_animated_human_hd_spawn_advances_authored_stand_pose() {
    let path = crate::asset::asset_cache::model(1011653).unwrap();
    let model = crate::asset::m2::load_m2(&path, &[0; 3]).unwrap();
    let stand = model
        .sequences
        .iter()
        .position(|sequence| sequence.id == 0)
        .unwrap();
    let moving_tracks = model
        .bone_tracks
        .iter()
        .filter(|track| {
            track
                .translation
                .sequences
                .get(stand)
                .is_some_and(|(_, values)| values.windows(2).any(|pair| pair[0] != pair[1]))
                || track
                    .rotation
                    .sequences
                    .get(stand)
                    .is_some_and(|(_, values)| values.windows(2).any(|pair| pair[0] != pair[1]))
        })
        .count();
    eprintln!(
        "HumanMaleHD: bones={} sequences={} stand_index={} duration={} moving_stand_tracks={} bounds={:?}..{:?}",
        model.bones.len(),
        model.sequences.len(),
        stand,
        model.sequences[stand].duration,
        moving_tracks,
        model.bounding_box_min,
        model.bounding_box_max
    );
    assert!(
        moving_tracks > 0,
        "authored HumanMaleHD Stand tracks must be available"
    );
    assert_display_idle_moves(&mut animated_app(), 3167);
}

#[test]
fn npc_visual_facing_maps_model_forward_to_logical_forward() {
    let mut world = World::new();
    for yaw in [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
        let root = world
            .run_system_once(move |mut commands: Commands| {
                let npc = commands
                    .spawn(Transform::from_rotation(Quat::from_rotation_y(yaw)))
                    .id();
                spawn_npc_visual_root(&mut commands, npc, 1.0)
            })
            .unwrap();
        let local = world.get::<Transform>(root).unwrap();
        let actual = Quat::from_rotation_y(yaw) * local.rotation * Vec3::X;
        let expected = Quat::from_rotation_y(yaw) * Vec3::Z;
        assert!(
            (actual - expected).length() < 0.0001,
            "yaw {yaw}: {actual} != {expected}"
        );
    }
}

fn animated_owners(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<crate::animation::M2AnimData>>()
        .iter(app.world())
        .collect()
}

fn clip_count(app: &App) -> usize {
    app.world()
        .resource::<Assets<bevy::animation::AnimationClip>>()
        .len()
}

/// The Stockade spawns 23 of display 2989: every instance of a model must play the same
/// sequence clips instead of building its own copy of the model's whole animation set.
#[test]
fn npc_instances_of_one_model_share_sequence_clips_until_the_last_despawns() {
    let display_id = 2989;
    let path =
        crate::asset::asset_cache::model(CreatureDisplayMap.get_fdid(display_id).unwrap()).unwrap();
    let skin = CreatureDisplayMap.get_skin_fdids(display_id).unwrap();
    let sequences = crate::asset::m2::load_m2(&path, &skin)
        .unwrap()
        .sequences
        .len();
    let mut app = animated_app();
    let npcs: Vec<_> = (0..3)
        .map(|_| spawn_display(&mut app, display_id, 1.0).0)
        .collect();
    app.update();
    let owners = animated_owners(&mut app);
    assert_eq!(owners.len(), 3);
    // One clip per authored sequence, plus each instance's own blend snapshot clip.
    assert_eq!(clip_count(&app), sequences + owners.len());
    for owner in owners {
        assert_idle_bone_motion(&mut app, owner);
    }

    app.world_mut().entity_mut(npcs[0]).despawn();
    app.update();
    app.update();
    assert_eq!(clip_count(&app), sequences + 2);
    for npc in &npcs[1..] {
        app.world_mut().entity_mut(*npc).despawn();
    }
    app.update();
    app.update();
    assert_eq!(
        clip_count(&app),
        0,
        "clips must not outlive the model's last instance"
    );
}

fn playing_id(app: &App, owner: Entity) -> u16 {
    let player = app
        .world()
        .get::<crate::animation::M2AnimPlayer>(owner)
        .unwrap();
    let data = app
        .world()
        .get::<crate::animation::M2AnimData>(owner)
        .unwrap();
    data.sequences[player.current_seq_idx].id
}

/// Riverpaw Slayer (display 384, the Stockade gnoll model 3886641; the hierarchy check
/// expects a 0.6 root scale) plays the animation of
/// its replicated `CreatureMotion`: Run (5) chasing, Walk (4) wandering, Stand (0) stopped.
#[test]
fn riverpaw_plays_run_walk_and_stand_from_its_replicated_motion() {
    let mut app = animated_app();
    app.add_systems(Update, sync_npc_motion_animation);
    let (npc, root) = spawn_display(&mut app, 384, 0.6);
    let owner = assert_animated_hierarchy(&mut app, npc, root);
    for (motion, anim_id) in [
        (CreatureMotion::Still, 0),
        (CreatureMotion::Run, 5),
        (CreatureMotion::Walk, 4),
        (CreatureMotion::Still, 0),
    ] {
        app.world_mut().entity_mut(npc).insert(motion);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(playing_id(&app, owner), anim_id, "{motion:?}");
    }
}

fn stockade_npc(template_id: u32, name: &str) -> Npc {
    Npc {
        template_id,
        name: name.into(),
    }
}

fn pose(stand_state: StandState, sheath_state: SheathState, emote_state: u32) -> UnitPose {
    UnitPose {
        stand_state,
        sheath_state,
        emote_state,
    }
}

/// The Stockade (map 34) TDB addons on the Retail models: Stockade Guard 375782 holds
/// Ready1H (26, emote 333) until it walks, Petty Criminal 46382 sleeps (100) and spawn
/// 375707 sits (97).
#[test]
fn stockade_guard_and_criminals_hold_their_authored_poses() {
    let mut app = animated_app();
    app.add_systems(
        Update,
        (sync_npc_motion_animation, npc_gear::sync_npc_pose_animation),
    );
    let (guard, guard_root) = spawn_display(&mut app, 2989, 1.0);
    let (criminal, criminal_root) = spawn_display(&mut app, 35069, 1.0);
    app.update();
    let owner = |app: &App, root: Entity| {
        app.world()
            .get::<Children>(root)
            .unwrap()
            .iter()
            .find(|child| {
                app.world()
                    .get::<crate::animation::M2AnimData>(*child)
                    .is_some()
            })
            .unwrap()
    };
    let (guard_model, criminal_model) = (owner(&app, guard_root), owner(&app, criminal_root));
    app.world_mut().entity_mut(guard).insert((
        stockade_npc(46405, "Stockade Guard"),
        pose(StandState::Stand, SheathState::Melee, 333),
        CreatureMotion::Still,
    ));
    app.world_mut().entity_mut(criminal).insert((
        stockade_npc(46382, "Petty Criminal"),
        pose(StandState::Sleep, SheathState::Melee, 0),
        CreatureMotion::Still,
    ));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(playing_id(&app, guard_model), 26);
    assert_eq!(playing_id(&app, criminal_model), 100);

    app.world_mut()
        .entity_mut(guard)
        .insert(CreatureMotion::Walk);
    app.world_mut()
        .entity_mut(criminal)
        .insert(pose(StandState::Sit, SheathState::Melee, 0));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        playing_id(&app, guard_model),
        4,
        "walking ends the ready stance"
    );
    assert_eq!(playing_id(&app, criminal_model), 97);

    app.world_mut()
        .entity_mut(guard)
        .insert(CreatureMotion::Still);
    app.world_mut()
        .entity_mut(criminal)
        .insert(pose(StandState::Stand, SheathState::Melee, 0));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(playing_id(&app, guard_model), 26);
    assert_eq!(playing_id(&app, criminal_model), 0);
}

/// Stockade Guard 46405 equipment 1 as the server replicates it: sword 5305
/// (ItemDisplayInfo 7526) and shield 1984 (1705). Drawn (SheathState 1) the sword sits
/// in the right palm (attachment 1) and the shield on the left wrist (0); sheathed the
/// sword hangs at the left hip (32) and the shield on the back (28).
#[test]
fn stockade_guard_draws_and_sheathes_sword_and_shield() {
    use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};
    let mut app = animated_app();
    app.insert_resource(game_engine::outfit_data::OutfitData::load(
        std::path::Path::new("data"),
    ));
    app.add_plugins(crate::equipment::EquipmentPlugin);
    app.add_systems(Update, npc_gear::sync_npc_equipment);
    let (guard, root) = spawn_display(&mut app, 2989, 1.0);
    app.update();
    let model = app
        .world()
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|child| {
            app.world()
                .get::<crate::animation::M2AnimData>(*child)
                .is_some()
        })
        .unwrap();
    let item = |slot, item_id, display_info_id, inventory_type| EquippedAppearanceEntry {
        slot,
        item_id: Some(item_id),
        display_info_id: Some(display_info_id),
        inventory_type,
        hidden: false,
    };
    let armor = npc_gear::resolve_display_armor(
        2989,
        1,
        0,
        app.world()
            .resource::<game_engine::outfit_data::OutfitData>(),
    );
    app.world_mut().entity_mut(guard).insert((
        npc_gear::NpcGear::new(1, 0, &armor),
        shared::components::EquipmentAppearance {
            entries: vec![
                item(EquipmentVisualSlot::MainHand, 5305, 7526, 13),
                item(EquipmentVisualSlot::OffHand, 1984, 1705, 14),
            ],
        },
        pose(StandState::Stand, SheathState::Melee, 333),
    ));
    for (sheath, main_hand, off_hand) in
        [(SheathState::Melee, 1, 0), (SheathState::Unarmed, 32, 28)]
    {
        app.world_mut()
            .entity_mut(guard)
            .insert(pose(StandState::Stand, sheath, 333));
        for _ in 0..3 {
            app.update();
        }
        let points = &app
            .world()
            .get::<crate::equipment::AttachmentPoints>(model)
            .unwrap()
            .points;
        let joints = &app
            .world()
            .get::<crate::animation::M2AnimData>(model)
            .unwrap()
            .joint_entities;
        let joint_of = |attachment: u32| joints[points[&attachment].0 as usize];
        let mut expected = vec![joint_of(main_hand), joint_of(off_hand)];
        let mut items: Vec<_> = app
            .world_mut()
            .query::<(&crate::equipment::EquipmentItem, &ChildOf)>()
            .iter(app.world())
            .map(|(_, parent)| parent.parent())
            .collect();
        items.sort();
        expected.sort();
        assert_eq!(items, expected, "{sheath:?}");
    }
}
