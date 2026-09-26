use std::collections::{HashMap, HashSet};

use crate::retail_m2_material::M2Material;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationChoice, CustomizationDb};

use crate::m2_spawn::{BatchTextureType, GeosetMesh};
use game_engine::creature_display::npc_appearance::{
    AuthoredNpcAppearance, load_authored_npc_appearance,
};

#[derive(Component, Clone)]
pub(super) struct NpcAppearanceRequest {
    display_id: u32,
    appearance: AuthoredNpcAppearance,
}

pub(super) fn load_and_queue_npc_appearance(
    commands: &mut Commands,
    root: Entity,
    display_id: u32,
) {
    let appearance =
        load_authored_npc_appearance(display_id).unwrap_or_else(|error| panic!("{error}"));
    if let Some(appearance) = appearance {
        commands.entity(root).insert(NpcAppearanceRequest {
            display_id,
            appearance,
        });
    }
}

#[derive(Default)]
struct NpcSelections {
    materials: Vec<(u16, u32)>,
    geosets: Vec<(u16, u16)>,
}

/// Applies the displayed model's choices. Choices of the race's unaltered form
/// (a Worgen profile's Gilnean-form choices) resolve but belong to the other model.
fn select_npc_choices(
    appearance: &AuthoredNpcAppearance,
    db: &CustomizationDb,
) -> Result<NpcSelections, String> {
    let (race, sex) = (appearance.race, appearance.sex);
    let displayed = appearance
        .choice_ids
        .iter()
        .filter_map(|&id| db.choice_by_id(race, sex, id));
    resolve_npc_choices(appearance, displayed, |id| {
        db.unaltered_form_choice_by_id(race, sex, id).is_some()
    })
}

fn resolve_npc_choices<'a>(
    appearance: &AuthoredNpcAppearance,
    choices: impl IntoIterator<Item = &'a CustomizationChoice>,
    is_other_form_choice: impl Fn(u32) -> bool,
) -> Result<NpcSelections, String> {
    let selected: HashSet<_> = appearance.choice_ids.iter().copied().collect();
    let mut missing = selected.clone();
    missing.retain(|&id| !is_other_form_choice(id));
    let mut result = NpcSelections::default();
    for choice in choices {
        if !selected.contains(&choice.id) {
            continue;
        }
        missing.remove(&choice.id);
        append_selected_choice(&mut result, choice, &selected);
    }
    if !missing.is_empty() {
        let mut missing: Vec<_> = missing.into_iter().collect();
        missing.sort_unstable();
        return Err(format!(
            "unresolved customization choices {missing:?} for race {} sex {}",
            appearance.race, appearance.sex
        ));
    }
    Ok(result)
}

fn append_selected_choice(
    output: &mut NpcSelections,
    choice: &CustomizationChoice,
    selected: &HashSet<u32>,
) {
    output.materials.extend_from_slice(&choice.materials);
    output.materials.extend(
        choice
            .related_materials
            .iter()
            .filter(|material| selected.contains(&material.related_choice_id))
            .map(|material| (material.target_id, material.fdid)),
    );
    output.geosets.extend_from_slice(&choice.geosets);
    output.geosets.extend(
        choice
            .related_geosets
            .iter()
            .filter(|geoset| selected.contains(&geoset.related_choice_id))
            .map(|geoset| (geoset.geoset_type, geoset.geoset_id)),
    );
}

#[derive(Component)]
struct PreparedNpcAppearance {
    display_id: u32,
    textures: HashMap<u32, Handle<Image>>,
    selected_geosets: Vec<(u16, u16)>,
    authored_geosets: Vec<(u16, u16)>,
}

fn prepare_npc_appearance(
    request: &NpcAppearanceRequest,
    db: &CustomizationDb,
    compositor: &CharTextureData,
    images: &mut Assets<Image>,
) -> Result<PreparedNpcAppearance, String> {
    let appearance = &request.appearance;
    let selected = select_npc_choices(appearance, db)?;
    let layout = db
        .layout_id(appearance.race, appearance.sex)
        .ok_or_else(|| {
            format!(
                "missing texture layout for race {} sex {}",
                appearance.race, appearance.sex
            )
        })?;
    let textures = load_and_composite_npc_textures(
        appearance,
        &selected.materials,
        layout,
        compositor,
        images,
    )?;
    Ok(PreparedNpcAppearance {
        display_id: request.display_id,
        textures,
        selected_geosets: selected.geosets,
        authored_geosets: appearance.geosets.clone(),
    })
}

fn load_npc_texture(fdid: u32) -> Result<Image, String> {
    let path = crate::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("missing NPC texture FDID {fdid}"))?;
    crate::asset::blp::load_blp_to_image(&path).map_err(|error| {
        format!(
            "decode NPC texture FDID {fdid} at {}: {error}",
            path.display()
        )
    })
}

fn load_and_composite_npc_textures(
    appearance: &AuthoredNpcAppearance,
    materials: &[(u16, u32)],
    layout: u32,
    compositor: &CharTextureData,
    images: &mut Assets<Image>,
) -> Result<HashMap<u32, Handle<Image>>, String> {
    for fdid in materials
        .iter()
        .map(|(_, fdid)| *fdid)
        .collect::<HashSet<_>>()
    {
        load_npc_texture(fdid)?;
    }
    let composed = compositor
        .composite_model_textures(materials, &[], layout)
        .ok_or_else(|| format!("cannot composite NPC texture layout {layout}"))?;
    let body = load_npc_body_texture(
        appearance.baked_texture_fdid,
        composed.body,
        load_npc_texture,
    )?;
    let mut textures = HashMap::from([(1, images.add(body))]);
    if let Some((pixels, width, height)) = select_npc_type6_texture(
        compositor.declares_hair(materials, layout),
        composed.hair,
        composed.head,
    )? {
        textures.insert(6, images.add(crate::rgba_image(pixels, width, height)));
    }
    if let Some(fdid) = compositor.replacement_texture_fdid(materials, layout, 19) {
        textures.insert(19, images.add(load_npc_texture(fdid)?));
    }
    Ok(textures)
}

type NpcTexturePixels = (Vec<u8>, u32, u32);

fn select_npc_type6_texture(
    declares_hair: bool,
    hair: Option<NpcTexturePixels>,
    head: Option<NpcTexturePixels>,
) -> Result<Option<NpcTexturePixels>, String> {
    if declares_hair {
        return hair
            .map(Some)
            .ok_or_else(|| "declared NPC hair target 10 did not produce a texture".to_string());
    }
    Ok(head)
}

fn load_npc_body_texture(
    baked_fdid: Option<u32>,
    composed: (Vec<u8>, u32, u32),
    load: impl FnOnce(u32) -> Result<Image, String>,
) -> Result<Image, String> {
    match baked_fdid {
        Some(fdid) => load(fdid),
        None => Ok(crate::rgba_image(composed.0, composed.1, composed.2)),
    }
}

fn npc_geoset_visible(mesh_part: u16, prepared: &PreparedNpcAppearance) -> bool {
    use crate::rendering::character_customization::{
        apply_exact_geoset_overrides, is_geoset_visible,
    };
    if mesh_part < 100
        && let Some((_, variant)) = prepared
            .authored_geosets
            .iter()
            .rev()
            .find(|(group, _)| *group == 0)
    {
        return is_geoset_visible(mesh_part, &[(0, *variant)], &[0]);
    }
    let active_types: Vec<_> = prepared
        .selected_geosets
        .iter()
        .map(|(group, _)| *group)
        .collect();
    let visible = is_geoset_visible(mesh_part, &prepared.selected_geosets, &active_types);
    apply_exact_geoset_overrides(mesh_part, visible, &prepared.authored_geosets)
}

fn is_descendant(entity: Entity, root: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        current = parent.parent();
        if current == root {
            return true;
        }
    }
    false
}

#[derive(SystemParam)]
struct NpcRenderTargets<'w, 's> {
    parents: Query<'w, 's, &'static ChildOf>,
    geosets: Query<'w, 's, (Entity, &'static GeosetMesh, &'static mut Visibility)>,
    meshes: Query<
        'w,
        's,
        (
            Entity,
            &'static BatchTextureType,
            &'static mut MeshMaterial3d<M2Material>,
        ),
    >,
    materials: ResMut<'w, Assets<M2Material>>,
}

fn apply_prepared_npc(
    root: Entity,
    prepared: &PreparedNpcAppearance,
    targets: &mut NpcRenderTargets,
) -> Result<(), String> {
    apply_npc_geosets(root, prepared, targets);
    apply_npc_textures(root, prepared, targets)
}

fn apply_npc_geosets(
    root: Entity,
    prepared: &PreparedNpcAppearance,
    targets: &mut NpcRenderTargets,
) {
    for (entity, geoset, mut visibility) in &mut targets.geosets {
        if is_descendant(entity, root, &targets.parents) {
            *visibility = if npc_geoset_visible(geoset.0, prepared) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn apply_npc_textures(
    root: Entity,
    prepared: &PreparedNpcAppearance,
    targets: &mut NpcRenderTargets,
) -> Result<(), String> {
    for (entity, texture_type, mut handle) in &mut targets.meshes {
        if !is_descendant(entity, root, &targets.parents) {
            continue;
        }
        let Some(texture) = prepared.textures.get(&texture_type.0) else {
            if matches!(texture_type.0, 1 | 6) {
                return Err(format!(
                    "missing NPC replacement texture type {} for entity {entity:?}",
                    texture_type.0
                ));
            }
            continue;
        };
        let mut material = targets
            .materials
            .get(&handle.0)
            .ok_or_else(|| format!("missing NPC material for entity {entity:?}"))?
            .clone();
        material.base.base_color_texture = Some(texture.clone());
        material.base.base_color = Color::WHITE;
        handle.0 = targets.materials.add(material);
    }
    Ok(())
}

fn prepare_added_npc_appearances(
    requests: Query<(Entity, &NpcAppearanceRequest), Added<NpcAppearanceRequest>>,
    db: Res<CustomizationDb>,
    compositor: Res<CharTextureData>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    for (root, request) in &requests {
        let prepared = prepare_npc_appearance(request, &db, &compositor, &mut images)
            .unwrap_or_else(|error| panic!("NPC display {}: {error}", request.display_id));
        commands
            .entity(root)
            .insert(prepared)
            .remove::<NpcAppearanceRequest>();
    }
}

fn apply_added_npc_appearances(
    requests: Query<(Entity, &PreparedNpcAppearance), Added<PreparedNpcAppearance>>,
    mut targets: NpcRenderTargets,
) {
    for (root, prepared) in &requests {
        apply_prepared_npc(root, prepared, &mut targets)
            .unwrap_or_else(|error| panic!("NPC display {}: {error}", prepared.display_id));
    }
}

pub(super) fn register_npc_appearance_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            prepare_added_npc_appearances
                .run_if(|requests: Query<(), Added<NpcAppearanceRequest>>| !requests.is_empty()),
            apply_added_npc_appearances
                .run_if(|requests: Query<(), Added<PreparedNpcAppearance>>| !requests.is_empty()),
        )
            .chain(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_appearance_declared_hair_never_substitutes_head_after_composition_failure() {
        let hair = (vec![180, 90, 30, 255], 1, 1);
        let head = (vec![230, 180, 160, 255], 1, 1);
        assert_eq!(
            select_npc_type6_texture(true, Some(hair.clone()), Some(head.clone())).unwrap(),
            Some(hair),
        );
        assert_eq!(
            select_npc_type6_texture(false, None, Some(head.clone())).unwrap(),
            Some(head.clone()),
        );
        assert!(select_npc_type6_texture(true, None, Some(head)).is_err());
    }

    #[test]
    fn npc_appearance_baked_body_preserves_clothing_and_missing_bake_is_an_error() {
        let composed = (vec![20, 30, 40, 255], 1, 1);
        let baked = load_npc_body_texture(Some(1050256), composed.clone(), |fdid| match fdid {
            1050256 => Ok(crate::rgba_image(vec![180, 60, 90, 255], 1, 1)),
            _ => Err(format!("unavailable fixture FDID {fdid}")),
        })
        .unwrap();
        assert_eq!(baked.data.as_deref(), Some([180, 60, 90, 255].as_slice()));
        let generated =
            load_npc_body_texture(None, composed.clone(), |_| Err("unexpected FDID".into()))
                .unwrap();
        assert_eq!(
            generated.data.as_deref(),
            Some([20, 30, 40, 255].as_slice())
        );
        let missing = load_npc_body_texture(Some(1050256), composed, |_| {
            Err("missing baked texture 1050256".into())
        });
        assert_eq!(missing.unwrap_err(), "missing baked texture 1050256");
    }

    #[test]
    fn npc_appearance_uses_full_choice_ids_and_related_selections() {
        use game_engine::customization_data::{ChoiceGeoset, ChoiceMaterial, OptionType};
        let db = CustomizationDb::load(std::path::Path::new("data"));
        let mut first = db
            .get_choice(1, 0, OptionType::SkinColor, 0)
            .unwrap()
            .clone();
        first.id = 70001;
        first.materials = vec![(1, 100)];
        first.geosets = vec![(0, 2)];
        first.related_materials = vec![
            ChoiceMaterial {
                related_choice_id: 70002,
                target_id: 2,
                fdid: 200,
            },
            ChoiceMaterial {
                related_choice_id: 70003,
                target_id: 2,
                fdid: 300,
            },
        ];
        first.related_geosets = vec![ChoiceGeoset {
            related_choice_id: 70002,
            geoset_type: 21,
            geoset_id: 3,
        }];
        let mut second = first.clone();
        second.id = 70002;
        second.materials.clear();
        second.geosets.clear();
        second.related_materials.clear();
        second.related_geosets.clear();
        let mut appearance = AuthoredNpcAppearance {
            race: 1,
            sex: 0,
            class: 0,
            baked_texture_fdid: None,
            choice_ids: vec![70001, 70002],
            geosets: vec![],
        };
        let selected = resolve_npc_choices(&appearance, [&first, &second], |_| false).unwrap();
        assert_eq!(selected.materials, vec![(1, 100), (2, 200)]);
        assert_eq!(selected.geosets, vec![(0, 2), (21, 3)]);
        appearance.choice_ids.push(70004);
        let error = resolve_npc_choices(&appearance, [&first, &second], |_| false)
            .err()
            .unwrap();
        assert!(error.contains("70004"), "{error}");
    }

    #[test]
    #[ignore]
    fn sweep_all_spawned_profiles() {
        use game_engine::creature_display::npc_appearance::query_authored_npc_appearance;
        let db = CustomizationDb::try_load(std::path::Path::new("data")).unwrap();
        let compositor = CharTextureData::load(std::path::Path::new("data"));
        let conn = rusqlite::Connection::open(std::env::var("SWEEP_CACHE").unwrap()).unwrap();
        let ids: Vec<u32> = std::fs::read_to_string(std::env::var("SWEEP_IDS").unwrap())
            .unwrap()
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        let (mut ok, mut ordinary, mut failures) = (0, 0, Vec::new());
        for (n, id) in ids.into_iter().enumerate() {
            let mut images = Assets::<Image>::default();
            if n % 250 == 0 {
                println!("SWEEP progress {n}");
            }
            match query_authored_npc_appearance(&conn, id) {
                Err(e) => failures.push(format!("{id}: {e}")),
                Ok(None) => ordinary += 1,
                Ok(Some(appearance)) => {
                    let request = NpcAppearanceRequest {
                        display_id: id,
                        appearance,
                    };
                    match prepare_npc_appearance(&request, &db, &compositor, &mut images) {
                        Ok(_) => ok += 1,
                        Err(e) => failures.push(format!("{id}: {e}")),
                    }
                }
            }
        }
        println!(
            "SWEEP ok={ok} ordinary={ordinary} failures={}",
            failures.len()
        );
        for f in &failures {
            println!("SWEEP FAIL {f}");
        }
    }

    fn real_profile(race: u8, sex: u8, choice_ids: Vec<u32>) -> AuthoredNpcAppearance {
        AuthoredNpcAppearance {
            race,
            sex,
            class: 0,
            baked_texture_fdid: None,
            choice_ids,
            geosets: vec![],
        }
    }

    #[test]
    fn kul_tiran_display_140376_choices_resolve() {
        let db = CustomizationDb::try_load(std::path::Path::new("data")).unwrap();
        // Suspicious Citizen, Retail display 140376 (race 32, sex 1).
        let appearance = real_profile(
            32,
            1,
            vec![
                3142, 3150, 3162, 3167, 3172, 3179, 7682, 45226, 54464, 56655,
            ],
        );
        let selected = select_npc_choices(&appearance, &db).unwrap();
        assert!(!selected.materials.is_empty());
    }

    #[test]
    fn dracthyr_display_110154_target_10_is_not_hair() {
        let db = CustomizationDb::try_load(std::path::Path::new("data")).unwrap();
        let compositor = CharTextureData::load(std::path::Path::new("data"));
        // Retail display 110154 (race 52 Dracthyr, layout 155): material target 10
        // feeds texture type 9, and the dragon M2 has no type-6 hair batch.
        let appearance = AuthoredNpcAppearance {
            baked_texture_fdid: Some(4736911),
            ..real_profile(
                52,
                0,
                vec![
                    19363, 19382, 19388, 19409, 19479, 19482, 19495, 19510, 19512, 19516, 19534,
                    19545, 19553, 19554, 19560, 19564, 19572, 19576, 19581, 19587, 19591, 26866,
                    26878, 28275, 28282, 29741, 29786, 29794, 29841, 29888, 29895, 29914, 29941,
                    29966, 29977, 29979, 29980, 29981, 29982, 29985, 30028, 30060,
                ],
            )
        };
        let request = NpcAppearanceRequest {
            display_id: 110154,
            appearance,
        };
        let mut images = Assets::<Image>::default();
        let prepared = prepare_npc_appearance(&request, &db, &compositor, &mut images).unwrap();
        assert!(prepared.textures.contains_key(&1));
        assert!(!prepared.textures.contains_key(&6));
        // Human layouts keep target 10 as the type-6 hair layer.
        let human_layout = db.layout_id(1, 0).unwrap();
        assert!(compositor.declares_hair(&[(10, 1)], human_layout));
        assert!(!compositor.declares_hair(&[(10, 1)], db.layout_id(52, 0).unwrap()));
    }

    #[test]
    fn worgen_display_31054_applies_worgen_choices_and_skips_gilnean_form() {
        let db = CustomizationDb::try_load(std::path::Path::new("data")).unwrap();
        let worgen = vec![2232, 2241, 2260, 2279, 7042, 7444];
        let gilnean = vec![1, 20, 48, 61, 81, 4138, 9912, 9914, 9925];
        let mixed = real_profile(22, 0, [worgen.clone(), gilnean].concat());
        let only_worgen = real_profile(22, 0, worgen);
        let selected = select_npc_choices(&mixed, &db).unwrap();
        let expected = select_npc_choices(&only_worgen, &db).unwrap();
        assert!(!selected.materials.is_empty());
        assert_eq!(selected.materials, expected.materials);
        assert_eq!(selected.geosets, expected.geosets);
        let unknown = real_profile(22, 0, vec![2232, 999_999]);
        let error = select_npc_choices(&unknown, &db).err().unwrap();
        assert!(error.contains("999999"), "{error}");
    }

    fn mesh(
        world: &mut World,
        root: Entity,
        material: Handle<M2Material>,
        part: u16,
        texture_type: u32,
    ) -> Entity {
        world
            .spawn((
                ChildOf(root),
                GeosetMesh(part),
                BatchTextureType(texture_type),
                MeshMaterial3d(material),
                Visibility::Inherited,
            ))
            .id()
    }

    #[test]
    fn npc_appearance_clones_shared_materials_and_preserves_other_entities() {
        let mut app = App::new();
        app.init_resource::<Assets<M2Material>>()
            .init_resource::<Assets<Image>>();
        register_npc_appearance_systems(&mut app);
        let original_texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(crate::rgba_image(vec![10, 100, 10, 255], 1, 1));
        let first_texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(crate::rgba_image(vec![180, 60, 30, 255], 1, 1));
        let second_texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(crate::rgba_image(vec![30, 60, 180, 255], 1, 1));
        let material = app.world_mut().resource_mut::<Assets<M2Material>>().add(
            crate::retail_m2_material::retail_m2_material(
                StandardMaterial {
                    base_color_texture: Some(original_texture.clone()),
                    ..default()
                },
                0,
                0,
            ),
        );
        let first = app
            .world_mut()
            .spawn(PreparedNpcAppearance {
                display_id: 19177,
                textures: HashMap::from([(1, first_texture.clone()), (6, first_texture.clone())]),
                selected_geosets: vec![(0, 5), (4, 1)],
                authored_geosets: vec![(0, 2), (4, 3)],
            })
            .id();
        let second = app
            .world_mut()
            .spawn(PreparedNpcAppearance {
                display_id: 19178,
                textures: HashMap::from([(1, second_texture.clone()), (6, second_texture.clone())]),
                selected_geosets: vec![(0, 5)],
                authored_geosets: vec![],
            })
            .id();
        let ordinary = app.world_mut().spawn_empty().id();
        let nested_first = app.world_mut().spawn(ChildOf(first)).id();
        let first_mesh = mesh(app.world_mut(), nested_first, material.clone(), 0, 1);
        let second_mesh = mesh(app.world_mut(), second, material.clone(), 0, 1);
        let other_mesh = mesh(app.world_mut(), ordinary, material.clone(), 0, 1);
        let first_hair = mesh(app.world_mut(), nested_first, material.clone(), 2, 6);
        let first_other_hair = mesh(app.world_mut(), nested_first, material.clone(), 5, 6);
        let second_hair = mesh(app.world_mut(), second, material.clone(), 5, 6);
        let first_glove = mesh(app.world_mut(), first, material.clone(), 403, 0);
        let first_other_glove = mesh(app.world_mut(), first, material.clone(), 401, 0);
        app.update();
        let a = app
            .world()
            .get::<MeshMaterial3d<M2Material>>(first_mesh)
            .unwrap()
            .0
            .clone();
        let b = app
            .world()
            .get::<MeshMaterial3d<M2Material>>(second_mesh)
            .unwrap()
            .0
            .clone();
        let c = app
            .world()
            .get::<MeshMaterial3d<M2Material>>(other_mesh)
            .unwrap()
            .0
            .clone();
        assert_ne!(a, b);
        assert_ne!(a, material);
        assert_eq!(c, material);
        let materials = app.world().resource::<Assets<M2Material>>();
        assert_eq!(
            materials.get(&a).unwrap().base.base_color_texture,
            Some(first_texture.clone())
        );
        assert_eq!(
            materials.get(&b).unwrap().base.base_color_texture,
            Some(second_texture.clone())
        );
        assert_eq!(
            materials.get(&c).unwrap().base.base_color_texture,
            Some(original_texture)
        );
        for (entity, expected_texture) in
            [(first_hair, first_texture), (second_hair, second_texture)]
        {
            let handle = &app
                .world()
                .get::<MeshMaterial3d<M2Material>>(entity)
                .unwrap()
                .0;
            assert_eq!(
                materials.get(handle).unwrap().base.base_color_texture,
                Some(expected_texture)
            );
        }
        for (entity, expected) in [
            (first_mesh, Visibility::Inherited),
            (first_hair, Visibility::Inherited),
            (second_hair, Visibility::Inherited),
            (first_other_hair, Visibility::Hidden),
            (first_glove, Visibility::Inherited),
            (first_other_glove, Visibility::Hidden),
        ] {
            assert_eq!(*app.world().get::<Visibility>(entity).unwrap(), expected);
        }
        let count = app.world().resource::<Assets<M2Material>>().len();
        app.update();
        assert_eq!(app.world().resource::<Assets<M2Material>>().len(), count);
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<M2Material>>(first_mesh)
                .unwrap()
                .0,
            a
        );
    }
}
