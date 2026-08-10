use super::*;

pub(super) fn build_scene_snapshot(world: &World) -> SceneSnapshot {
    let mut entities = all_world_entities(world)
        .into_iter()
        .filter_map(|entity| snapshot_entity(world, entity))
        .map(|node| (node.entity, node))
        .collect::<HashMap<_, _>>();
    let included = entities.keys().copied().collect::<HashSet<_>>();

    for node in entities.values_mut() {
        node.parent = node.parent.filter(|parent| included.contains(parent));
        node.children.retain(|child| included.contains(child));
    }

    let labels = entities
        .iter()
        .map(|(entity, node)| (*entity, node.label.to_ascii_lowercase()))
        .collect::<HashMap<_, _>>();
    for node in entities.values_mut() {
        sort_entities(&mut node.children, &labels);
    }

    let mut roots = entities
        .values()
        .filter(|node| node.parent.is_none())
        .map(|node| node.entity)
        .collect::<Vec<_>>();
    sort_entities(&mut roots, &labels);

    SceneSnapshot { roots, entities }
}

pub(super) fn all_world_entities(world: &World) -> Vec<Entity> {
    world
        .archetypes()
        .iter()
        .flat_map(|archetype| archetype.entities().iter().map(|entity| entity.id()))
        .collect()
}

pub(super) fn snapshot_entity(world: &World, entity: Entity) -> Option<SceneEntityNode> {
    let mut component_names = world
        .inspect_entity(entity)
        .ok()?
        .map(|info| info.name().to_string())
        .collect::<Vec<_>>();
    component_names.sort_unstable();
    let has_scene_transform =
        world.get::<Transform>(entity).is_some() || world.get::<GlobalTransform>(entity).is_some();
    if !has_scene_transform
        || component_names
            .iter()
            .any(|name| is_ui_render_component(name))
    {
        return None;
    }

    let transform = world.get::<Transform>(entity).copied();
    let (translation, rotation_degrees, scale) =
        transform
            .map(transform_parts)
            .unwrap_or((Vec3::ZERO, Vec3::ZERO, Vec3::ONE));
    let parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
    let children = world
        .get::<Children>(entity)
        .map(|children| children.iter().collect())
        .unwrap_or_default();

    Some(SceneEntityNode {
        entity,
        parent,
        children,
        label: entity_label(world, entity, &component_names),
        component_names,
        translation,
        rotation_degrees,
        scale,
        point_light: world
            .get::<PointLight>(entity)
            .map(|light| PointLightSnapshot {
                intensity: light.intensity,
                range: light.range,
                shadows_enabled: light.shadow_maps_enabled,
            }),
        directional_light: world.get::<DirectionalLight>(entity).map(|light| {
            DirectionalLightSnapshot {
                illuminance: light.illuminance,
                shadows_enabled: light.shadow_maps_enabled,
            }
        }),
    })
}

pub(super) fn transform_parts(transform: Transform) -> (Vec3, Vec3, Vec3) {
    let (x, y, z) = transform.rotation.to_euler(EulerRot::XYZ);
    (
        transform.translation,
        Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees()),
        transform.scale,
    )
}

pub(super) fn entity_label(world: &World, entity: Entity, component_names: &[String]) -> String {
    if let Some(name) = world.get::<Name>(entity) {
        return name.as_str().to_string();
    }
    let component_label = component_names
        .iter()
        .map(|name| name.rsplit("::").next().unwrap_or(name))
        .find(|name| !is_structural_component(name));
    component_label
        .map(|name| format!("{name} {entity:?}"))
        .unwrap_or_else(|| format!("Entity {entity:?}"))
}

pub(super) fn is_structural_component(name: &str) -> bool {
    matches!(
        name,
        "Transform"
            | "GlobalTransform"
            | "Visibility"
            | "InheritedVisibility"
            | "ViewVisibility"
            | "ChildOf"
            | "Children"
    )
}

pub(super) fn is_ui_render_component(name: &str) -> bool {
    const UI_COMPONENTS: [&str; 11] = [
        "UiCamera",
        "UiQuad",
        "UiBackdropQuad",
        "UiText",
        "UiButtonHighlight",
        "UiThreeSlicePart",
        "UiTile",
        "UiTextShadow",
        "UiTextOutline",
        "UiBorder",
        "UiNineSlicePart",
    ];
    let short_name = name.rsplit("::").next().unwrap_or(name);
    UI_COMPONENTS.contains(&short_name) || short_name == "UiBorderPart"
}

pub(super) fn sort_entities(entities: &mut [Entity], labels: &HashMap<Entity, String>) {
    entities.sort_by(|left, right| {
        labels
            .get(left)
            .cmp(&labels.get(right))
            .then_with(|| left.to_bits().cmp(&right.to_bits()))
    });
}

pub(super) fn visible_scene_rows(
    snapshot: &SceneSnapshot,
    expanded: &HashSet<Entity>,
    filter: &str,
) -> Vec<VisibleSceneRow> {
    let normalized_filter = filter.trim().to_ascii_lowercase();
    let included = if normalized_filter.is_empty() {
        None
    } else {
        Some(filtered_entities_with_ancestors(
            snapshot,
            &normalized_filter,
        ))
    };
    let mut rows = Vec::new();
    for root in &snapshot.roots {
        emit_visible_row(snapshot, *root, 0, expanded, included.as_ref(), &mut rows);
    }
    rows
}

pub(super) fn matching_entity_count(snapshot: &SceneSnapshot, filter: &str) -> usize {
    let normalized_filter = filter.trim().to_ascii_lowercase();
    if normalized_filter.is_empty() {
        return snapshot.entities.len();
    }
    snapshot
        .entities
        .values()
        .filter(|node| entity_matches_filter(node, &normalized_filter))
        .count()
}

pub(super) fn entity_matches_filter(node: &SceneEntityNode, normalized_filter: &str) -> bool {
    node.label.to_ascii_lowercase().contains(normalized_filter)
        || node
            .component_names
            .iter()
            .any(|name| name.to_ascii_lowercase().contains(normalized_filter))
}

pub(super) fn filtered_entities_with_ancestors(
    snapshot: &SceneSnapshot,
    normalized_filter: &str,
) -> HashSet<Entity> {
    let mut included = HashSet::new();
    for node in snapshot.entities.values() {
        if !entity_matches_filter(node, normalized_filter) {
            continue;
        }
        let mut current = Some(node.entity);
        while let Some(entity) = current {
            if !included.insert(entity) {
                break;
            }
            current = snapshot.entities.get(&entity).and_then(|node| node.parent);
        }
    }
    included
}

pub(super) fn emit_visible_row(
    snapshot: &SceneSnapshot,
    entity: Entity,
    depth: usize,
    expanded: &HashSet<Entity>,
    included: Option<&HashSet<Entity>>,
    rows: &mut Vec<VisibleSceneRow>,
) {
    if included.is_some_and(|included| !included.contains(&entity)) {
        return;
    }
    let Some(node) = snapshot.entities.get(&entity) else {
        return;
    };
    rows.push(VisibleSceneRow { entity, depth });
    let show_children = included.is_some() || expanded.contains(&entity);
    if !show_children {
        return;
    }
    for child in &node.children {
        emit_visible_row(snapshot, *child, depth + 1, expanded, included, rows);
    }
}

pub(super) fn apply_model_action(model: &mut WorldBuilderModel, action: WorldBuilderAction) {
    match action {
        WorldBuilderAction::Close => model.open = false,
        WorldBuilderAction::Refresh => {
            model.snapshot_dirty = true;
            model.overrides_dirty =
                !model.render_roots.is_empty() || !model.processing_roots.is_empty();
            model.status = "Refresh requested.".to_string();
        }
        WorldBuilderAction::PreviousPage => {
            model.page_index = model.page_index.saturating_sub(1);
        }
        WorldBuilderAction::NextPage => {
            let page_count = visible_page_count(model);
            model.page_index = (model.page_index + 1).min(page_count.saturating_sub(1));
        }
        WorldBuilderAction::RenderOffRoots => {
            model.render_roots = model.snapshot.roots.iter().copied().collect();
            mark_overrides_dirty(model);
            model.status = "Render disabled for every scene root.".to_string();
        }
        WorldBuilderAction::ProcessOffRoots => {
            model.processing_roots = model.snapshot.roots.iter().copied().collect();
            mark_overrides_dirty(model);
            model.status = "Processing suspended for every scene root.".to_string();
        }
        WorldBuilderAction::EnableAll => {
            model.render_roots.clear();
            model.processing_roots.clear();
            mark_overrides_dirty(model);
            model.status = "All world-builder overrides cleared.".to_string();
        }
        WorldBuilderAction::SelectEntity(bits) => {
            let Some(entity) = Entity::try_from_bits(bits) else {
                model.status = format!("Invalid entity id {bits}.");
                return;
            };
            if model.snapshot.entities.contains_key(&entity) {
                model.selected = Some(entity);
                model.status = format!("Selected {entity:?}.");
            }
        }
        WorldBuilderAction::ToggleExpand(bits) => {
            toggle_entity_set(&mut model.expanded, bits);
            model.page_index = 0;
        }
        WorldBuilderAction::ToggleRender(bits) => {
            toggle_entity_set(&mut model.render_roots, bits);
            mark_overrides_dirty(model);
        }
        WorldBuilderAction::ToggleProcessing(bits) => {
            toggle_entity_set(&mut model.processing_roots, bits);
            mark_overrides_dirty(model);
        }
        WorldBuilderAction::ApplyTransform(_)
        | WorldBuilderAction::ApplyPointLight(_)
        | WorldBuilderAction::ApplyDirectionalLight(_)
        | WorldBuilderAction::TogglePointLightShadows(_)
        | WorldBuilderAction::ToggleDirectionalLightShadows(_) => {}
    }
}

pub(super) fn mark_overrides_dirty(model: &mut WorldBuilderModel) {
    model.overrides_dirty = true;
    model.snapshot_dirty = true;
}

pub(super) fn toggle_entity_set(entities: &mut HashSet<Entity>, bits: u64) {
    let Some(entity) = Entity::try_from_bits(bits) else {
        return;
    };
    if !entities.remove(&entity) {
        entities.insert(entity);
    }
}

pub(super) fn visible_page_count(model: &WorldBuilderModel) -> usize {
    let row_count = visible_scene_rows(&model.snapshot, &model.expanded, &model.filter).len();
    row_count.div_ceil(WORLD_BUILDER_PAGE_SIZE).max(1)
}

pub(super) fn build_view_state(model: &WorldBuilderModel) -> WorldBuilderViewState {
    let visible_rows = visible_scene_rows(&model.snapshot, &model.expanded, &model.filter);
    let page_count = visible_rows.len().div_ceil(WORLD_BUILDER_PAGE_SIZE).max(1);
    let page_index = model.page_index.min(page_count - 1);
    let page_start = page_index * WORLD_BUILDER_PAGE_SIZE;
    let rows = visible_rows
        .iter()
        .skip(page_start)
        .take(WORLD_BUILDER_PAGE_SIZE)
        .filter_map(|row| view_row(model, *row))
        .collect();

    WorldBuilderViewState {
        open: model.open,
        total_entities: model.snapshot.entities.len(),
        matching_entities: matching_entity_count(&model.snapshot, &model.filter),
        rows,
        page_index,
        page_count,
        filter: model.filter.clone(),
        selected: model
            .selected
            .and_then(|entity| selected_properties(model, entity)),
        status: model.status.clone(),
    }
}

pub(super) fn view_row(model: &WorldBuilderModel, row: VisibleSceneRow) -> Option<WorldBuilderRow> {
    let node = model.snapshot.entities.get(&row.entity)?;
    Some(WorldBuilderRow {
        entity_bits: row.entity.to_bits(),
        depth: row.depth,
        label: node.label.clone(),
        has_children: !node.children.is_empty(),
        expanded: model.expanded.contains(&row.entity),
        selected: model.selected == Some(row.entity),
        render_hidden: entity_or_ancestor_is_selected(
            &model.snapshot,
            &model.render_roots,
            row.entity,
        ),
        processing_suspended: entity_or_ancestor_is_selected(
            &model.snapshot,
            &model.processing_roots,
            row.entity,
        ),
    })
}

fn entity_or_ancestor_is_selected(
    snapshot: &SceneSnapshot,
    selected: &HashSet<Entity>,
    entity: Entity,
) -> bool {
    let mut current = Some(entity);
    while let Some(candidate) = current {
        if selected.contains(&candidate) {
            return true;
        }
        current = snapshot
            .entities
            .get(&candidate)
            .and_then(|node| node.parent);
    }
    false
}

pub(super) fn selected_properties(
    model: &WorldBuilderModel,
    entity: Entity,
) -> Option<WorldBuilderPropertyState> {
    let node = model.snapshot.entities.get(&entity)?;
    Some(WorldBuilderPropertyState {
        selected_label: node.label.clone(),
        selected_id: entity.to_bits(),
        translation: format_vec3(node.translation),
        rotation_degrees: format_vec3(node.rotation_degrees),
        scale: format_vec3(node.scale),
        component_names: node.component_names.clone(),
        point_light: node.point_light.map(|light| WorldBuilderPointLightState {
            intensity: format_float(light.intensity),
            range: format_float(light.range),
            shadows_enabled: light.shadows_enabled,
        }),
        directional_light: node
            .directional_light
            .map(|light| WorldBuilderDirectionalLightState {
                illuminance: format_float(light.illuminance),
                shadows_enabled: light.shadows_enabled,
            }),
    })
}

pub(super) fn format_vec3(value: Vec3) -> [String; 3] {
    [
        format_float(value.x),
        format_float(value.y),
        format_float(value.z),
    ]
}

pub(super) fn format_float(value: f32) -> String {
    format!("{value:.3}")
}

pub(super) fn reconcile_render_suppression(world: &mut World, roots: &HashSet<Entity>) {
    let desired = collect_root_subtrees(world, roots);
    let existing = all_world_entities(world)
        .into_iter()
        .filter_map(|entity| {
            world
                .get::<WorldBuilderRenderSuppressed>(entity)
                .copied()
                .map(|marker| (entity, marker))
        })
        .collect::<Vec<_>>();

    for (entity, marker) in existing {
        if desired.contains(&entity) {
            if let Some(mut visibility) = world.get_mut::<Visibility>(entity) {
                *visibility = Visibility::Hidden;
            }
            continue;
        }
        if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
            entity_mut.insert(marker.previous);
            entity_mut.remove::<WorldBuilderRenderSuppressed>();
        }
    }

    for entity in desired {
        if world.get::<WorldBuilderRenderSuppressed>(entity).is_some() {
            continue;
        }
        let Some(previous) = world.get::<Visibility>(entity).copied() else {
            continue;
        };
        if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
            entity_mut.insert((
                WorldBuilderRenderSuppressed { previous },
                Visibility::Hidden,
            ));
        }
    }
}

pub(super) fn reconcile_processing_suspension(world: &mut World, roots: &HashSet<Entity>) {
    let desired = collect_root_subtrees(world, roots);
    restore_processing_outside_desired_subtrees(world, &desired);
    suspend_processing_in_desired_subtrees(world, desired);
}

fn restore_processing_outside_desired_subtrees(world: &mut World, desired: &HashSet<Entity>) {
    let existing = all_world_entities(world)
        .into_iter()
        .filter_map(|entity| processing_marker(world, entity).map(|marker| (entity, marker)))
        .collect::<Vec<_>>();
    for (entity, marker) in existing {
        if !desired.contains(&entity) {
            restore_processing_entity(world, entity, marker);
        }
    }
}

fn processing_marker(world: &World, entity: Entity) -> Option<WorldBuilderProcessingSuspended> {
    world
        .get::<WorldBuilderProcessingSuspended>(entity)
        .copied()
}

fn restore_processing_entity(
    world: &mut World,
    entity: Entity,
    marker: WorldBuilderProcessingSuspended,
) {
    let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
        return;
    };
    entity_mut.remove::<WorldBuilderProcessingSuspended>();
    if marker.remove_disabled_on_restore {
        entity_mut.remove::<Disabled>();
    }
}

fn suspend_processing_in_desired_subtrees(world: &mut World, desired: HashSet<Entity>) {
    for entity in desired {
        if processing_marker(world, entity).is_none() {
            suspend_processing_entity(world, entity);
        }
    }
}

fn suspend_processing_entity(world: &mut World, entity: Entity) {
    let remove_disabled_on_restore = world.get::<Disabled>(entity).is_none();
    let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
        return;
    };
    entity_mut.insert(WorldBuilderProcessingSuspended {
        remove_disabled_on_restore,
    });
    if remove_disabled_on_restore {
        entity_mut.insert(Disabled);
    }
}

pub(super) fn collect_root_subtrees(world: &World, roots: &HashSet<Entity>) -> HashSet<Entity> {
    let mut collected = HashSet::new();
    for root in roots {
        collect_subtree(world, *root, &mut collected);
    }
    collected
}

pub(super) fn collect_subtree(world: &World, entity: Entity, collected: &mut HashSet<Entity>) {
    if !world.entities().contains(entity) || !collected.insert(entity) {
        return;
    }
    let Some(children) = world.get::<Children>(entity) else {
        return;
    };
    for child in children.iter() {
        collect_subtree(world, child, collected);
    }
}

pub(super) fn apply_point_light_edit(
    world: &mut World,
    entity: Entity,
    edit: PointLightEdit,
) -> Result<(), String> {
    if !edit.intensity.is_finite() || !edit.range.is_finite() {
        return Err("point-light values must be finite".to_string());
    }
    if edit.intensity < 0.0 || edit.range <= 0.0 {
        return Err(
            "point-light intensity must be non-negative and range must be positive".to_string(),
        );
    }
    let Some(mut light) = world.get_mut::<PointLight>(entity) else {
        return Err(format!("entity {entity:?} has no PointLight"));
    };
    light.intensity = edit.intensity;
    light.range = edit.range;
    Ok(())
}

pub(super) fn apply_directional_light_edit(
    world: &mut World,
    entity: Entity,
    illuminance: f32,
) -> Result<(), String> {
    if !illuminance.is_finite() || illuminance < 0.0 {
        return Err("directional-light illuminance must be finite and non-negative".to_string());
    }
    let Some(mut light) = world.get_mut::<DirectionalLight>(entity) else {
        return Err(format!("entity {entity:?} has no DirectionalLight"));
    };
    light.illuminance = illuminance;
    Ok(())
}

pub(super) fn apply_transform_edit(
    world: &mut World,
    entity: Entity,
    edit: TransformEdit,
) -> Result<(), String> {
    validate_transform_edit(edit)?;
    let Some(mut transform) = world.get_mut::<Transform>(entity) else {
        return Err(format!("entity {entity:?} has no Transform"));
    };
    transform.translation = edit.translation;
    transform.rotation = Quat::from_euler(
        EulerRot::XYZ,
        edit.rotation_degrees.x.to_radians(),
        edit.rotation_degrees.y.to_radians(),
        edit.rotation_degrees.z.to_radians(),
    );
    transform.scale = edit.scale;
    Ok(())
}

pub(super) fn validate_transform_edit(edit: TransformEdit) -> Result<(), String> {
    if !edit.translation.is_finite()
        || !edit.rotation_degrees.is_finite()
        || !edit.scale.is_finite()
    {
        return Err("transform values must be finite".to_string());
    }
    let scale_in_range = edit.scale.cmple(Vec3::splat(MAX_EDITABLE_SCALE)).all()
        && edit.scale.cmpge(Vec3::splat(MIN_EDITABLE_SCALE)).all();
    if !scale_in_range {
        return Err(format!(
            "scale must stay between {MIN_EDITABLE_SCALE} and {MAX_EDITABLE_SCALE}"
        ));
    }
    Ok(())
}
