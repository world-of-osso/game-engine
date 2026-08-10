use super::*;

pub(super) fn build_world_builder_ui(world: &mut World) {
    if world
        .resource::<UiState>()
        .registry
        .get_by_name(WORLD_BUILDER_ROOT.0)
        .is_some()
    {
        return;
    }
    let view_state = build_view_state(world.resource::<WorldBuilderModel>());
    let mut shared = SharedContext::new();
    shared.insert(view_state);
    let mut screen = Screen::new(world_builder_screen);
    screen.sync(&shared, &mut world.resource_mut::<UiState>().registry);
    world.insert_non_send(WorldBuilderScreenWrap(WorldBuilderScreenRes {
        screen,
        shared,
    }));
}

pub(super) fn toggle_world_builder_hotkey(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut model: ResMut<WorldBuilderModel>,
) {
    let Some(keys) = keys else { return };
    if !keys.just_pressed(KeyCode::F9) {
        return;
    }
    model.open = !model.open;
    if model.open {
        model.snapshot_dirty = true;
        model.status = "World Builder opened. Refresh before recording FPS.".to_string();
    }
}

pub(super) fn world_builder_mouse_input(
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    model: Res<WorldBuilderModel>,
    mut events: MessageWriter<WorldBuilderClickEvent>,
) {
    let Some(buttons) = buttons else { return };
    if !model.open || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows.iter().find_map(Window::cursor_position) else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    if !is_world_builder_frame(&ui.registry, frame_id) {
        return;
    }
    if ui
        .registry
        .get(frame_id)
        .is_some_and(|frame| frame.is_editbox())
    {
        let clear_property_value = should_clear_property_editbox(&ui, frame_id);
        ui.registry.click_frame(frame_id);
        if clear_property_value {
            clear_editbox_text(&mut ui.registry, frame_id);
        }
        ui.focused_frame = ui.registry.focused_frame;
        return;
    }
    ui.focused_frame = None;
    ui.registry.focused_frame = None;
    if let Some(action) = crate::ui_input::walk_up_for_onclick(&ui.registry, frame_id) {
        events.write(WorldBuilderClickEvent(action));
    }
}

fn should_clear_property_editbox(ui: &UiState, frame_id: u64) -> bool {
    let already_focused = ui.focused_frame.or(ui.registry.focused_frame) == Some(frame_id);
    if already_focused {
        return false;
    }
    ui.registry
        .get(frame_id)
        .and_then(|frame| frame.name.as_deref())
        .is_some_and(is_property_editbox_name)
}

fn is_property_editbox_name(name: &str) -> bool {
    const PREFIXES: [&str; 5] = [
        "WorldBuilderTransform_",
        "WorldBuilderRotation_",
        "WorldBuilderScale_",
        "WorldBuilderPointLight_",
        "WorldBuilderDirectionalLight_",
    ];
    PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

fn clear_editbox_text(registry: &mut game_engine::ui::registry::FrameRegistry, frame_id: u64) {
    let Some(WidgetData::EditBox(editbox)) = registry
        .get_mut(frame_id)
        .and_then(|frame| frame.widget_data.as_mut())
    else {
        return;
    };
    editbox.replace_range(0, editbox.text.len(), "");
}

pub(super) fn is_world_builder_frame(
    registry: &game_engine::ui::registry::FrameRegistry,
    mut id: u64,
) -> bool {
    let Some(root_id) = registry.get_by_name(WORLD_BUILDER_ROOT.0) else {
        return false;
    };
    loop {
        if id == root_id {
            return true;
        }
        let Some(parent_id) = registry.get(id).and_then(|frame| frame.parent_id) else {
            return false;
        };
        id = parent_id;
    }
}

pub(super) fn world_builder_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    mut model: ResMut<WorldBuilderModel>,
) {
    if !model.open {
        return;
    }
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let Some(focused_id) = ui.focused_frame.or(ui.registry.focused_frame) else {
            continue;
        };
        if !is_world_builder_frame(&ui.registry, focused_id) {
            continue;
        }
        if event.key_code == KeyCode::Escape || event.key_code == KeyCode::Enter {
            clear_world_builder_focus(&mut ui);
            continue;
        }
        let editbox_changed = mutate_editbox_from_key(&mut ui.registry, focused_id, event);
        let filter_changed = ui.registry.get_by_name(WORLD_BUILDER_FILTER.0) == Some(focused_id);
        if editbox_changed && filter_changed {
            sync_filter_from_editbox(&ui.registry, focused_id, &mut model);
        }
    }
}

pub(super) fn clear_world_builder_focus(ui: &mut UiState) {
    ui.focused_frame = None;
    ui.registry.focused_frame = None;
}

pub(super) fn mutate_editbox_from_key(
    registry: &mut game_engine::ui::registry::FrameRegistry,
    focused_id: u64,
    event: &KeyboardInput,
) -> bool {
    let Some(WidgetData::EditBox(editbox)) = registry
        .get_mut(focused_id)
        .and_then(|frame| frame.widget_data.as_mut())
    else {
        return false;
    };
    match event.key_code {
        KeyCode::Backspace => editbox.backspace(),
        KeyCode::Delete => editbox.delete_forward(),
        KeyCode::ArrowLeft => editbox.cursor_left(),
        KeyCode::ArrowRight => editbox.cursor_right(),
        KeyCode::Home => editbox.cursor_home(),
        KeyCode::End => editbox.cursor_end(),
        _ => {
            let Some(text) = event.text.as_ref() else {
                return false;
            };
            editbox.insert_at_cursor(text.as_str());
        }
    }
    true
}

pub(super) fn sync_filter_from_editbox(
    registry: &game_engine::ui::registry::FrameRegistry,
    focused_id: u64,
    model: &mut WorldBuilderModel,
) {
    if registry.get_by_name(WORLD_BUILDER_FILTER.0) != Some(focused_id) {
        return;
    }
    let Some(text) = editbox_text(registry, focused_id) else {
        return;
    };
    model.filter = text;
    model.page_index = 0;
}

pub(super) fn dispatch_world_builder_actions(
    mut events: MessageReader<WorldBuilderClickEvent>,
    mut model: ResMut<WorldBuilderModel>,
    ui: Res<UiState>,
    mut world_actions: ResMut<WorldBuilderWorldActionQueue>,
) {
    for event in events.read() {
        let Some(action) = WorldBuilderAction::parse(&event.0) else {
            continue;
        };
        match build_world_action(action, &ui.registry) {
            Ok(Some(action)) => world_actions.0.push(action),
            Ok(None) => apply_model_action(&mut model, action),
            Err(error) => model.status = error,
        }
    }
}

pub(super) fn build_world_action(
    action: WorldBuilderAction,
    registry: &game_engine::ui::registry::FrameRegistry,
) -> Result<Option<WorldBuilderWorldAction>, String> {
    let action = match action {
        WorldBuilderAction::ApplyTransform(bits) => WorldBuilderWorldAction::ApplyTransform(
            entity_from_bits(bits)?,
            transform_edit_from_registry(registry, bits)?,
        ),
        WorldBuilderAction::ApplyPointLight(bits) => WorldBuilderWorldAction::ApplyPointLight(
            entity_from_bits(bits)?,
            point_light_edit_from_registry(registry, bits)?,
        ),
        WorldBuilderAction::ApplyDirectionalLight(bits) => {
            WorldBuilderWorldAction::ApplyDirectionalLight(
                entity_from_bits(bits)?,
                directional_light_edit_from_registry(registry, bits)?,
            )
        }
        WorldBuilderAction::TogglePointLightShadows(bits) => {
            WorldBuilderWorldAction::TogglePointLightShadows(entity_from_bits(bits)?)
        }
        WorldBuilderAction::ToggleDirectionalLightShadows(bits) => {
            WorldBuilderWorldAction::ToggleDirectionalLightShadows(entity_from_bits(bits)?)
        }
        _ => return Ok(None),
    };
    Ok(Some(action))
}

pub(super) fn entity_from_bits(bits: u64) -> Result<Entity, String> {
    Entity::try_from_bits(bits).ok_or_else(|| format!("invalid entity id {bits}"))
}

pub(super) fn transform_edit_from_registry(
    registry: &game_engine::ui::registry::FrameRegistry,
    bits: u64,
) -> Result<TransformEdit, String> {
    Ok(TransformEdit {
        translation: read_vec3(registry, bits, world_builder_transform_edit_name)?,
        rotation_degrees: read_vec3(registry, bits, world_builder_rotation_edit_name)?,
        scale: read_vec3(registry, bits, world_builder_scale_edit_name)?,
    })
}

pub(super) fn read_vec3(
    registry: &game_engine::ui::registry::FrameRegistry,
    bits: u64,
    field_name: fn(u64, usize) -> String,
) -> Result<Vec3, String> {
    Ok(Vec3::new(
        read_named_float(registry, &field_name(bits, 0))?,
        read_named_float(registry, &field_name(bits, 1))?,
        read_named_float(registry, &field_name(bits, 2))?,
    ))
}

pub(super) fn point_light_edit_from_registry(
    registry: &game_engine::ui::registry::FrameRegistry,
    bits: u64,
) -> Result<PointLightEdit, String> {
    Ok(PointLightEdit {
        intensity: read_named_float(registry, &world_builder_point_light_edit_name(bits, 0))?,
        range: read_named_float(registry, &world_builder_point_light_edit_name(bits, 1))?,
    })
}

pub(super) fn directional_light_edit_from_registry(
    registry: &game_engine::ui::registry::FrameRegistry,
    bits: u64,
) -> Result<f32, String> {
    read_named_float(registry, &world_builder_directional_light_edit_name(bits))
}

pub(super) fn read_named_float(
    registry: &game_engine::ui::registry::FrameRegistry,
    name: &str,
) -> Result<f32, String> {
    let id = registry
        .get_by_name(name)
        .ok_or_else(|| format!("missing property field {name}"))?;
    let text = editbox_text(registry, id).ok_or_else(|| format!("{name} is not an EditBox"))?;
    text.trim()
        .parse::<f32>()
        .map_err(|error| format!("invalid value in {name}: {error}"))
}

pub(super) fn editbox_text(
    registry: &game_engine::ui::registry::FrameRegistry,
    id: u64,
) -> Option<String> {
    registry.get(id).and_then(|frame| match &frame.widget_data {
        Some(WidgetData::EditBox(editbox)) => Some(editbox.text.clone()),
        _ => None,
    })
}

pub(super) fn process_world_builder_world(world: &mut World) {
    apply_pending_world_actions(world);
    reconcile_requested_overrides(world);
    refresh_snapshot_if_needed(world);
}

pub(super) fn update_world_builder_m2_effect_uvs(
    time: Option<Res<Time>>,
    enabled: Res<WorldBuilderM2UvUpdatesEnabled>,
    suppressed: Res<WorldBuilderSuppressedM2Materials>,
    materials: Option<ResMut<Assets<M2EffectMaterial>>>,
) {
    if !enabled.0 {
        return;
    }
    let (Some(time), Some(mut materials)) = (time, materials) else {
        return;
    };
    let time_ms = (time.elapsed_secs_f64() * 1000.0) as u32;
    for (id, material) in materials.iter_mut() {
        if !suppressed.0.contains(&id) {
            update_m2_effect_material_uv(material, time_ms);
        }
    }
}

pub(super) fn rebuild_suppressed_m2_materials(world: &mut World) {
    let mut usage = HashMap::<AssetId<M2EffectMaterial>, (usize, usize)>::new();
    for entity in all_world_entities(world) {
        let Some(material) = world.get::<MeshMaterial3d<M2EffectMaterial>>(entity) else {
            continue;
        };
        let counts = usage.entry(material.0.id()).or_default();
        counts.0 += 1;
        counts.1 += usize::from(
            world
                .get::<WorldBuilderProcessingSuspended>(entity)
                .is_some(),
        );
    }
    let suppressed = usage
        .into_iter()
        .filter_map(|(id, (total, suspended))| (total == suspended).then_some(id))
        .collect();
    world.insert_resource(WorldBuilderSuppressedM2Materials(suppressed));
}

pub(super) fn apply_pending_world_actions(world: &mut World) {
    let actions = std::mem::take(&mut world.resource_mut::<WorldBuilderWorldActionQueue>().0);
    if actions.is_empty() {
        return;
    }
    for action in actions {
        let status = apply_world_action(world, action);
        let mut model = world.resource_mut::<WorldBuilderModel>();
        model.status = status.unwrap_or_else(|error| error);
        model.snapshot_dirty = true;
    }
}

pub(super) fn apply_world_action(
    world: &mut World,
    action: WorldBuilderWorldAction,
) -> Result<String, String> {
    match action {
        WorldBuilderWorldAction::ApplyTransform(entity, edit) => {
            apply_transform_edit(world, entity, edit)?;
            Ok(format!("Updated Transform on {entity:?}."))
        }
        WorldBuilderWorldAction::ApplyPointLight(entity, edit) => {
            apply_point_light_edit(world, entity, edit)?;
            Ok(format!("Updated PointLight on {entity:?}."))
        }
        WorldBuilderWorldAction::ApplyDirectionalLight(entity, illuminance) => {
            apply_directional_light_edit(world, entity, illuminance)?;
            Ok(format!("Updated DirectionalLight on {entity:?}."))
        }
        WorldBuilderWorldAction::TogglePointLightShadows(entity) => {
            toggle_point_light_shadows(world, entity)?;
            Ok(format!("Toggled PointLight shadows on {entity:?}."))
        }
        WorldBuilderWorldAction::ToggleDirectionalLightShadows(entity) => {
            toggle_directional_light_shadows(world, entity)?;
            Ok(format!("Toggled DirectionalLight shadows on {entity:?}."))
        }
    }
}

pub(super) fn toggle_point_light_shadows(world: &mut World, entity: Entity) -> Result<(), String> {
    let Some(mut light) = world.get_mut::<PointLight>(entity) else {
        return Err(format!("entity {entity:?} has no PointLight"));
    };
    light.shadow_maps_enabled = !light.shadow_maps_enabled;
    Ok(())
}

pub(super) fn toggle_directional_light_shadows(
    world: &mut World,
    entity: Entity,
) -> Result<(), String> {
    let Some(mut light) = world.get_mut::<DirectionalLight>(entity) else {
        return Err(format!("entity {entity:?} has no DirectionalLight"));
    };
    light.shadow_maps_enabled = !light.shadow_maps_enabled;
    Ok(())
}

pub(super) fn reconcile_requested_overrides(world: &mut World) {
    let (dirty, render_roots, processing_roots) = {
        let model = world.resource::<WorldBuilderModel>();
        (
            model.overrides_dirty,
            model.render_roots.clone(),
            model.processing_roots.clone(),
        )
    };
    if !dirty {
        return;
    }
    reconcile_render_suppression(world, &render_roots);
    reconcile_processing_suspension(world, &processing_roots);
    rebuild_suppressed_m2_materials(world);
    let mut model = world.resource_mut::<WorldBuilderModel>();
    model.overrides_dirty = false;
    model.snapshot_dirty = true;
}

pub(super) fn refresh_snapshot_if_needed(world: &mut World) {
    let (dirty, was_empty) = {
        let model = world.resource::<WorldBuilderModel>();
        (model.snapshot_dirty, model.snapshot.entities.is_empty())
    };
    if !dirty {
        return;
    }
    let snapshot = build_scene_snapshot(world);
    let entity_count = snapshot.entities.len();
    let mut model = world.resource_mut::<WorldBuilderModel>();
    prune_model_entities(&mut model, &snapshot);
    model.snapshot = snapshot;
    model.snapshot_dirty = false;
    model.page_index = model.page_index.min(visible_page_count(&model) - 1);
    if was_empty || model.status == "Refresh requested." {
        model.status = format!("Snapshot refreshed: {entity_count} scene entities.");
    }
}

pub(super) fn prune_model_entities(model: &mut WorldBuilderModel, snapshot: &SceneSnapshot) {
    let exists = |entity: &Entity| snapshot.entities.contains_key(entity);
    model.expanded.retain(exists);
    model.render_roots.retain(exists);
    model.processing_roots.retain(exists);
    if model
        .selected
        .is_some_and(|entity| !snapshot.entities.contains_key(&entity))
    {
        model.selected = None;
    }
}

pub(super) fn sync_world_builder_ui(
    model: Res<WorldBuilderModel>,
    mut screen: Option<NonSendMut<WorldBuilderScreenWrap>>,
    mut ui: ResMut<UiState>,
) {
    if !model.is_changed() {
        return;
    }
    let Some(screen) = screen.as_mut() else {
        return;
    };
    let focused_name = focused_frame_name(&ui);
    let wrap = &mut screen.0;
    wrap.shared.insert(build_view_state(&model));
    wrap.screen.sync(&wrap.shared, &mut ui.registry);
    restore_focused_frame(&mut ui, focused_name.as_deref());
}

pub(super) fn focused_frame_name(ui: &UiState) -> Option<String> {
    let focused = ui.focused_frame.or(ui.registry.focused_frame)?;
    ui.registry.get(focused)?.name.clone()
}

pub(super) fn restore_focused_frame(ui: &mut UiState, name: Option<&str>) {
    let focused = name.and_then(|name| ui.registry.get_by_name(name));
    ui.focused_frame = focused;
    ui.registry.focused_frame = focused;
}

pub(super) fn enforce_render_suppression(
    mut visibility: Query<&mut Visibility, (With<WorldBuilderRenderSuppressed>, Allow<Disabled>)>,
) {
    for mut visibility in &mut visibility {
        *visibility = Visibility::Hidden;
    }
}

pub(super) fn exit_world_builder(world: &mut World) {
    reconcile_render_suppression(world, &HashSet::new());
    reconcile_processing_suspension(world, &HashSet::new());
    world.insert_resource(WorldBuilderSuppressedM2Materials::default());
    if let Some(mut screen) = world.remove_non_send::<WorldBuilderScreenWrap>() {
        let mut ui = world.resource_mut::<UiState>();
        screen.0.screen.teardown(&mut ui.registry);
        clear_world_builder_focus(&mut ui);
    }
    if let Some(mut model) = world.get_resource_mut::<WorldBuilderModel>() {
        *model = WorldBuilderModel::default();
    }
}
