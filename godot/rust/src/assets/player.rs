//! Selected-player body appearance from authored customization and local CASC.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use crate::{
    equipment_appearance_data::{ResolvedEquipmentAppearance, resolve_equipment_appearance},
    outfit_data::OutfitData,
};
use game_engine_core::{
    asset::m2_texture,
    char_texture_data::CharTextureData,
    character_model_data::race_model_wow_path,
    customization_data::{CustomizationChoice, CustomizationDb},
    npc_appearance_assets::{load_compositor, load_customization_db},
};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;
use shared::components::{CharacterAppearance, EquipmentAppearance, Player};

use super::{
    appearance::{PreparedAppearance, load_appearance_texture},
    build_model,
    creature::{cache_model_files, cache_model_textures, local_resolver},
    equipment::attach_equipment,
    material::texture_from_rgba,
    read_model,
};

type TexturePixels = (Vec<u8>, u32, u32);

struct PlayerChoices {
    #[cfg(test)]
    choice_ids: HashSet<u32>,
    materials: Vec<(u16, u32)>,
    geosets: Vec<(u16, u16)>,
}

fn select_player_choices(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    appearance: &CharacterAppearance,
) -> Result<PlayerChoices, String> {
    db.options_for(race, sex)
        .ok_or_else(|| format!("missing player customization for race {race} sex {sex}"))?;
    let choices = crate::appearance_options::selected_choices(db, race, sex, class, appearance);
    let choice_ids: HashSet<_> = choices.iter().map(|choice| choice.id).collect();
    let materials = project_player_materials(&choices, &choice_ids);
    let geosets = project_player_geosets(&choices, &choice_ids);
    Ok(PlayerChoices {
        #[cfg(test)]
        choice_ids,
        materials,
        geosets,
    })
}

fn project_player_materials(
    choices: &[&CustomizationChoice],
    choice_ids: &HashSet<u32>,
) -> Vec<(u16, u32)> {
    choices
        .iter()
        .flat_map(|choice| {
            choice.materials.iter().copied().chain(
                choice
                    .related_materials
                    .iter()
                    .filter(|related| choice_ids.contains(&related.related_choice_id))
                    .map(|related| (related.target_id, related.fdid)),
            )
        })
        .collect()
}

fn project_player_geosets(
    choices: &[&CustomizationChoice],
    choice_ids: &HashSet<u32>,
) -> Vec<(u16, u16)> {
    choices
        .iter()
        .flat_map(|choice| {
            choice.geosets.iter().copied().chain(
                choice
                    .related_geosets
                    .iter()
                    .filter(|related| choice_ids.contains(&related.related_choice_id))
                    .map(|related| (related.geoset_type, related.geoset_id)),
            )
        })
        .collect()
}

fn compose_player_pixels(
    compositor: &CharTextureData,
    choices: &PlayerChoices,
    item_textures: &[(u8, u32)],
    layout_id: u32,
    load: impl FnMut(u32) -> Result<TexturePixels, String>,
) -> Result<HashMap<u32, TexturePixels>, String> {
    let default_fdid = default_player_body_fdid(compositor, layout_id)?;
    let decoded =
        load_required_player_pixels(&choices.materials, item_textures, default_fdid, load)?;
    let composed = compositor
        .composite_model_textures_with(
            &choices.materials,
            item_textures,
            layout_id,
            default_fdid,
            |fdid| decoded.get(&fdid).cloned(),
        )
        .ok_or_else(|| format!("cannot composite player texture layout {layout_id}"))?;
    let mut textures = HashMap::from([(1, composed.body)]);
    let type6 = select_player_head_pixels(
        compositor,
        &choices.materials,
        layout_id,
        composed.hair,
        composed.head,
    )?;
    if let Some(pixels) = type6 {
        textures.insert(6, pixels);
    }
    insert_player_eye_pixels(
        &mut textures,
        compositor,
        &choices.materials,
        layout_id,
        &decoded,
    )?;
    Ok(textures)
}

fn default_player_body_fdid(compositor: &CharTextureData, layout_id: u32) -> Result<u32, String> {
    let layout = compositor
        .layout(layout_id)
        .ok_or_else(|| format!("missing player texture layout {layout_id}"))?;
    m2_texture::default_fdid_for_type(1, layout.width == 2048 && layout.height == 1024, &[0, 0, 0])
        .ok_or_else(|| format!("missing default player body texture for layout {layout_id}"))
}

fn load_required_player_pixels(
    materials: &[(u16, u32)],
    item_textures: &[(u8, u32)],
    default_fdid: u32,
    mut load: impl FnMut(u32) -> Result<TexturePixels, String>,
) -> Result<HashMap<u32, TexturePixels>, String> {
    let required: HashSet<u32> = materials
        .iter()
        .map(|(_, fdid)| *fdid)
        .chain(item_textures.iter().map(|(_, fdid)| *fdid))
        .chain(std::iter::once(default_fdid))
        .collect();
    required
        .into_iter()
        .map(|fdid| load(fdid).map(|pixels| (fdid, pixels)))
        .collect()
}

fn select_player_head_pixels(
    compositor: &CharTextureData,
    materials: &[(u16, u32)],
    layout_id: u32,
    hair: Option<TexturePixels>,
    head: Option<TexturePixels>,
) -> Result<Option<TexturePixels>, String> {
    if compositor.declares_hair(materials, layout_id) {
        Ok(Some(hair.ok_or(
            "declared player hair target 10 did not produce a texture",
        )?))
    } else {
        Ok(head)
    }
}

fn insert_player_eye_pixels(
    textures: &mut HashMap<u32, TexturePixels>,
    compositor: &CharTextureData,
    materials: &[(u16, u32)],
    layout_id: u32,
    decoded: &HashMap<u32, TexturePixels>,
) -> Result<(), String> {
    if let Some(fdid) = compositor.replacement_texture_fdid(materials, layout_id, 19) {
        let pixels = decoded
            .get(&fdid)
            .ok_or_else(|| format!("missing player eye texture FDID {fdid}"))?;
        textures.insert(19, pixels.clone());
    }
    Ok(())
}

pub(crate) fn load_player_model(
    data_root: &Path,
    cache_root: &Path,
    player: &Player,
    equipment: &EquipmentAppearance,
) -> Result<Gd<Node3D>, String> {
    let resolver = local_resolver(data_root, cache_root);
    let path = crate::profile::time(
        || "player cache_player_model".to_owned(),
        || cache_player_model(&resolver, data_root, player),
    )?;
    let outfit = crate::profile::time(
        || "player OutfitData::load".to_owned(),
        || OutfitData::load(data_root),
    );
    let equipment =
        resolve_equipment_appearance(equipment, &outfit, player.race, player.appearance.sex)?;
    let appearance = crate::profile::time(
        || "player prepare_player_appearance".to_owned(),
        || prepare_player_appearance(&resolver, data_root, player, &equipment),
    )?;
    let path = GString::from(path.to_string_lossy().as_ref());
    let parsed = crate::profile::time(|| "player read_model".to_owned(), || read_model(&path))?;
    cache_model_textures(&resolver, data_root, &[0; 3], &parsed)?;
    let (mut model, missing) = crate::profile::time(
        || "player build_model".to_owned(),
        || build_model(&parsed, &path, &[0; 3], Some(&appearance)),
    )?;
    if !missing.is_empty() {
        godot_warn!(
            "Player {} missing authored texture FDIDs: {missing:?}",
            player.name
        );
    }
    let attached = crate::profile::time(
        || "player attach_equipment".to_owned(),
        || {
            attach_equipment(
                &mut model,
                &parsed,
                &resolver,
                data_root,
                &equipment.runtime_models,
            )
        },
    );
    if let Err(error) = attached {
        model.free();
        return Err(error);
    }
    Ok(model)
}

fn cache_player_model(
    resolver: &CascListfileResolver,
    data_root: &Path,
    player: &Player,
) -> Result<PathBuf, String> {
    let race = player.race;
    let sex = player.appearance.sex;
    let wow_path = race_model_wow_path(race, sex)
        .ok_or_else(|| format!("no player model for race {race} sex {sex}"))?;
    let fdid = resolver
        .lookup_path(wow_path)
        .ok_or_else(|| format!("player model {wow_path} absent from local listfile"))?;
    cache_model_files(resolver, data_root, fdid)
}

fn prepare_player_appearance(
    resolver: &CascListfileResolver,
    data_root: &Path,
    player: &Player,
    equipment: &ResolvedEquipmentAppearance,
) -> Result<PreparedAppearance, String> {
    let race = player.race;
    let sex = player.appearance.sex;
    let db = crate::profile::time(
        || "player load_customization_db".to_owned(),
        || load_customization_db(data_root),
    )?;
    let mut selected = select_player_choices(&db, race, sex, player.class, &player.appearance)?;
    for group in &equipment.hidden_character_geoset_groups {
        selected.geosets.retain(|(active, _)| active != group);
        let variant = if *group == 0 {
            db.scalp_fallback_hair_geoset(race, sex).unwrap_or(1)
        } else {
            1
        };
        selected.geosets.push((*group, variant));
    }
    let layout_id = db
        .layout_id(race, sex)
        .ok_or_else(|| format!("missing player texture layout for race {race} sex {sex}"))?;
    let compositor = crate::profile::time(
        || "player load_compositor".to_owned(),
        || load_compositor(data_root),
    )?;
    let mut seen = HashSet::new();
    let item_textures: Vec<_> = equipment
        .outfit
        .item_textures
        .iter()
        .copied()
        .filter(|texture| seen.insert(*texture))
        .collect();
    let span = crate::profile::span(|| "player compose_player_pixels".to_owned());
    let mut pixels = compose_player_pixels(
        &compositor,
        &selected,
        &item_textures,
        layout_id,
        |texture_fdid| load_appearance_texture(resolver, data_root, texture_fdid, "player"),
    )?;
    if let Some(fdid) = equipment.merged_cape_texture_fdid {
        pixels.insert(
            2,
            load_appearance_texture(resolver, data_root, fdid, "player cape")?,
        );
    }
    drop(span);
    let textures = pixels
        .into_iter()
        .map(|(kind, (rgba, width, height))| {
            texture_from_rgba(&rgba, width, height).map(|texture| (kind, texture))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(PreparedAppearance {
        source: "player",
        textures,
        selected_geosets: selected.geosets,
        authored_geosets: Vec::new(),
        equipment_geosets: equipment.outfit.geoset_overrides.clone(),
        hidden_geoset_ids: equipment.hidden_character_geoset_ids.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_core::{
        customization_data::OptionType, npc_appearance_assets::load_customization_db,
    };
    use shared::components::CharacterAppearance;

    fn data_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn selected_body_uses_roster_sex_class_and_core_choices() {
        let db = load_customization_db(&data_root()).unwrap();
        let mut appearance = CharacterAppearance::default();
        appearance.skin_color = 1;
        appearance.hair_style = 2;
        let chosen = select_player_choices(&db, 1, 0, 2, &appearance).unwrap();
        let expected_skin = db
            .get_choice_for_class(1, 0, 2, OptionType::SkinColor, 1)
            .unwrap();
        let expected_hair = db
            .get_choice_for_class(1, 0, 2, OptionType::HairStyle, 2)
            .unwrap();
        assert!(chosen.choice_ids.contains(&expected_skin.id));
        assert!(chosen.choice_ids.contains(&expected_hair.id));
        assert!(
            chosen
                .materials
                .iter()
                .any(|material| expected_skin.materials.contains(material))
        );
        assert!(
            chosen
                .geosets
                .iter()
                .any(|geoset| expected_hair.geosets.contains(geoset))
        );
        let female = select_player_choices(&db, 1, 1, 2, &appearance).unwrap();
        assert_ne!(chosen.choice_ids, female.choice_ids);
    }

    #[test]
    fn additional_option_retains_direct_and_related_choice_effects() {
        let db = load_customization_db(&data_root()).unwrap();
        let race = 1;
        let sex = 0;
        let class = 2;
        let option = db
            .options_for(race, sex)
            .unwrap()
            .iter()
            .filter(|option| !crate::appearance_options::is_core_option(&db, race, sex, option))
            .find(|option| {
                db.choices_for_option(race, sex, class, option.id)
                    .iter()
                    .any(|choice| !choice.materials.is_empty() || !choice.geosets.is_empty())
            })
            .unwrap();
        let choice = db
            .choices_for_option(race, sex, class, option.id)
            .into_iter()
            .find(|choice| !choice.materials.is_empty() || !choice.geosets.is_empty())
            .unwrap();
        let mut appearance = CharacterAppearance::default();
        appearance
            .customization_choices
            .push(shared::components::CustomizationChoiceSelection {
                option_id: option.id,
                choice_id: choice.id,
            });
        let selected = select_player_choices(&db, race, sex, class, &appearance).unwrap();
        assert!(selected.choice_ids.contains(&choice.id));
        for material in &choice.materials {
            assert!(selected.materials.contains(material));
        }
        for geoset in &choice.geosets {
            assert!(selected.geosets.contains(geoset));
        }
        for related in &choice.related_materials {
            if selected.choice_ids.contains(&related.related_choice_id) {
                assert!(
                    selected
                        .materials
                        .contains(&(related.target_id, related.fdid))
                );
            }
        }
        for related in &choice.related_geosets {
            if selected.choice_ids.contains(&related.related_choice_id) {
                assert!(
                    selected
                        .geosets
                        .contains(&(related.geoset_type, related.geoset_id))
                );
            }
        }
    }

    #[test]
    fn selected_texture_pixels_replace_body_and_eye_without_white_placeholder() {
        let compositor =
            game_engine_core::npc_appearance_assets::load_compositor(&data_root()).unwrap();
        let db = load_customization_db(&data_root()).unwrap();
        let selected =
            select_player_choices(&db, 1, 0, 2, &CharacterAppearance::default()).unwrap();
        let layout = db.layout_id(1, 0).unwrap();
        let composed = compose_player_pixels(&compositor, &selected, &[], layout, |_fdid| {
            Ok((vec![120, 80, 40, 255], 1, 1))
        })
        .unwrap();
        let body = &composed[&1];
        assert_eq!((body.1, body.2), (1024, 512));
        assert!(
            body.0
                .chunks_exact(4)
                .any(|pixel| pixel == [120, 80, 40, 255])
        );
    }

    fn clothing_compositor() -> CharTextureData {
        use game_engine_core::char_texture_data::{TextureLayout, TextureSection};
        CharTextureData::from_parts(
            Vec::new(),
            HashMap::from([(
                (7, 3),
                TextureSection {
                    x: 1,
                    y: 0,
                    width: 1,
                    height: 2,
                },
            )]),
            HashMap::from([(
                7,
                TextureLayout {
                    width: 2,
                    height: 2,
                },
            )]),
        )
    }

    fn empty_choices() -> PlayerChoices {
        PlayerChoices {
            choice_ids: HashSet::new(),
            materials: Vec::new(),
            geosets: Vec::new(),
        }
    }

    #[test]
    fn equipped_clothing_replaces_its_section_and_preserves_uncovered_skin() {
        let textures = compose_player_pixels(
            &clothing_compositor(),
            &empty_choices(),
            &[(3, 777)],
            7,
            |fdid| {
                Ok((
                    if fdid == 777 {
                        vec![10, 200, 30, 255]
                    } else {
                        vec![120, 80, 40, 255]
                    },
                    1,
                    1,
                ))
            },
        )
        .unwrap();
        assert_eq!(
            textures[&1].0,
            [
                120, 80, 40, 255, 10, 200, 30, 255, 120, 80, 40, 255, 10, 200, 30, 255
            ]
        );
    }

    #[test]
    fn missing_equipped_clothing_texture_fails_explicitly() {
        let result = compose_player_pixels(
            &clothing_compositor(),
            &empty_choices(),
            &[(3, 777)],
            7,
            |fdid| {
                if fdid == 777 {
                    Err("missing equipped texture FDID 777".into())
                } else {
                    Ok((vec![120, 80, 40, 255], 1, 1))
                }
            },
        );
        assert_eq!(result.unwrap_err(), "missing equipped texture FDID 777");
    }

    #[test]
    fn missing_selected_texture_fails_before_compositing() {
        let compositor =
            game_engine_core::npc_appearance_assets::load_compositor(&data_root()).unwrap();
        let db = load_customization_db(&data_root()).unwrap();
        let selected =
            select_player_choices(&db, 1, 0, 2, &CharacterAppearance::default()).unwrap();
        let layout = db.layout_id(1, 0).unwrap();
        let missing = selected.materials.first().unwrap().1;
        let result = compose_player_pixels(&compositor, &selected, &[], layout, |fdid| {
            if fdid == missing {
                Err(format!("missing player texture FDID {fdid}"))
            } else {
                Ok((vec![200, 100, 50, 255], 1, 1))
            }
        });
        assert!(result.unwrap_err().contains(&format!("FDID {missing}")));
    }
}

/// Rendered skin against the Customize swatch on the real catalog and local textures.
#[cfg(test)]
mod swatch_tests {
    use super::*;
    use crate::char_create::{CharCreateState, appearance::select_choice};
    use game_engine_core::{
        customization_data::OptionType, npc_appearance_assets::load_customization_db,
    };

    fn data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn rgb(raw: i32) -> [f64; 3] {
        let argb = raw as u32;
        [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8].map(f64::from)
    }

    /// Mean opaque colour of the body atlas' left half (arms, torso and legs).
    fn body_mean((pixels, width, height): &TexturePixels) -> [f64; 3] {
        let (mut sum, mut count) = ([0.0; 3], 0.0);
        for y in 0..*height {
            for x in 0..width / 2 {
                let pixel = &pixels[((y * width + x) * 4) as usize..][..4];
                if pixel[3] == 255 {
                    (0..3).for_each(|channel| sum[channel] += f64::from(pixel[channel]));
                    count += 1.0;
                }
            }
        }
        sum.map(|channel| channel / count)
    }

    /// Chromaticity distance (x100): lighting shades the body darker than its swatch,
    /// so hue/saturation are compared rather than brightness.
    fn chroma_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
        let (sum_a, sum_b) = (a.iter().sum::<f64>(), b.iter().sum::<f64>());
        (0..3)
            .map(|channel| (a[channel] / sum_a - b[channel] / sum_b).powi(2))
            .sum::<f64>()
            .sqrt()
            * 100.0
    }

    /// Select `skin_id` through the shared creation rules and composite the player body.
    fn rendered_body(
        db: &CustomizationDb,
        state: &mut CharCreateState,
        skin_option: u32,
        skin_id: u32,
    ) -> TexturePixels {
        select_choice(state, skin_option, skin_id, db);
        let (race, sex, class) = (
            state.selected_race,
            state.selected_sex,
            state.selected_class,
        );
        let chosen = select_player_choices(db, race, sex, class, &state.appearance).unwrap();
        let compositor =
            game_engine_core::npc_appearance_assets::load_compositor(&data_root()).unwrap();
        let resolver =
            super::super::creature::local_resolver(&data_root(), &data_root().join("cache"));
        let layout = db.layout_id(race, sex).unwrap();
        let mut textures = compose_player_pixels(&compositor, &chosen, &[], layout, |fdid| {
            load_appearance_texture(&resolver, &data_root(), fdid, "swatch test")
        })
        .unwrap_or_else(|error| panic!("race {race} sex {sex} skin {skin_id}: {error}"));
        textures.remove(&1).unwrap()
    }

    fn creation_state(race: u8, sex: u8, class: u8, db: &CustomizationDb) -> CharCreateState {
        let mut state = CharCreateState {
            selected_race: race,
            selected_sex: sex,
            selected_class: class,
            ..CharCreateState::default()
        };
        crate::char_create::randomize_appearance_with_seed(&mut state, db, 11);
        state
    }

    /// Reported: tan swatch 4978 with face 27 rendered a flat teal body because the
    /// authored target-30 overlay layer replaced the skin instead of tinting it.
    #[test]
    fn reported_tan_skin_renders_its_swatch_color() {
        let db = load_customization_db(&data_root()).unwrap();
        let mut state = creation_state(1, 0, 1, &db);
        select_choice(&mut state, 10, 15430, &db);
        let body = rendered_body(&db, &mut state, 9, 4978);
        let swatch = rgb(db.choice_by_id(1, 0, 4978).unwrap().swatch_colors[0]);
        let distance = chroma_distance(body_mean(&body), swatch);
        assert!(
            distance < 14.0,
            "body {:?} vs swatch {swatch:?}: {distance:.1}",
            body_mean(&body)
        );
    }

    #[test]
    fn every_offered_skin_renders_close_to_its_swatch() {
        let db = load_customization_db(&data_root()).unwrap();
        for (race, class) in [(1, 1), (2, 1), (3, 1), (4, 1), (10, 2)] {
            for sex in [0, 1] {
                let mut state = creation_state(race, sex, class, &db);
                let skin_option = db
                    .options_for(race, sex)
                    .unwrap()
                    .iter()
                    .filter(|option| option.option_type == OptionType::SkinColor)
                    .map(|option| option.id)
                    .min()
                    .unwrap();
                for skin in db.offered_choices(race, sex, class, skin_option) {
                    if skin.swatch_colors[0] == 0 {
                        continue;
                    }
                    let body = rendered_body(&db, &mut state, skin_option, skin.id);
                    let swatch = rgb(skin.swatch_colors[0]);
                    let distance = chroma_distance(body_mean(&body), swatch);
                    assert!(
                        distance < 14.0,
                        "race {race} sex {sex} skin {}: body {:?} vs swatch {swatch:?}: {distance:.1}",
                        skin.id,
                        body_mean(&body)
                    );
                }
            }
        }
    }
}
