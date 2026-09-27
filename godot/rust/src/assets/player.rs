//! Selected-player body appearance from authored customization and local CASC.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
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
use shared::{components::CharacterAppearance, protocol::CharacterListEntry};

use super::{
    appearance::{PreparedAppearance, load_appearance_texture},
    creature::{cache_model_files, cache_model_textures, local_resolver},
    load_model_node_with_appearance,
    material::texture_from_rgba,
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
    layout_id: u32,
    load: impl FnMut(u32) -> Result<TexturePixels, String>,
) -> Result<HashMap<u32, TexturePixels>, String> {
    let default_fdid = default_player_body_fdid(compositor, layout_id)?;
    let decoded = load_required_player_pixels(&choices.materials, default_fdid, load)?;
    let composed = compositor
        .composite_model_textures_with(&choices.materials, &[], layout_id, default_fdid, |fdid| {
            decoded.get(&fdid).cloned()
        })
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
    default_fdid: u32,
    mut load: impl FnMut(u32) -> Result<TexturePixels, String>,
) -> Result<HashMap<u32, TexturePixels>, String> {
    let required: HashSet<u32> = materials
        .iter()
        .map(|(_, fdid)| *fdid)
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
    character: &CharacterListEntry,
) -> Result<Gd<Node3D>, String> {
    let resolver = local_resolver(data_root, cache_root);
    let path = cache_player_model(&resolver, data_root, character)?;
    let appearance = prepare_player_appearance(&resolver, data_root, character)?;
    let slots = [0; 3];
    let (mut model, missing) = load_model_node_with_appearance(
        &GString::from(path.to_string_lossy().as_ref()),
        &slots,
        Some(&appearance),
    )?;
    if !missing.is_empty() {
        godot_warn!(
            "Player {} missing authored texture FDIDs: {missing:?}",
            character.name
        );
    }
    mark_unsupported_player_equipment(&mut model, character);
    Ok(model)
}

fn cache_player_model(
    resolver: &CascListfileResolver,
    data_root: &Path,
    character: &CharacterListEntry,
) -> Result<PathBuf, String> {
    let race = character.race;
    let sex = character.appearance.sex;
    let wow_path = race_model_wow_path(race, sex)
        .ok_or_else(|| format!("no player model for race {race} sex {sex}"))?;
    let fdid = resolver
        .lookup_path(wow_path)
        .ok_or_else(|| format!("player model {wow_path} absent from local listfile"))?;
    let path = cache_model_files(resolver, data_root, fdid)?;
    cache_model_textures(resolver, data_root, &[0; 3], &path)?;
    Ok(path)
}

fn prepare_player_appearance(
    resolver: &CascListfileResolver,
    data_root: &Path,
    character: &CharacterListEntry,
) -> Result<PreparedAppearance, String> {
    let race = character.race;
    let sex = character.appearance.sex;
    let db = load_customization_db(data_root)?;
    let selected = select_player_choices(&db, race, sex, character.class, &character.appearance)?;
    let layout_id = db
        .layout_id(race, sex)
        .ok_or_else(|| format!("missing player texture layout for race {race} sex {sex}"))?;
    let compositor = load_compositor(data_root)?;
    let pixels = compose_player_pixels(&compositor, &selected, layout_id, |texture_fdid| {
        load_appearance_texture(resolver, data_root, texture_fdid, "player")
    })?;
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
    })
}

fn mark_unsupported_player_equipment(model: &mut Gd<Node3D>, character: &CharacterListEntry) {
    if !character.equipment_appearance.entries.is_empty() {
        model.set_meta(
            "unsupported_equipment_appearance",
            &format!("{:?}", character.equipment_appearance).to_variant(),
        );
        godot_warn!(
            "Player {} body loaded; gear appearance unsupported",
            character.name
        );
    }
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
        let composed = compose_player_pixels(&compositor, &selected, layout, |_fdid| {
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
        let result = compose_player_pixels(&compositor, &selected, layout, |fdid| {
            if fdid == missing {
                Err(format!("missing player texture FDID {fdid}"))
            } else {
                Ok((vec![200, 100, 50, 255], 1, 1))
            }
        });
        assert!(result.unwrap_err().contains(&format!("FDID {missing}")));
    }
}
