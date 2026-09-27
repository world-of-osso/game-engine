//! Selected-player body appearance from authored customization and local CASC.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use game_engine_core::{
    asset::m2_texture,
    char_texture_data::CharTextureData,
    character_model_data::race_model_wow_path,
    customization_data::CustomizationDb,
    npc_appearance_assets::{load_compositor, load_customization_db},
};
use godot::{classes::Node3D, prelude::*};
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
    let materials = choices
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
        .collect();
    let geosets = choices
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
        .collect();
    Ok(PlayerChoices {
        #[cfg(test)]
        choice_ids,
        materials,
        geosets,
    })
}

fn compose_player_pixels(
    compositor: &CharTextureData,
    choices: &PlayerChoices,
    layout_id: u32,
    mut load: impl FnMut(u32) -> Result<TexturePixels, String>,
) -> Result<HashMap<u32, TexturePixels>, String> {
    let layout = compositor
        .layout(layout_id)
        .ok_or_else(|| format!("missing player texture layout {layout_id}"))?;
    let default_fdid = m2_texture::default_fdid_for_type(
        1,
        layout.width == 2048 && layout.height == 1024,
        &[0, 0, 0],
    )
    .ok_or_else(|| format!("missing default player body texture for layout {layout_id}"))?;
    let required: HashSet<u32> = choices
        .materials
        .iter()
        .map(|(_, fdid)| *fdid)
        .chain(std::iter::once(default_fdid))
        .collect();
    let decoded: HashMap<u32, TexturePixels> = required
        .into_iter()
        .map(|fdid| load(fdid).map(|pixels| (fdid, pixels)))
        .collect::<Result<_, _>>()?;
    let composed = compositor
        .composite_model_textures_with(&choices.materials, &[], layout_id, default_fdid, |fdid| {
            decoded.get(&fdid).cloned()
        })
        .ok_or_else(|| format!("cannot composite player texture layout {layout_id}"))?;
    let mut textures = HashMap::from([(1, composed.body)]);
    let type6 = if compositor.declares_hair(&choices.materials, layout_id) {
        Some(
            composed
                .hair
                .ok_or("declared player hair target 10 did not produce a texture")?,
        )
    } else {
        composed.head
    };
    if let Some(pixels) = type6 {
        textures.insert(6, pixels);
    }
    if let Some(fdid) = compositor.replacement_texture_fdid(&choices.materials, layout_id, 19) {
        let pixels = decoded
            .get(&fdid)
            .ok_or_else(|| format!("missing player eye texture FDID {fdid}"))?;
        textures.insert(19, pixels.clone());
    }
    Ok(textures)
}

pub(crate) fn load_player_model(
    data_root: &Path,
    cache_root: &Path,
    character: &CharacterListEntry,
) -> Result<Gd<Node3D>, String> {
    let race = character.race;
    let sex = character.appearance.sex;
    let wow_path = race_model_wow_path(race, sex)
        .ok_or_else(|| format!("no player model for race {race} sex {sex}"))?;
    let resolver = local_resolver(data_root, cache_root);
    let fdid = resolver
        .lookup_path(wow_path)
        .ok_or_else(|| format!("player model {wow_path} absent from local listfile"))?;
    let path = cache_model_files(&resolver, data_root, fdid)?;
    let slots = [0; 3];
    cache_model_textures(&resolver, data_root, &slots, &path)?;

    let db = load_customization_db(data_root)?;
    let selected = select_player_choices(&db, race, sex, character.class, &character.appearance)?;
    let layout_id = db
        .layout_id(race, sex)
        .ok_or_else(|| format!("missing player texture layout for race {race} sex {sex}"))?;
    let compositor = load_compositor(data_root)?;
    let pixels = compose_player_pixels(&compositor, &selected, layout_id, |texture_fdid| {
        load_appearance_texture(&resolver, data_root, texture_fdid, "player")
    })?;
    let textures = pixels
        .into_iter()
        .map(|(kind, (rgba, width, height))| {
            texture_from_rgba(&rgba, width, height).map(|texture| (kind, texture))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;
    let appearance = PreparedAppearance {
        source: "player",
        textures,
        selected_geosets: selected.geosets,
        authored_geosets: Vec::new(),
    };
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
    Ok(model)
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
