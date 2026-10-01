//! Bevy-free authored NPC customization and mesh visibility decisions.

use std::collections::HashSet;

use crate::customization_data::{CustomizationChoice, CustomizationDb};
use crate::geoset_visibility_data::{apply_exact_geoset_overrides, is_geoset_visible};
use crate::npc_appearance_data::AuthoredNpcAppearance;

#[derive(Default)]
pub struct NpcSelections {
    pub materials: Vec<(u16, u32)>,
    pub geosets: Vec<(u16, u16)>,
}

/// Applies the displayed model's choices. Choices of the race's unaltered form
/// (a Worgen profile's Gilnean-form choices) resolve but belong to the other model.
pub fn select_npc_choices(
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

pub fn resolve_npc_choices<'a>(
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

pub fn append_selected_choice(
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

pub fn npc_geoset_visible(
    mesh_part: u16,
    selected: &[(u16, u16)],
    authored: &[(u16, u16)],
) -> bool {
    if mesh_part < 100
        && let Some((_, variant)) = authored.iter().rev().find(|(group, _)| *group == 0)
    {
        return is_geoset_visible(mesh_part, &[(0, *variant)], &[0]);
    }
    let active_types: Vec<_> = selected.iter().map(|(group, _)| *group).collect();
    let visible = is_geoset_visible(mesh_part, selected, &active_types);
    apply_exact_geoset_overrides(mesh_part, visible, authored)
}

pub type NpcTexturePixels = (Vec<u8>, u32, u32);

pub fn select_npc_type6_texture(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_group_zero_last_value_wins_without_hiding_body() {
        let selected = [(0, 5), (4, 1)];
        let authored = [(0, 5), (0, 2), (4, 3)];
        assert!(npc_geoset_visible(0, &selected, &authored));
        assert!(!npc_geoset_visible(1, &selected, &authored));
        assert!(npc_geoset_visible(2, &selected, &authored));
        assert!(!npc_geoset_visible(5, &selected, &authored));
        assert!(npc_geoset_visible(403, &selected, &authored));
        assert!(!npc_geoset_visible(401, &selected, &authored));
    }

    #[test]
    fn declared_hair_must_exist_and_never_uses_head() {
        let head = (vec![1, 2, 3, 255], 1, 1);
        assert_eq!(
            select_npc_type6_texture(true, None, Some(head.clone())),
            Err("declared NPC hair target 10 did not produce a texture".to_string())
        );
        assert_eq!(
            select_npc_type6_texture(false, None, Some(head.clone())),
            Ok(Some(head))
        );
    }
}
