use std::collections::{HashMap, HashSet};

use crate::retail_m2_material::M2Material;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::CustomizationDb;
#[cfg(test)]
use game_engine::npc_appearance_selection_data::resolve_npc_choices;
use game_engine::npc_appearance_selection_data::{
    npc_geoset_visible, select_npc_choices, select_npc_type6_texture,
};

use crate::equipment::EquipmentItem;
use crate::equipment_appearance::ResolvedEquipmentAppearance;
use crate::m2_spawn::{BatchTextureType, GeosetMesh};
use game_engine::creature_display::npc_appearance::{
    AuthoredNpcAppearance, load_authored_npc_appearance,
};
use game_engine::geoset_visibility_data::apply_exact_geoset_overrides;

#[derive(Component, Clone)]
pub(super) struct NpcAppearanceRequest {
    display_id: u32,
    appearance: AuthoredNpcAppearance,
    armor: NpcArmorGeosets,
}

/// Body geosets of a display's authored armor; its textures are in the display's bake.
#[derive(Clone, Debug, Default)]
struct NpcArmorGeosets {
    overrides: Vec<(u16, u16)>,
    hidden_groups: HashSet<u16>,
    hidden_ids: HashSet<u16>,
}

impl NpcArmorGeosets {
    fn from_resolved(armor: &ResolvedEquipmentAppearance) -> Self {
        Self {
            overrides: armor.outfit.geoset_overrides.clone(),
            hidden_groups: armor.hidden_character_geoset_groups.clone(),
            hidden_ids: armor.hidden_character_geoset_ids.clone(),
        }
    }
}

pub(super) fn load_npc_appearance(display_id: u32) -> Option<AuthoredNpcAppearance> {
    load_authored_npc_appearance(display_id).unwrap_or_else(|error| panic!("{error}"))
}

pub(super) fn queue_npc_appearance(
    commands: &mut Commands,
    root: Entity,
    display_id: u32,
    appearance: AuthoredNpcAppearance,
    armor: &ResolvedEquipmentAppearance,
) {
    commands.entity(root).insert(NpcAppearanceRequest {
        display_id,
        appearance,
        armor: NpcArmorGeosets::from_resolved(armor),
    });
}

#[derive(Component)]
struct PreparedNpcAppearance {
    display_id: u32,
    textures: HashMap<u32, Handle<Image>>,
    selected_geosets: Vec<(u16, u16)>,
    authored_geosets: Vec<(u16, u16)>,
    armor: NpcArmorGeosets,
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
        selected_geosets: armor_hidden_geosets(selected.geosets, request, db),
        authored_geosets: appearance.geosets.clone(),
        armor: request.armor.clone(),
    })
}

/// Customization geosets with the groups the armor hides replaced (hair by the scalp).
fn armor_hidden_geosets(
    mut geosets: Vec<(u16, u16)>,
    request: &NpcAppearanceRequest,
    db: &CustomizationDb,
) -> Vec<(u16, u16)> {
    crate::character_customization::apply_hidden_geoset_groups(
        &mut geosets,
        &request.armor.hidden_groups,
        request.appearance.race,
        request.appearance.sex,
        db,
    );
    geosets
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

/// Whether `entity` is below `root` and not part of an equipment item model there.
fn is_body_descendant(
    entity: Entity,
    root: Entity,
    parents: &Query<&ChildOf>,
    items: &Query<(), With<EquipmentItem>>,
) -> bool {
    let mut current = entity;
    while let Ok(parent) = parents.get(current) {
        if items.contains(current) {
            return false;
        }
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
    items: Query<'w, 's, (), With<EquipmentItem>>,
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
        if is_body_descendant(entity, root, &targets.parents, &targets.items) {
            *visibility = if npc_body_geoset_visible(geoset.0, prepared) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Customization and authored display geosets, then the armor's (as the native client's
/// `equipment_geosets`).
fn npc_body_geoset_visible(mesh_part: u16, prepared: &PreparedNpcAppearance) -> bool {
    let visible = !prepared.armor.hidden_ids.contains(&mesh_part)
        && npc_geoset_visible(
            mesh_part,
            &prepared.selected_geosets,
            &prepared.authored_geosets,
        );
    apply_exact_geoset_overrides(mesh_part, visible, &prepared.armor.overrides)
}

fn apply_npc_textures(
    root: Entity,
    prepared: &PreparedNpcAppearance,
    targets: &mut NpcRenderTargets,
) -> Result<(), String> {
    for (entity, texture_type, mut handle) in &mut targets.meshes {
        if !is_body_descendant(entity, root, &targets.parents, &targets.items) {
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
                        armor: NpcArmorGeosets::default(),
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
            armor: NpcArmorGeosets::default(),
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

    /// Stockade Guard display 2989: its gloves, boots and tabard (GeosetGroup_0 = 1)
    /// switch groups 4, 5 and 12 to variant 2; its sword keeps its own meshes.
    #[test]
    fn npc_armor_geosets_switch_body_groups_but_not_equipment_models() {
        let mut app = App::new();
        app.init_resource::<Assets<M2Material>>()
            .init_resource::<Assets<Image>>();
        register_npc_appearance_systems(&mut app);
        let texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(crate::rgba_image(vec![180, 60, 30, 255], 1, 1));
        let material = app.world_mut().resource_mut::<Assets<M2Material>>().add(
            crate::retail_m2_material::retail_m2_material(StandardMaterial::default(), 0, 0),
        );
        let root = app
            .world_mut()
            .spawn(PreparedNpcAppearance {
                display_id: 2989,
                textures: HashMap::from([(1, texture.clone()), (6, texture)]),
                selected_geosets: vec![(0, 5)],
                authored_geosets: vec![],
                armor: NpcArmorGeosets {
                    overrides: vec![(4, 2), (5, 2), (20, 2), (12, 2)],
                    ..default()
                },
            })
            .id();
        let parts = [401, 402, 501, 502, 1201, 1202];
        let body: Vec<_> = parts
            .iter()
            .map(|&part| mesh(app.world_mut(), root, material.clone(), part, 0))
            .collect();
        let sword = app
            .world_mut()
            .spawn((
                EquipmentItem {
                    _slot: crate::equipment::EquipmentSlot::MainHand,
                },
                ChildOf(root),
            ))
            .id();
        let sword_mesh = mesh(app.world_mut(), sword, material.clone(), 401, 1);
        app.update();
        let visible: Vec<_> = body
            .iter()
            .map(|&entity| *app.world().get::<Visibility>(entity).unwrap() == Visibility::Inherited)
            .collect();
        assert_eq!(visible, [false, true, false, true, false, true]);
        assert_eq!(
            *app.world().get::<Visibility>(sword_mesh).unwrap(),
            Visibility::Inherited
        );
        let sword_material = &app
            .world()
            .get::<MeshMaterial3d<M2Material>>(sword_mesh)
            .unwrap()
            .0;
        assert_eq!(*sword_material, material);
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
                armor: NpcArmorGeosets::default(),
            })
            .id();
        let second = app
            .world_mut()
            .spawn(PreparedNpcAppearance {
                display_id: 19178,
                textures: HashMap::from([(1, second_texture.clone()), (6, second_texture.clone())]),
                selected_geosets: vec![(0, 5)],
                authored_geosets: vec![],
                armor: NpcArmorGeosets::default(),
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
