#[cfg(test)]
#[path = "../../../ui/screens/menu_character_layout_test_support.rs"]
mod native_layout_support;

use std::collections::HashSet;

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::screens::world_builder_component::{
    WORLD_BUILDER_APPLY_TRANSFORM, WorldBuilderAction, WorldBuilderViewState,
    world_builder_row_name,
};
use native_layout_support::compute_layout as recompute_layouts;

use crate::{
    asset::m2_anim::AnimTrack,
    m2_effect_material::{M2EffectMaterial, M2EffectSettings},
};

use super::*;

#[derive(Component)]
struct TickCount(u32);

#[test]
fn snapshot_builds_scene_forest_and_keeps_disabled_entities() {
    let mut world = World::new();
    let root = world
        .spawn((Name::new("Root"), Transform::default(), Visibility::Visible))
        .id();
    let child = world
        .spawn((
            Name::new("Child"),
            Transform::from_xyz(1.0, 2.0, 3.0),
            Visibility::Inherited,
            Disabled,
        ))
        .id();
    world.entity_mut(root).add_child(child);

    let snapshot = build_scene_snapshot(&world);

    assert_eq!(snapshot.roots, vec![root]);
    assert_eq!(snapshot.entities.len(), 2);
    let child_node = snapshot.entities.get(&child).expect("disabled child");
    assert_eq!(child_node.parent, Some(root));
    assert_eq!(child_node.translation, Vec3::new(1.0, 2.0, 3.0));
    assert!(
        child_node
            .component_names
            .iter()
            .any(|name| name.ends_with("Disabled"))
    );
}

#[test]
fn filtered_rows_include_matching_entity_ancestors() {
    let mut world = World::new();
    let root = world.spawn((Name::new("Root"), Transform::default())).id();
    let child = world
        .spawn((Name::new("Needle Child"), Transform::default()))
        .id();
    let other = world
        .spawn((Name::new("Other Root"), Transform::default()))
        .id();
    world.entity_mut(root).add_child(child);
    let snapshot = build_scene_snapshot(&world);

    let rows = visible_scene_rows(&snapshot, &HashSet::new(), "needle");

    assert_eq!(
        rows.iter().map(|row| row.entity).collect::<Vec<_>>(),
        vec![root, child]
    );
    assert_eq!(
        rows.iter().map(|row| row.depth).collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(!rows.iter().any(|row| row.entity == other));
}

#[test]
fn render_suppression_hides_and_restores_subtree_visibility() {
    let mut world = World::new();
    let root = world.spawn(Visibility::Visible).id();
    let child = world.spawn(Visibility::Inherited).id();
    world.entity_mut(root).add_child(child);

    reconcile_render_suppression(&mut world, &HashSet::from([root]));
    assert_eq!(world.get::<Visibility>(root), Some(&Visibility::Hidden));
    assert_eq!(world.get::<Visibility>(child), Some(&Visibility::Hidden));

    reconcile_render_suppression(&mut world, &HashSet::new());
    assert_eq!(world.get::<Visibility>(root), Some(&Visibility::Visible));
    assert_eq!(world.get::<Visibility>(child), Some(&Visibility::Inherited));
}

#[test]
fn processing_suspension_stops_default_query_systems_and_resumes() {
    let mut app = App::new();
    app.add_systems(Update, |mut counts: Query<&mut TickCount>| {
        for mut count in &mut counts {
            count.0 += 1;
        }
    });
    let root = app.world_mut().spawn(TickCount(0)).id();
    let child = app.world_mut().spawn(TickCount(0)).id();
    app.world_mut().entity_mut(root).add_child(child);

    app.update();
    reconcile_processing_suspension(app.world_mut(), &HashSet::from([root]));
    app.update();
    assert_eq!(app.world().get::<TickCount>(root).expect("root").0, 1);
    assert_eq!(app.world().get::<TickCount>(child).expect("child").0, 1);

    reconcile_processing_suspension(app.world_mut(), &HashSet::new());
    app.update();
    assert_eq!(app.world().get::<TickCount>(root).expect("root").0, 2);
    assert_eq!(app.world().get::<TickCount>(child).expect("child").0, 2);
}

#[test]
fn model_actions_drive_bulk_root_suppression_and_paged_view_state() {
    let mut world = World::new();
    let root = world.spawn((Name::new("Root"), Transform::IDENTITY)).id();
    let child = world
        .spawn((
            Name::new("Child"),
            Transform::from_xyz(1.0, 2.0, 3.0),
            PointLight {
                intensity: 1200.0,
                range: 8.0,
                ..default()
            },
        ))
        .id();
    world.entity_mut(root).add_child(child);

    let mut model = WorldBuilderModel {
        snapshot: build_scene_snapshot(&world),
        selected: Some(child),
        ..default()
    };
    apply_model_action(&mut model, WorldBuilderAction::RenderOffRoots);
    apply_model_action(&mut model, WorldBuilderAction::ProcessOffRoots);
    model.expanded.insert(root);

    assert_eq!(model.render_roots, HashSet::from([root]));
    assert_eq!(model.processing_roots, HashSet::from([root]));

    let view: WorldBuilderViewState = build_view_state(&model);
    assert_eq!(view.total_entities, 2);
    assert_eq!(view.rows.len(), 2);
    let child_row = view
        .rows
        .iter()
        .find(|row| row.entity_bits == child.to_bits())
        .expect("child row");
    assert!(child_row.render_hidden);
    assert!(child_row.processing_suspended);
    let selected = view.selected.expect("selected properties");
    assert_eq!(selected.selected_id, child.to_bits());
    assert_eq!(selected.translation, ["1.000", "2.000", "3.000"]);
    assert_eq!(
        selected.point_light.expect("point light").intensity,
        "1200.000"
    );

    apply_model_action(&mut model, WorldBuilderAction::EnableAll);
    assert!(model.render_roots.is_empty());
    assert!(model.processing_roots.is_empty());
}

#[test]
fn light_edits_reject_invalid_values_without_mutation() {
    let mut world = World::new();
    let point = world
        .spawn(PointLight {
            intensity: 1000.0,
            range: 10.0,
            shadow_maps_enabled: false,
            ..default()
        })
        .id();
    let directional = world
        .spawn(DirectionalLight {
            illuminance: 5000.0,
            shadow_maps_enabled: false,
            ..default()
        })
        .id();

    apply_point_light_edit(
        &mut world,
        point,
        PointLightEdit {
            intensity: 2000.0,
            range: 25.0,
        },
    )
    .expect("valid point light");
    apply_directional_light_edit(&mut world, directional, 9000.0).expect("valid directional light");
    assert_eq!(world.get::<PointLight>(point).expect("point").range, 25.0);
    assert_eq!(
        world
            .get::<DirectionalLight>(directional)
            .expect("directional")
            .illuminance,
        9000.0
    );

    let before = world.get::<PointLight>(point).expect("point").clone();
    assert!(
        apply_point_light_edit(
            &mut world,
            point,
            PointLightEdit {
                intensity: f32::NAN,
                range: -1.0,
            },
        )
        .is_err()
    );
    assert_eq!(
        world.get::<PointLight>(point).expect("point").range,
        before.range
    );
    assert_eq!(
        world.get::<PointLight>(point).expect("point").intensity,
        before.intensity
    );
}

#[test]
fn processing_suspension_skips_only_fully_suspended_m2_materials() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(WorldBuilderM2UvUpdatesEnabled(true));
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.init_resource::<WorldBuilderSuppressedM2Materials>();
    app.add_systems(Update, update_world_builder_m2_effect_uvs);

    let fully_suspended_material = app
        .world_mut()
        .resource_mut::<Assets<M2EffectMaterial>>()
        .add(animated_test_material());
    let shared_material = app
        .world_mut()
        .resource_mut::<Assets<M2EffectMaterial>>()
        .add(animated_test_material());
    let fully_suspended = app
        .world_mut()
        .spawn((
            Transform::IDENTITY,
            MeshMaterial3d(fully_suspended_material.clone()),
        ))
        .id();
    let suspended_shared_user = app
        .world_mut()
        .spawn((Transform::IDENTITY, MeshMaterial3d(shared_material.clone())))
        .id();
    app.world_mut()
        .spawn((Transform::IDENTITY, MeshMaterial3d(shared_material.clone())));

    reconcile_processing_suspension(
        app.world_mut(),
        &HashSet::from([fully_suspended, suspended_shared_user]),
    );
    rebuild_suppressed_m2_materials(app.world_mut());
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(500));
    app.update();

    let materials = app.world().resource::<Assets<M2EffectMaterial>>();
    assert_eq!(
        materials
            .get(&fully_suspended_material)
            .expect("fully suspended material")
            .settings
            .uv_offset_1,
        Vec2::new(9.0, 10.0)
    );
    assert_eq!(
        materials
            .get(&shared_material)
            .expect("shared material with an active user")
            .settings
            .uv_offset_1,
        Vec2::new(0.5, 0.75)
    );
}

#[test]
fn sidebar_property_fields_apply_transform_to_selected_entity() {
    let mut world = World::new();
    let entity = world
        .spawn((Name::new("Editable"), Transform::IDENTITY))
        .id();
    let model = WorldBuilderModel {
        snapshot: build_scene_snapshot(&world),
        selected: Some(entity),
        ..default()
    };
    let mut registry = game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(build_view_state(&model));
    Screen::new(world_builder_screen).sync(&shared, &mut registry);
    set_test_editbox_text(
        &mut registry,
        &world_builder_transform_edit_name(entity.to_bits(), 0),
        "4.5",
    );

    let action = build_world_action(
        WorldBuilderAction::ApplyTransform(entity.to_bits()),
        &registry,
    )
    .expect("valid fields")
    .expect("world action");
    apply_world_action(&mut world, action).expect("apply transform");

    assert_eq!(
        world
            .get::<Transform>(entity)
            .expect("transform")
            .translation,
        Vec3::new(4.5, 0.0, 0.0)
    );
}

fn set_test_editbox_text(
    registry: &mut game_engine::ui::registry::FrameRegistry,
    name: &str,
    text: &str,
) {
    let id = registry.get_by_name(name).expect(name);
    let Some(WidgetData::EditBox(editbox)) = registry
        .get_mut(id)
        .and_then(|frame| frame.widget_data.as_mut())
    else {
        panic!("{name} is not an EditBox");
    };
    editbox.replace_range(0, editbox.text.len(), text);
}

#[test]
fn scheduled_pointer_and_keyboard_edit_applies_transform() {
    let mut app = world_builder_interaction_app();
    let editable = app
        .world_mut()
        .spawn((Name::new("Editable"), Transform::IDENTITY))
        .id();
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1920, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();

    app.update();
    click_world_builder_frame(
        &mut app,
        window,
        &world_builder_row_name(editable.to_bits()),
    );
    let editbox_name = world_builder_transform_edit_name(editable.to_bits(), 0);
    click_world_builder_frame(&mut app, window, &editbox_name);
    let focused_value = {
        let ui = app.world().resource::<game_engine::ui::plugin::UiState>();
        let id = ui.registry.get_by_name(&editbox_name).expect("editbox");
        assert_eq!(ui.focused_frame, Some(id));
        editbox_text(&ui.registry, id).expect("editbox text")
    };
    assert_eq!(focused_value, "");

    type_world_builder_text(&mut app, "4.5");
    let editbox_value = {
        let ui = app.world().resource::<game_engine::ui::plugin::UiState>();
        let id = ui.registry.get_by_name(&editbox_name).expect("editbox");
        editbox_text(&ui.registry, id).expect("editbox text")
    };
    assert_eq!(editbox_value, "4.5");
    click_world_builder_frame(&mut app, window, WORLD_BUILDER_APPLY_TRANSFORM.0);

    assert_eq!(
        app.world()
            .get::<Transform>(editable)
            .expect("editable transform")
            .translation,
        Vec3::new(4.5, 0.0, 0.0)
    );
}

fn world_builder_interaction_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.add_message::<KeyboardInput>();
    app.init_state::<crate::game_state::GameState>();
    app.insert_state(crate::game_state::GameState::InWorld);
    app.insert_resource(game_engine::ui::plugin::UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.add_plugins(WorldBuilderPlugin);
    app
}

fn click_world_builder_frame(app: &mut App, window: Entity, name: &str) {
    let center = {
        let mut ui = app
            .world_mut()
            .resource_mut::<game_engine::ui::plugin::UiState>();
        recompute_layouts(&mut ui.registry);
        let id = ui.registry.get_by_name(name).expect(name);
        let rect = ui
            .registry
            .get(id)
            .and_then(|frame| frame.layout_rect.as_ref())
            .expect("frame layout");
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    };
    app.world_mut()
        .entity_mut(window)
        .get_mut::<Window>()
        .expect("window")
        .set_cursor_position(Some(center));
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.release(MouseButton::Left);
    buttons.press(MouseButton::Left);
    drop(buttons);
    app.update();
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.release(MouseButton::Left);
    buttons.clear();
}

fn type_world_builder_text(app: &mut App, text: &str) {
    for character in text.chars() {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code: KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified),
                logical_key: bevy::input::keyboard::Key::Character(character.to_string().into()),
                state: ButtonState::Pressed,
                text: Some(character.to_string().into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
    }
    app.update();
}

#[test]
fn scheduled_filter_typing_rebuilds_visible_rows() {
    let mut app = world_builder_interaction_app();
    let editable = app
        .world_mut()
        .spawn((Name::new("Editable"), Transform::IDENTITY))
        .id();
    let other = app
        .world_mut()
        .spawn((Name::new("Other"), Transform::IDENTITY))
        .id();
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1920, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();

    app.update();
    click_world_builder_frame(&mut app, window, WORLD_BUILDER_FILTER.0);
    type_world_builder_text(&mut app, "other");

    assert_eq!(app.world().resource::<WorldBuilderModel>().filter, "other");
    let registry = &app
        .world()
        .resource::<game_engine::ui::plugin::UiState>()
        .registry;
    assert!(
        registry
            .get_by_name(&world_builder_row_name(other.to_bits()))
            .is_some()
    );
    assert!(
        registry
            .get_by_name(&world_builder_row_name(editable.to_bits()))
            .is_none()
    );
}

#[test]
fn f9_toggles_the_world_builder_panel() {
    let mut app = world_builder_interaction_app();
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F9);

    app.update();

    assert!(!app.world().resource::<WorldBuilderModel>().open);
    let ui = app.world().resource::<game_engine::ui::plugin::UiState>();
    let root = ui
        .registry
        .get_by_name(WORLD_BUILDER_ROOT.0)
        .and_then(|id| ui.registry.get(id))
        .expect("World Builder root");
    assert!(root.hidden);
}

#[test]
fn plugin_mounts_sidebar_on_inworld_entry_and_removes_it_on_exit() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        bevy::input::InputPlugin,
    ));
    app.init_state::<crate::game_state::GameState>();
    app.insert_state(crate::game_state::GameState::InWorld);
    app.insert_resource(game_engine::ui::plugin::UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.add_plugins(WorldBuilderPlugin);

    app.update();
    assert!(
        app.world()
            .resource::<game_engine::ui::plugin::UiState>()
            .registry
            .get_by_name(game_engine::ui::screens::world_builder_component::WORLD_BUILDER_ROOT.0)
            .is_some()
    );

    app.world_mut()
        .resource_mut::<NextState<crate::game_state::GameState>>()
        .set(crate::game_state::GameState::Login);
    app.update();
    assert!(
        app.world()
            .resource::<game_engine::ui::plugin::UiState>()
            .registry
            .get_by_name(game_engine::ui::screens::world_builder_component::WORLD_BUILDER_ROOT.0)
            .is_none()
    );
}

fn animated_test_material() -> M2EffectMaterial {
    M2EffectMaterial {
        settings: M2EffectSettings {
            transparency: 1.0,
            alpha_test: 0.0,
            shader_id: 0,
            blend_mode: 0,
            uv_mode_1: 0,
            uv_mode_2: 0,
            render_flags: 0,
            uv_offset_1: Vec2::new(9.0, 10.0),
            uv_offset_2: Vec2::new(11.0, 12.0),
        },
        base_texture: Handle::default(),
        second_texture: Handle::default(),
        blend_mode: 0,
        two_sided: false,
        texture_anim_1: Some(AnimTrack {
            interpolation_type: 0,
            global_sequence: -1,
            sequences: vec![(vec![0, 1000], vec![[0.0, 0.0, 0.0], [1.0, 1.5, 0.0]])],
        }),
        texture_anim_2: None,
    }
}

#[test]
fn transform_edit_rejects_invalid_values_without_mutation() {
    let mut world = World::new();
    let entity = world.spawn(Transform::IDENTITY).id();
    let valid = TransformEdit {
        translation: Vec3::new(4.0, 5.0, 6.0),
        rotation_degrees: Vec3::new(0.0, 90.0, 0.0),
        scale: Vec3::splat(2.0),
    };

    apply_transform_edit(&mut world, entity, valid).expect("valid transform");
    let applied = world.get::<Transform>(entity).expect("transform");
    assert_eq!(applied.translation, valid.translation);
    assert_eq!(applied.scale, valid.scale);

    let before = *applied;
    let invalid = TransformEdit {
        scale: Vec3::ZERO,
        ..valid
    };
    assert!(apply_transform_edit(&mut world, entity, invalid).is_err());
    assert_eq!(world.get::<Transform>(entity), Some(&before));
}
