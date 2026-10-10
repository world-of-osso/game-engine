use super::outfit_data::{OutfitData, OutfitResult};
use crate::asset::m2_format::m2_anim::M2Bone;
use game_engine_core::asset_product::{AssetProduct, AssetTexture};
use serde::{Deserialize, Serialize};
use shared::components::{EquipmentAppearance, EquipmentVisualSlot};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    ShoulderLeft,
    ShoulderRight,
    Back,
    Chest,
    Hands,
    Wrist,
    Waist,
    Legs,
    Feet,
    MainHand,
    OffHand,
    /// A creature's ranged virtual item.
    Ranged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeModelAppearance {
    pub slot: EquipmentSlot,
    pub fdid: u32,
    pub product: AssetProduct,
    pub skin_fdids: [u32; 3],
    pub skin_products: [Option<AssetProduct>; 3],
    /// ItemDisplayInfoModelMatRes (M2 texture type, texture FDID) for this model column.
    pub texture_replacements: Vec<(u32, AssetTexture)>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedEquipmentAppearance {
    pub outfit: OutfitResult,
    pub runtime_models: Vec<RuntimeModelAppearance>,
    pub merged_cape_texture_fdid: Option<u32>,
    pub merged_cape_texture: Option<AssetTexture>,
    pub body_texture_assets: Vec<(u8, AssetTexture)>,
    /// Textures needed by non-head item overlays, cloaks, and runtime models.
    pub texture_fdids: Vec<u32>,
    pub explicit_slots: HashSet<EquipmentVisualSlot>,
    pub hidden_character_geoset_groups: HashSet<u16>,
    pub hidden_character_geoset_ids: HashSet<u16>,
}

pub fn visual_slot_to_runtime_slots(slot: EquipmentVisualSlot) -> Vec<EquipmentSlot> {
    match slot {
        EquipmentVisualSlot::Head => vec![EquipmentSlot::Head],
        EquipmentVisualSlot::Shoulder => {
            vec![EquipmentSlot::ShoulderLeft, EquipmentSlot::ShoulderRight]
        }
        EquipmentVisualSlot::Back => vec![EquipmentSlot::Back],
        EquipmentVisualSlot::Chest => vec![EquipmentSlot::Chest],
        EquipmentVisualSlot::Waist => vec![EquipmentSlot::Waist],
        EquipmentVisualSlot::Legs => vec![EquipmentSlot::Legs],
        EquipmentVisualSlot::Hands => vec![EquipmentSlot::Hands],
        EquipmentVisualSlot::Wrist => vec![EquipmentSlot::Wrist],
        EquipmentVisualSlot::Feet => vec![EquipmentSlot::Feet],
        EquipmentVisualSlot::MainHand => vec![EquipmentSlot::MainHand],
        EquipmentVisualSlot::OffHand => vec![EquipmentSlot::OffHand],
        EquipmentVisualSlot::Ranged => vec![EquipmentSlot::Ranged],
        _ => Vec::new(),
    }
}

pub fn resolve_equipment_appearance(
    appearance: &EquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> Result<ResolvedEquipmentAppearance, String> {
    load_equipment_appearance_checked(appearance, outfit_data, race, sex, false)
}

/// An authored NPC bake already paints body armor; retain its models and geosets.
pub fn load_baked_equipment_appearance(
    appearance: &EquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> Result<ResolvedEquipmentAppearance, String> {
    load_equipment_appearance_checked(appearance, outfit_data, race, sex, true)
}

fn load_equipment_appearance_checked(
    appearance: &EquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
    baked_body: bool,
) -> Result<ResolvedEquipmentAppearance, String> {
    let mut failure = None;
    let resolved =
        load_equipment_entries(appearance, outfit_data, race, sex, baked_body, |error| {
            if failure.is_none() {
                failure = Some(error);
            }
        });
    match failure {
        Some(error) => Err(error),
        None => Ok(resolved),
    }
}

/// Keep valid slots visible when a separate equipped item cannot be resolved.
pub fn resolve_equipment_appearance_with_errors(
    appearance: &EquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
    report_error: impl FnMut(String),
) -> ResolvedEquipmentAppearance {
    load_equipment_entries(appearance, outfit_data, race, sex, false, report_error)
}

fn load_equipment_entries(
    appearance: &EquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
    baked_body: bool,
    mut report_error: impl FnMut(String),
) -> ResolvedEquipmentAppearance {
    let mut resolved = ResolvedEquipmentAppearance::default();
    let mut body = BodyDisplays::default();
    for entry in &appearance.entries {
        resolved.explicit_slots.insert(entry.slot);
        if entry.hidden {
            continue;
        }
        let entry_data = match entry.definition_source {
            Some(shared::item_data::ItemDefinitionSource::Retail) => {
                outfit_data.load_owned_retail()
            }
            Some(shared::item_data::ItemDefinitionSource::Forever70205) => {
                outfit_data.load_owned_forever_70205()
            }
            None if entry.item_id.is_none() => Ok(outfit_data),
            None => Err("owned item missing definition source".to_string()),
        };
        let entry_data = match entry_data {
            Ok(data) => data,
            Err(error) => {
                report_error(format!(
                    "Equipment {:?} source {:?}: {error}",
                    entry.slot, entry.definition_source
                ));
                continue;
            }
        };
        let display_info_id = match (entry.display_info_id, entry.item_id) {
            (Some(display_id), _) => display_id,
            (None, Some(item_id)) => match entry_data.resolve_item_display_id(item_id) {
                Ok(display_id) => display_id,
                Err(error) => {
                    report_error(format!(
                        "Equipment {:?} source {:?} item {item_id}: {error}",
                        entry.slot, entry.definition_source
                    ));
                    continue;
                }
            },
            (None, None) => continue,
        };
        match apply_visible_entry(
            &mut resolved,
            entry.slot,
            display_info_id,
            entry_data,
            race,
            sex,
            baked_body,
        ) {
            Ok(textures) => body.record(entry.slot, display_info_id, entry_data, textures),
            Err(error) => report_error(format!(
                "Equipment {:?} source {:?} display {display_info_id}: {error}",
                entry.slot, entry.definition_source
            )),
        }
    }
    apply_body_geosets(&mut resolved, &body);
    layer_item_textures(&mut resolved, &body);
    resolved
}

/// The displays of the slots whose items paint the body and pick its sleeve, robe and
/// leg geosets.
#[derive(Default)]
struct BodyDisplays<'a> {
    /// Texture-bearing `CCharacterComponent` slot rows (see [`ITEM_PRIORITIES`]) with
    /// their displays and body textures, in equip order.
    painted: Vec<(usize, u32, &'a OutfitData, Vec<(u8, AssetTexture)>)>,
    shirt: Option<(u32, &'a OutfitData)>,
    chest: Option<(u32, &'a OutfitData)>,
    legs: Option<(u32, &'a OutfitData)>,
    hands: Option<(u32, &'a OutfitData)>,
}

impl<'a> BodyDisplays<'a> {
    fn record(
        &mut self,
        slot: EquipmentVisualSlot,
        display_id: u32,
        data: &'a OutfitData,
        textures: Vec<(u8, AssetTexture)>,
    ) {
        match slot {
            EquipmentVisualSlot::Shirt => self.shirt = Some((display_id, data)),
            EquipmentVisualSlot::Chest => self.chest = Some((display_id, data)),
            EquipmentVisualSlot::Legs => self.legs = Some((display_id, data)),
            EquipmentVisualSlot::Hands => self.hands = Some((display_id, data)),
            _ => {}
        }
        if let Some(row) = component_slot_row(slot) {
            self.painted.push((row, display_id, data, textures));
        }
    }
}

/// The robe and sleeves decided across slots, after every item's own overrides, in the
/// order of build 12340's `CCharacterComponent` (solarityclient
/// `character_component/geoset.rs` `apply_equipment_geosets`): gloves (GeosetGroup[0])
/// take the arms, else the chest's, else the shirt's GeosetGroup[0] picks sleeves 801+n
/// and their GeosetGroup[1] the undershirt 1001+n (wowdev.wiki DB/ItemDisplayInfo); a
/// chest robe (GeosetGroup[2], inventory type 20), else a legs one, hides boots 5xx,
/// kneepads 902-999 and pants 11xx and shows skirt 1301+n in place of the pants' trousers.
fn apply_body_geosets(resolved: &mut ResolvedEquipmentAppearance, body: &BodyDisplays<'_>) {
    let group = |display: Option<(u32, &OutfitData)>, index| {
        display.and_then(|(display, data)| data.display_geoset_variant(display, index))
    };
    let overrides = &mut resolved.outfit.geoset_overrides;
    let mut set = |geoset: u16, variant: u16| {
        overrides.retain(|(existing, _)| *existing != geoset);
        overrides.push((geoset, variant));
    };
    let torso = |index| group(body.chest, index).or_else(|| group(body.shirt, index));
    if group(body.hands, 0).is_none()
        && let Some(sleeves) = torso(0)
    {
        set(8, sleeves);
    }
    if let Some(undershirt) = torso(1) {
        set(10, undershirt);
    }
    let Some(robe) = group(body.chest, 2).or_else(|| group(body.legs, 2)) else {
        return;
    };
    set(9, 1);
    set(13, robe);
    resolved
        .outfit
        .geoset_overrides
        .retain(|(geoset, _)| !matches!(geoset, 5 | 11));
    resolved
        .hidden_character_geoset_ids
        .extend((501..=599).chain(1100..=1199));
}

/// Build 12340 `CCharacterComponent` item texture paste priority: rows head, shoulder,
/// shirt, chest, waist, legs, feet, wrist, hands, tabard; columns body sections ArmUpper,
/// ArmLower, Hand, TorsoUpper, TorsoLower, LegUpper, LegLower, Foot; -1 never pastes
/// (solarityclient `character_component/atlas.rs` `ITEM_PRIORITIES`).
const ITEM_PRIORITIES: [[i8; 8]; 10] = [
    [-1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1],
    [0, 0, -1, 0, 0, -1, -1, -1],
    [1, 1, -1, 1, 1, 1, 1, -1],
    [-1, -1, -1, -1, 5, 2, -1, -1],
    [-1, -1, -1, -1, -1, 0, 0, -1],
    [-1, -1, -1, -1, -1, -1, 2, 0],
    [-1, 2, -1, -1, -1, -1, -1, -1],
    [-1, 3, 0, -1, -1, -1, -1, -1],
    [-1, -1, -1, 4, 4, -1, -1, -1],
];

fn component_slot_row(slot: EquipmentVisualSlot) -> Option<usize> {
    Some(match slot {
        EquipmentVisualSlot::Head => 0,
        EquipmentVisualSlot::Shoulder => 1,
        EquipmentVisualSlot::Shirt => 2,
        EquipmentVisualSlot::Chest => 3,
        EquipmentVisualSlot::Waist => 4,
        EquipmentVisualSlot::Legs => 5,
        EquipmentVisualSlot::Feet => 6,
        EquipmentVisualSlot::Wrist => 7,
        EquipmentVisualSlot::Hands => 8,
        EquipmentVisualSlot::Tabard => 9,
        _ => return None,
    })
}

/// Paste priority of slot `row`'s texture in body `section`, with the stock sleeve, robe
/// and boot adjustments (atlas.rs `adjusted_item_priority`); none where it never pastes.
fn item_texture_priority(
    row: usize,
    section: usize,
    display: u32,
    data: &OutfitData,
) -> Option<i8> {
    let base = *ITEM_PRIORITIES.get(row)?.get(section)?;
    let has = |index| data.display_geoset_variant(display, index).is_some();
    let priority = match (row, section) {
        (3, 1) if has(0) => 5,
        (8, 1) if has(0) => 6,
        (3, 6) if has(2) => 4,
        (6, 6) if has(0) => 3,
        _ => base,
    };
    (priority >= 0).then_some(priority)
}

/// Order the body item textures by paste priority, whatever the equip order, so a robe
/// paints over the shirt and the pants: the compositor pastes them in list order.
fn layer_item_textures(resolved: &mut ResolvedEquipmentAppearance, body: &BodyDisplays<'_>) {
    let priority = |texture: &(u8, AssetTexture)| {
        body.painted
            .iter()
            .rev()
            .find(|(_, _, _, textures)| textures.contains(texture))
            .and_then(|&(row, display, data, _)| {
                item_texture_priority(row, usize::from(texture.0), display, data)
            })
            .unwrap_or(i8::MAX)
    };
    resolved.body_texture_assets.sort_by_key(priority);
    resolved.outfit.item_textures = resolved
        .body_texture_assets
        .iter()
        .map(|&(section, texture)| (section, texture.fdid))
        .collect();
}

fn apply_visible_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
    baked_body: bool,
) -> Result<Vec<(u8, AssetTexture)>, String> {
    let display = if baked_body {
        outfit_data.try_load_baked_display_info(display_info_id, race, sex)?
    } else {
        outfit_data.try_resolve_display_info(display_info_id, race, sex)?
    };
    let mut display = display.ok_or_else(|| format!("display {display_info_id} missing"))?;
    let textures = if baked_body {
        Vec::new()
    } else {
        outfit_data.load_source_item_textures(display_info_id, race, sex)?
    };
    resolved
        .body_texture_assets
        .extend(textures.iter().copied());
    if slot == EquipmentVisualSlot::Head {
        let has_vis_data = outfit_data.has_helmet_geoset_vis_data(display_info_id);
        resolved
            .hidden_character_geoset_groups
            .extend(outfit_data.helmet_hide_geoset_groups(display_info_id, race));
        apply_geoset_overrides(
            &mut display,
            outfit_data.head_geoset_overrides(display_info_id),
        );
        merge_overlay_texture_sets(&mut resolved.outfit, &display);
        let models =
            runtime_slot_models(outfit_data, display_info_id, EquipmentSlot::Head, race, sex)?;
        if !models.is_empty() && !has_vis_data {
            resolved.hidden_character_geoset_groups.insert(0);
        }
        resolved.runtime_models.extend(models);
        return Ok(textures);
    }
    if slot == EquipmentVisualSlot::Back {
        if let Some(texture) = outfit_data
            .load_source_skin_textures(display_info_id, race, sex)?
            .first()
            .copied()
        {
            resolved.merged_cape_texture = Some(texture);
            resolved.merged_cape_texture_fdid = Some(texture.fdid);
            resolved.texture_fdids.push(texture.fdid);
        }
    }
    if let Some(overrides) = slot_geoset_overrides(slot, display_info_id, outfit_data) {
        apply_geoset_overrides(&mut display, overrides);
    }
    resolved
        .texture_fdids
        .extend(display.item_textures.iter().map(|(_, fdid)| *fdid));
    merge_overlay_texture_sets(&mut resolved.outfit, &display);
    for runtime_slot in visual_slot_to_runtime_slots(slot) {
        for model in runtime_slot_models(outfit_data, display_info_id, runtime_slot, race, sex)? {
            resolved
                .texture_fdids
                .extend(model.skin_fdids.into_iter().filter(|fdid| *fdid != 0));
            resolved.texture_fdids.extend(
                model
                    .texture_replacements
                    .iter()
                    .map(|&(_, texture)| texture.fdid),
            );
            resolved.runtime_models.push(model);
        }
    }
    Ok(textures)
}

/// The models display `display_info_id` puts in `slot`: one shoulder per side; both
/// model columns of a helmet, cloak, belt or bracer (a belt's buckle and its collection,
/// a cloak's model in either column); a weapon's or body armor's first model.
fn runtime_slot_models(
    outfit_data: &OutfitData,
    display_info_id: u32,
    slot: EquipmentSlot,
    race: u8,
    sex: u8,
) -> Result<Vec<RuntimeModelAppearance>, String> {
    let models = runtime_model_columns(outfit_data, display_info_id, slot, race, sex)?;
    models
        .into_iter()
        .map(|(column, fdid, skin_fdids)| {
            let (product, material_product) =
                outfit_data.load_column_products(display_info_id, column)?;
            let skin_products = runtime_skin_products(
                outfit_data,
                display_info_id,
                slot,
                material_product,
                &skin_fdids,
                race,
                sex,
            )?;
            Ok(RuntimeModelAppearance {
                slot,
                fdid,
                product,
                skin_fdids,
                skin_products,
                texture_replacements: outfit_data.load_source_model_textures(
                    display_info_id,
                    column,
                    race,
                    sex,
                )?,
            })
        })
        .collect()
}

fn runtime_skin_products(
    data: &OutfitData,
    display: u32,
    slot: EquipmentSlot,
    column_product: Option<AssetProduct>,
    skins: &[u32; 3],
    race: u8,
    sex: u8,
) -> Result<[Option<AssetProduct>; 3], String> {
    if matches!(
        slot,
        EquipmentSlot::Head
            | EquipmentSlot::Back
            | EquipmentSlot::Waist
            | EquipmentSlot::Wrist
            | EquipmentSlot::ShoulderLeft
            | EquipmentSlot::ShoulderRight
    ) {
        return Ok(skins.map(|fdid| (fdid != 0).then_some(column_product).flatten()));
    }
    let textures = data.load_source_skin_textures(display, race, sex)?;
    Ok(std::array::from_fn(|index| {
        textures.get(index).map(|texture| texture.product)
    }))
}

fn runtime_model_columns(
    outfit_data: &OutfitData,
    display_info_id: u32,
    slot: EquipmentSlot,
    race: u8,
    sex: u8,
) -> Result<Vec<(usize, u32, [u32; 3])>, String> {
    Ok(match slot {
        EquipmentSlot::ShoulderLeft | EquipmentSlot::ShoulderRight => {
            let side = usize::from(slot == EquipmentSlot::ShoulderRight);
            let model =
                outfit_data.resolve_shoulder_runtime_model(display_info_id, side, race, sex);
            match model {
                Some((fdid, skins)) => vec![(
                    outfit_data.shoulder_model_column(display_info_id, side)?,
                    fdid,
                    skins,
                )],
                None => Vec::new(),
            }
        }
        EquipmentSlot::Head | EquipmentSlot::Back | EquipmentSlot::Waist | EquipmentSlot::Wrist => {
            outfit_data.try_resolve_column_models(display_info_id, race, sex)?
        }
        _ => {
            let Some((fdid, skins)) =
                outfit_data.try_resolve_runtime_model(display_info_id, race, sex)?
            else {
                return Ok(Vec::new());
            };
            let display = outfit_data
                .load_display_info(display_info_id)?
                .ok_or_else(|| format!("Display {display_info_id} missing"))?;
            let column = display
                .model_resource_columns
                .iter()
                .position(|resource| *resource != 0)
                .ok_or_else(|| {
                    format!("Display {display_info_id}: resolved model has no source column")
                })?;
            vec![(column, fdid, skins)]
        }
    })
}

fn merge_overlay_texture_sets(base: &mut OutfitResult, overlay: &OutfitResult) {
    for &texture in &overlay.item_textures {
        if !base.item_textures.contains(&texture) {
            base.item_textures.push(texture);
        }
    }
    for &(group, value) in &overlay.geoset_overrides {
        base.geoset_overrides
            .retain(|(existing, _)| *existing != group);
        base.geoset_overrides.push((group, value));
    }
    for &model in &overlay.model_fdids {
        if !base.model_fdids.contains(&model) {
            base.model_fdids.push(model);
        }
    }
}

/// The body geoset groups an item's `ItemDisplayInfo.GeosetGroup[index]` selects
/// (wowdev.wiki DB/ItemDisplayInfo "Geoset Group Field Meaning"): geoset
/// `group * 100 + 1 + value`; a group of 0 keeps the body's own (the hidden-cloak display
/// 146518 has GeosetGroup[0] 0 and no cape). Shirt/chest sleeves (8xx) and undershirt (10xx) and the
/// robe (13xx) are decided across slots ([`apply_body_geosets`]); the helmet in
/// `head_geoset_overrides`; boots' feet (20xx) in [`feet_geoset`].
fn slot_geoset_groups(slot: EquipmentVisualSlot) -> &'static [(usize, u16)] {
    match slot {
        EquipmentVisualSlot::Shoulder => &[(0, 26)],
        EquipmentVisualSlot::Chest => &[(3, 22), (4, 28)],
        EquipmentVisualSlot::Waist => &[(0, 18)],
        EquipmentVisualSlot::Legs => &[(0, 11), (1, 9), (2, 13)],
        EquipmentVisualSlot::Feet => &[(0, 5)],
        EquipmentVisualSlot::Hands => &[(0, 4), (1, 23)],
        EquipmentVisualSlot::Back => &[(0, 15)],
        EquipmentVisualSlot::Tabard => &[(0, 12)],
        _ => &[],
    }
}

fn slot_geoset_overrides(
    slot: EquipmentVisualSlot,
    display_id: u32,
    data: &OutfitData,
) -> Option<Vec<(u16, u16)>> {
    let mut overrides: Vec<(u16, u16)> = slot_geoset_groups(slot)
        .iter()
        .filter_map(|&(index, group)| {
            data.display_geoset_variant(display_id, index)
                .map(|variant| (group, variant))
        })
        .collect();
    if slot == EquipmentVisualSlot::Feet {
        overrides.push(feet_geoset(display_id, data));
    }
    (!overrides.is_empty()).then_some(overrides)
}

/// Boots' feet: 2000 + GeosetGroup[1], or 2002 when it is 0 (wowdev.wiki: "If you are
/// wearing boots and geosetGroup[1] for your boots is 0, you get 2002"); bare feet keep
/// the body's 2001.
fn feet_geoset(display_id: u32, data: &OutfitData) -> (u16, u16) {
    match data.display_geoset_raw(display_id, 1) {
        Some(raw) if raw > 0 => (20, raw),
        _ => (20, 2),
    }
}

fn apply_geoset_overrides(display: &mut OutfitResult, overrides: Vec<(u16, u16)>) {
    for (group, value) in overrides {
        display
            .geoset_overrides
            .retain(|(existing, _)| *existing != group);
        display.geoset_overrides.push((group, value));
    }
}

/// WoW authored attachment lookup ID for a runtime slot.
pub fn slot_attachment_id(slot: EquipmentSlot) -> u32 {
    match slot {
        EquipmentSlot::Head => 11,
        EquipmentSlot::ShoulderLeft => 6,
        EquipmentSlot::ShoulderRight => 5,
        EquipmentSlot::Back => 12,
        EquipmentSlot::Chest => unreachable!("chest runtime models anchor on the character root"),
        EquipmentSlot::Hands => unreachable!("hands runtime models anchor on the character root"),
        EquipmentSlot::Wrist => unreachable!("wrist runtime models anchor on the character root"),
        EquipmentSlot::Waist => 53,
        EquipmentSlot::Legs => unreachable!("legs runtime models anchor on the character root"),
        EquipmentSlot::Feet => unreachable!("feet runtime models anchor on the character root"),
        EquipmentSlot::MainHand => 1,
        EquipmentSlot::OffHand => 2,
        EquipmentSlot::Ranged => 1,
    }
}

pub fn is_collection_model(path: &Path) -> bool {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .contains("item/objectcomponents/collections/")
}

/// Body armor and skeletal collections bind to character joints. Attachment
/// models can also live under `collections/`: an independent non-key skeleton
/// without the slot's body geosets stays on its authored attachment.
pub fn slot_uses_bound_joints(
    slot: EquipmentSlot,
    m2_path: &Path,
    bones: &[M2Bone],
    mesh_parts: impl IntoIterator<Item = u16>,
) -> bool {
    let body_slot = matches!(
        slot,
        EquipmentSlot::Chest
            | EquipmentSlot::Hands
            | EquipmentSlot::Wrist
            | EquipmentSlot::Legs
            | EquipmentSlot::Feet
    );
    if body_slot {
        return true;
    }
    if is_attachment_local_model(slot, bones, mesh_parts) {
        return false;
    }
    is_collection_model(m2_path)
}

/// Named shoulders retain their existing collection policy. Unnamed shoulders must
/// prove attachment-local geometry; an unknown skeletal model needs authored identity.
pub fn shoulder_uses_bound_joints(
    slot: EquipmentSlot,
    model: &game_engine_core::m2::Model,
    m2_path: Option<&Path>,
) -> Result<bool, String> {
    if !matches!(
        slot,
        EquipmentSlot::ShoulderLeft | EquipmentSlot::ShoulderRight
    ) {
        return Err(format!(
            "Shoulder binding requested for non-shoulder slot {slot:?}"
        ));
    }
    if let Some(path) = m2_path {
        return Ok(slot_uses_bound_joints(
            slot,
            path,
            &model.bones,
            model.submeshes.iter().map(|part| part.mesh_part_id),
        ));
    }
    if is_attachment_local_shoulder(model) {
        return Ok(false);
    }
    Err(format!(
        "Unnamed {slot:?} model is not proven attachment-local; skeletal shoulder binding requires an authored model path"
    ))
}

fn is_attachment_local_shoulder(model: &game_engine_core::m2::Model) -> bool {
    let untransformed_root = matches!(
        model.bones.as_slice(),
        [M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: -1,
            ..
        }]
    );
    let base_mesh_only =
        !model.submeshes.is_empty() && model.submeshes.iter().all(|part| part.mesh_part_id == 0);
    let root_influence_only =
        !model.vertices.is_empty() && model.vertices.iter().all(vertex_uses_only_root);
    let unauthored_pose = matches!(model.bone_tracks.as_slice(), [tracks]
        if track_is_unauthored(&tracks.translation)
            && track_is_unauthored(&tracks.rotation)
            && track_is_unauthored(&tracks.scale));
    untransformed_root
        && base_mesh_only
        && root_influence_only
        && unauthored_pose
        && model.skeleton_fdid.is_none()
}

fn vertex_uses_only_root(vertex: &game_engine_core::m2::Vertex) -> bool {
    let total_weight: u16 = vertex.bone_weights.iter().copied().map(u16::from).sum();
    let only_root = vertex
        .bone_weights
        .iter()
        .zip(vertex.bone_indices)
        .all(|(&weight, bone)| weight == 0 || bone == 0);
    total_weight == u16::from(u8::MAX) && only_root
}

fn track_is_unauthored<T>(track: &crate::asset::m2_format::m2_anim::AnimTrack<T>) -> bool {
    // Missing external animations retain empty per-sequence entries, not an empty
    // outer array. Reject them too: absence of loaded keys is not absence of tracks.
    track.global_sequence == -1 && track.sequences.is_empty()
}

fn is_attachment_local_model(
    slot: EquipmentSlot,
    bones: &[M2Bone],
    mesh_parts: impl IntoIterator<Item = u16>,
) -> bool {
    let own_root = matches!(
        bones.first(),
        Some(M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: -1,
            ..
        })
    );
    let own_skeleton = own_root && bones.iter().all(|bone| bone.key_bone_id == -1);
    let mut parts = mesh_parts.into_iter().peekable();
    let has_meshes = parts.peek().is_some();
    let body_geosets = parts.any(|part| collection_mesh_part_in_slot(slot, part));
    own_skeleton && has_meshes && !body_geosets
}

/// Whether a collection model's mesh part belongs to `slot`: a collection file is shared
/// by several slots' items, each showing the geoset groups of its slot (WMVx
/// `MergedEquipmentGeosetModifier`; body armor keeps the groups it had); all of the
/// slot's parts show, whatever the body's variant (the
/// Tauren heritage belt collection has only 1801 while the belt selects 1802).
pub fn collection_mesh_part_in_slot(slot: EquipmentSlot, mesh_part_id: u16) -> bool {
    let groups: &[u16] = match slot {
        EquipmentSlot::Head => &[21, 24, 27, 37],
        EquipmentSlot::ShoulderLeft | EquipmentSlot::ShoulderRight => &[26],
        EquipmentSlot::Back => &[15],
        EquipmentSlot::Chest => &[22],
        EquipmentSlot::Hands => &[4],
        // WMVx wristbands 8xx; the Fanciful Corsage collection's bracer is 2301.
        EquipmentSlot::Wrist => &[8, 23],
        EquipmentSlot::Waist => &[18],
        EquipmentSlot::Legs => &[11, 13],
        EquipmentSlot::Feet => &[5, 20],
        EquipmentSlot::MainHand | EquipmentSlot::OffHand | EquipmentSlot::Ranged => &[],
    };
    groups.contains(&(mesh_part_id / 100))
}

pub fn model_attachment_id(slot: EquipmentSlot, path: &Path) -> u32 {
    let is_shield = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|category| category.to_str())
        .is_some_and(|category| category.eq_ignore_ascii_case("shield"));
    if slot == EquipmentSlot::OffHand && is_shield {
        return 0;
    }
    slot_attachment_id(slot)
}

pub fn runtime_mesh_part_allowed(slot: EquipmentSlot, mesh_part_id: u16) -> bool {
    if mesh_part_id / 100 == 17 {
        return false;
    }
    match slot {
        EquipmentSlot::Chest => mesh_part_id / 100 == 22,
        EquipmentSlot::Waist => mesh_part_id == 0 || mesh_part_id / 100 == 18,
        EquipmentSlot::Legs => matches!(mesh_part_id / 100, 11 | 13),
        EquipmentSlot::Hands => mesh_part_id / 100 == 4,
        EquipmentSlot::Feet => matches!(mesh_part_id / 100, 5 | 20),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::EquippedAppearanceEntry;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    fn outfit_data() -> &'static OutfitData {
        static OUTFIT_DATA: OnceLock<OutfitData> = OnceLock::new();
        OUTFIT_DATA.get_or_init(|| OutfitData::load(&data_dir()))
    }

    fn data_dir() -> PathBuf {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo = if env!("CARGO_PKG_NAME") == "game-engine" {
            manifest.to_path_buf()
        } else {
            manifest.join("../..")
        };
        repo.join("data")
    }

    fn entry(slot: EquipmentVisualSlot, item_id: u32) -> EquippedAppearanceEntry {
        EquippedAppearanceEntry {
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type: 0,
            hidden: false,
        }
    }

    fn resolve(entries: Vec<EquippedAppearanceEntry>) -> ResolvedEquipmentAppearance {
        resolve_equipment_appearance(&EquipmentAppearance { entries }, outfit_data(), 1, 0).unwrap()
    }

    #[test]
    fn baked_appearance_ailee_keeps_boot_geosets_without_unused_body_overlay() {
        let data = data_dir();
        let gear = crate::npc_gear_data::NpcGearData::load(&data.join("db2/12.1.0.69933")).unwrap();
        let armor = gear.display_armor(136968).unwrap();
        let outfits = outfit_data();
        let error = resolve_equipment_appearance(&armor, outfits, 95, 1).unwrap_err();
        assert!(error.contains("1102747"), "{error}");
        let baked = load_baked_equipment_appearance(&armor, outfits, 95, 1).unwrap();
        assert!(baked.outfit.item_textures.is_empty());
        assert!(baked.runtime_models.is_empty());
        assert!(baked.outfit.geoset_overrides.contains(&(5, 3)));
        assert!(baked.outfit.geoset_overrides.contains(&(20, 2)));
    }

    #[test]
    fn baked_appearance_keeps_original_grove_ranger_models_and_geosets() {
        let data = data_dir();
        let gear = crate::npc_gear_data::NpcGearData::load(&data.join("db2/12.1.0.69933")).unwrap();
        let armor = gear.display_armor(139403).unwrap();
        let outfits = outfit_data();
        let plain = resolve_equipment_appearance(&armor, outfits, 95, 0).unwrap();
        let baked = load_baked_equipment_appearance(&armor, outfits, 95, 0).unwrap();
        assert_eq!(baked.runtime_models, plain.runtime_models);
        assert_eq!(baked.outfit.geoset_overrides, plain.outfit.geoset_overrides);
        assert_eq!(
            baked.hidden_character_geoset_ids,
            plain.hidden_character_geoset_ids
        );
        assert!(baked.outfit.item_textures.is_empty());
        for (slot, fdid) in [
            (EquipmentSlot::ShoulderLeft, 7579617),
            (EquipmentSlot::ShoulderRight, 7579618),
        ] {
            assert!(baked.runtime_models.iter().any(|model| model.slot == slot
                && model.fdid == fdid
                && model.skin_fdids == [7731197, 0, 0]));
        }
    }

    #[test]
    fn starter_items_match_their_explicit_display_and_keep_model_fdids() {
        let data = outfit_data();
        for (item_id, slot) in [
            (25, EquipmentVisualSlot::MainHand),
            (38, EquipmentVisualSlot::Shirt),
            (39, EquipmentVisualSlot::Legs),
            (40, EquipmentVisualSlot::Feet),
            (2362, EquipmentVisualSlot::OffHand),
        ] {
            let item = entry(slot, item_id);
            let display = data.resolve_item_display_id(item_id).unwrap();
            let actual = resolve(vec![item.clone()]);
            let expected = resolve(vec![EquippedAppearanceEntry {
                definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                display_info_id: Some(display),
                ..item
            }]);
            assert!(
                !actual.outfit.item_textures.is_empty() || !actual.runtime_models.is_empty(),
                "item {item_id} has no visual assets"
            );
            assert_eq!(
                actual.outfit.item_textures, expected.outfit.item_textures,
                "item {item_id}"
            );
            assert_eq!(
                actual.outfit.geoset_overrides, expected.outfit.geoset_overrides,
                "item {item_id}"
            );
            assert_eq!(
                actual.runtime_models, expected.runtime_models,
                "item {item_id}"
            );
            assert!(actual.runtime_models.iter().all(|model| model.fdid > 0));
        }
    }

    #[test]
    fn hidden_and_explicit_display_precedence() {
        let mut hidden = entry(EquipmentVisualSlot::MainHand, 25);
        hidden.hidden = true;
        hidden.item_id = Some(u32::MAX);
        let result = resolve(vec![hidden]);
        assert!(
            result
                .explicit_slots
                .contains(&EquipmentVisualSlot::MainHand)
        );
        assert!(result.runtime_models.is_empty());
        assert!(result.outfit.item_textures.is_empty());
        let display = outfit_data().resolve_item_display_id(25).unwrap();
        let mut explicit = entry(EquipmentVisualSlot::MainHand, u32::MAX);
        explicit.display_info_id = Some(display);
        let result = resolve(vec![explicit]);
        assert_eq!(
            result.runtime_models,
            resolve(vec![entry(EquipmentVisualSlot::MainHand, 25)]).runtime_models
        );
        assert!(!result.runtime_models.is_empty());
    }

    #[test]
    fn missing_item_reports_error_instead_of_silent_fallback() {
        let data = outfit_data();
        let appearance = EquipmentAppearance {
            entries: vec![entry(EquipmentVisualSlot::OffHand, u32::MAX)],
        };
        let error = resolve_equipment_appearance(&appearance, data, 1, 0).unwrap_err();
        assert!(
            error.contains("OffHand") && error.contains(&u32::MAX.to_string()),
            "{error}"
        );
    }

    #[test]
    fn authored_attachment_and_bound_joint_selection() {
        let shield = Path::new("data/item-models/item/objectcomponents/shield/shield.m2");
        let sword = Path::new("data/item-models/item/objectcomponents/weapon/sword.m2");
        let collection = Path::new("data/item-models/item/objectcomponents/collections/helm.m2");
        assert_eq!(model_attachment_id(EquipmentSlot::OffHand, shield), 0);
        assert_eq!(model_attachment_id(EquipmentSlot::OffHand, sword), 2);
        assert_eq!(model_attachment_id(EquipmentSlot::MainHand, sword), 1);
        assert!(slot_uses_bound_joints(
            EquipmentSlot::Head,
            collection,
            &[],
            [2101]
        ));
        assert!(!slot_uses_bound_joints(
            EquipmentSlot::Head,
            sword,
            &[],
            [0]
        ));
        assert!(slot_uses_bound_joints(
            EquipmentSlot::Chest,
            sword,
            &[],
            [0]
        ));
    }

    fn parsed_shoulder(fdid: u32) -> game_engine_core::m2::Model {
        let root = data_dir().join("models");
        let bytes = std::fs::read(root.join(format!("{fdid}.m2"))).unwrap();
        let references = game_engine_core::m2::parse_asset_references(&bytes).unwrap();
        let skin = std::fs::read(root.join(format!("{}.skin", references.skin_fdids[0]))).unwrap();
        game_engine_core::m2::parse_model(&bytes, &skin).unwrap()
    }

    #[test]
    fn shoulder_policy_original_unnamed_models_use_authored_side_attachments() {
        for (fdid, slot, attachment) in [
            (7_579_617, EquipmentSlot::ShoulderLeft, 6),
            (7_579_618, EquipmentSlot::ShoulderRight, 5),
        ] {
            let model = parsed_shoulder(fdid);
            let bound = shoulder_uses_bound_joints(slot, &model, None).unwrap();
            assert!(!bound, "original shoulder {fdid} must be attachment-local");
            assert_eq!(slot_attachment_id(slot), attachment);
            assert!(
                model
                    .submeshes
                    .iter()
                    .all(|part| runtime_mesh_part_allowed(slot, part.mesh_part_id))
            );
        }
        // Splitting influence among entries for the same root changes no vertex;
        // zero-weight indices likewise contribute nothing to the attachment pose.
        let mut model = parsed_shoulder(7_579_617);
        model.vertices[0].bone_weights = [128, 127, 0, 0];
        model.vertices[0].bone_indices = [0, 0, 255, 255];
        assert_eq!(
            shoulder_uses_bound_joints(EquipmentSlot::ShoulderLeft, &model, None),
            Ok(false)
        );
    }

    #[test]
    fn shoulder_policy_rejects_ambiguous_roots_weights_and_parts() {
        for invalid in 0..14 {
            let mut model = parsed_shoulder(7_579_617);
            match invalid {
                0 => model.bones[0].key_bone_id = 0,
                1 => model.bones[0].flags = 0x200,
                2 => model.bones[0].parent_bone_id = 0,
                3 => model.bones.push(model.bones[0].clone()),
                4 => model.bones.clear(),
                5 => {
                    model.vertices[0].bone_weights = [128, 127, 0, 0];
                    model.vertices[0].bone_indices[1] = 1;
                }
                6 => model.vertices[0].bone_indices[0] = 1,
                7 => model.vertices[0].bone_weights = [0; 4],
                8 => model.vertices.clear(),
                9 => model.submeshes[0].mesh_part_id = 2601,
                10 => model.submeshes.clear(),
                11 => model.bone_tracks = Vec::new().into(),
                12 => model.skeleton_fdid = Some(1),
                13 => model.vertices[0].bone_weights = [254, 0, 0, 0],
                _ => unreachable!(),
            }
            let error = shoulder_uses_bound_joints(EquipmentSlot::ShoulderLeft, &model, None)
                .expect_err(&format!(
                    "ambiguous structure {invalid} must not acquire a mount"
                ));
            assert!(
                error.contains("ShoulderLeft") && error.contains("Unnamed"),
                "{error}"
            );
        }
    }

    #[test]
    fn shoulder_policy_rejects_authored_tracks_even_when_constant_or_unloaded() {
        for component in 0..3 {
            for unloaded in [false, true] {
                let mut model = parsed_shoulder(7_579_617);
                let tracks = std::sync::Arc::make_mut(&mut model.bone_tracks);
                let timeline = if unloaded { Vec::new() } else { vec![0] };
                match component {
                    0 => tracks[0].translation.sequences.push((
                        timeline,
                        if unloaded {
                            vec![]
                        } else {
                            vec![[1.0, 0.0, 0.0]]
                        },
                    )),
                    1 => tracks[0].rotation.sequences.push((
                        timeline,
                        if unloaded {
                            vec![]
                        } else {
                            vec![[32767, 32767, 32767, -1]]
                        },
                    )),
                    2 => tracks[0]
                        .scale
                        .sequences
                        .push((timeline, if unloaded { vec![] } else { vec![[1.0; 3]] })),
                    _ => unreachable!(),
                }
                assert!(
                    shoulder_uses_bound_joints(EquipmentSlot::ShoulderRight, &model, None).is_err(),
                    "component {component}, unloaded {unloaded}"
                );
            }
        }
        let mut model = parsed_shoulder(7_579_617);
        std::sync::Arc::make_mut(&mut model.bone_tracks)[0]
            .translation
            .global_sequence = 0;
        assert!(shoulder_uses_bound_joints(EquipmentSlot::ShoulderRight, &model, None).is_err());
    }

    #[test]
    fn shoulder_policy_named_skeletal_collection_keeps_existing_binding() {
        let root = data_dir().join("models");
        let model = game_engine_core::m2::parse_model(
            &std::fs::read(root.join("1360753.m2")).unwrap(),
            &std::fs::read(root.join("136075300.skin")).unwrap(),
        )
        .unwrap();
        assert!(model.bones.len() > 1);
        let path = Path::new(
            "item/objectcomponents/collections/collections_leather_raidroguemythic_q_01_hu_m.m2",
        );
        for slot in [EquipmentSlot::ShoulderLeft, EquipmentSlot::ShoulderRight] {
            assert_eq!(
                shoulder_uses_bound_joints(slot, &model, Some(path)),
                Ok(true)
            );
            assert!(shoulder_uses_bound_joints(slot, &model, None).is_err());
            assert!(collection_mesh_part_in_slot(slot, 2601));
            assert!(!collection_mesh_part_in_slot(slot, 0));
        }
    }

    #[test]
    fn shoulder_policy_is_not_a_generic_unnamed_asset_policy() {
        let model = parsed_shoulder(7_579_617);
        for path in [None, Some(Path::new("weapon.m2"))] {
            assert!(shoulder_uses_bound_joints(EquipmentSlot::MainHand, &model, path).is_err());
        }
    }

    #[test]
    fn rigid_waist_base_mesh_uses_authored_attachment_53() {
        // Zaralda display 138959: FDID 6378872, SKIN part 0; all 1839 vertices
        // reference this single attachment-local root, not a character joint.
        let root = rigid_waist_root();
        let mesh_parts = [0];
        let path =
            Path::new("item/objectcomponents/collections/belt_leather_questbloodelf_b_01.m2");
        let bound = slot_uses_bound_joints(EquipmentSlot::Waist, path, &[root], mesh_parts);
        let attachment = (!bound).then(|| model_attachment_id(EquipmentSlot::Waist, path));
        assert_eq!(attachment, Some(53));
        assert!(runtime_mesh_part_allowed(EquipmentSlot::Waist, 0));
    }

    fn rigid_waist_root() -> M2Bone {
        M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: -1,
            submesh_id: 0,
            name_crc: 3_962_896_125,
            pivot: [0.014494737, 0.0, -0.20134047],
        }
    }

    #[test]
    fn skeletal_waist_collections_keep_binding_and_group_18() {
        let path = Path::new("item/objectcomponents/collections/belt.m2");
        let root = rigid_waist_root();
        for parts in [vec![1801], vec![1802], vec![0, 1801], vec![]] {
            assert!(slot_uses_bound_joints(
                EquipmentSlot::Waist,
                path,
                &[root.clone()],
                parts
            ));
        }
        for bone in [
            M2Bone {
                flags: 0x200,
                ..root.clone()
            },
            M2Bone {
                parent_bone_id: 0,
                ..root.clone()
            },
            M2Bone {
                key_bone_id: 0,
                ..root.clone()
            },
        ] {
            assert!(slot_uses_bound_joints(
                EquipmentSlot::Waist,
                path,
                &[bone],
                [0]
            ));
        }
        // An independent skeleton may have multiple roots (e.g. shoulder emitters).
        assert!(!slot_uses_bound_joints(
            EquipmentSlot::Waist,
            path,
            &[root.clone(), root],
            [0]
        ));
        assert!(slot_uses_bound_joints(EquipmentSlot::Waist, path, &[], [0]));
        assert!(collection_mesh_part_in_slot(EquipmentSlot::Waist, 1801));
        assert!(collection_mesh_part_in_slot(EquipmentSlot::Waist, 1802));
        assert!(!collection_mesh_part_in_slot(EquipmentSlot::Waist, 0));
    }

    #[test]
    fn attachment_local_skeleton_keeps_body_slots_bound() {
        let path = Path::new("item/objectcomponents/collections/belt.m2");
        for slot in [
            EquipmentSlot::Chest,
            EquipmentSlot::Hands,
            EquipmentSlot::Wrist,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
        ] {
            assert!(slot_uses_bound_joints(
                slot,
                path,
                &[rigid_waist_root()],
                [0]
            ));
        }
        for slot in [
            EquipmentSlot::Head,
            EquipmentSlot::ShoulderLeft,
            EquipmentSlot::ShoulderRight,
            EquipmentSlot::Back,
            EquipmentSlot::MainHand,
            EquipmentSlot::OffHand,
            EquipmentSlot::Ranged,
        ] {
            assert!(!slot_uses_bound_joints(
                slot,
                path,
                &[rigid_waist_root()],
                [0]
            ));
        }
    }

    #[test]
    fn runtime_mesh_filters_body_slots_without_hiding_weapon_parts() {
        assert!(runtime_mesh_part_allowed(EquipmentSlot::Chest, 2201));
        assert!(!runtime_mesh_part_allowed(EquipmentSlot::Chest, 1801));
        assert!(runtime_mesh_part_allowed(EquipmentSlot::Feet, 2001));
        assert!(!runtime_mesh_part_allowed(EquipmentSlot::Feet, 1701));
        assert!(runtime_mesh_part_allowed(EquipmentSlot::MainHand, 1801));
        assert!(!runtime_mesh_part_allowed(EquipmentSlot::MainHand, 1701));
    }

    /// Stockade Guard display 2989's authored armor (Extra 1274): gloves 9449, boots 6229
    /// and tabard 6255 each have ItemDisplayInfo GeosetGroup_0 = 1.
    #[test]
    fn stockade_guard_armor_switches_glove_boot_and_tabard_geosets() {
        let display = |slot, id| EquippedAppearanceEntry {
            definition_source: None,
            display_info_id: Some(id),
            item_id: None,
            ..entry(slot, 0)
        };
        let armor = resolve(vec![
            display(EquipmentVisualSlot::Hands, 9449),
            display(EquipmentVisualSlot::Feet, 6229),
            display(EquipmentVisualSlot::Tabard, 6255),
        ]);
        for geoset in [(4, 2), (5, 2), (20, 2), (12, 2)] {
            assert!(
                armor.outfit.geoset_overrides.contains(&geoset),
                "{geoset:?} in {:?}",
                armor.outfit.geoset_overrides
            );
        }
    }

    /// The mage starter set (showcase gear.json), equipped in grant order: Apprentice's
    /// Robe 56 (display 12647, GeosetGroup 1/0/1), Pants 1395, Shirt 6096, Boots 55. The
    /// robe shows sleeves 802 and skirt 1302 over the pants' legs and hides boots and
    /// kneepads, and its body textures paste over the shirt's and the pants'.
    #[test]
    fn apprentice_robe_covers_the_pants_and_shirt() {
        let robe = entry(EquipmentVisualSlot::Chest, 56);
        let others = [
            entry(EquipmentVisualSlot::Legs, 1395),
            entry(EquipmentVisualSlot::Shirt, 6096),
            entry(EquipmentVisualSlot::Feet, 55),
        ];
        let robe_textures = resolve(vec![robe.clone()]).outfit.item_textures;
        assert!(!robe_textures.is_empty());
        let mut after_robe = vec![robe.clone()];
        after_robe.extend(others.iter().cloned());
        let mut before_robe = others.to_vec();
        before_robe.push(robe);
        for entries in [after_robe, before_robe] {
            let set = resolve(entries);
            let overrides = &set.outfit.geoset_overrides;
            for geoset in [(8, 2), (13, 2), (9, 1)] {
                assert!(overrides.contains(&geoset), "{geoset:?} in {overrides:?}");
            }
            assert!(
                !overrides.iter().any(|(group, _)| matches!(group, 5 | 11)),
                "{overrides:?}"
            );
            for id in [501, 502, 1101, 1102] {
                assert!(set.hidden_character_geoset_ids.contains(&id), "{id}");
            }
            // The last paste of every section the robe paints is the robe's.
            for &(section, fdid) in &robe_textures {
                let last = set
                    .outfit
                    .item_textures
                    .iter()
                    .rev()
                    .find(|(s, _)| *s == section);
                assert_eq!(
                    last,
                    Some(&(section, fdid)),
                    "{:?}",
                    set.outfit.item_textures
                );
            }
        }
    }

    /// wowdev.wiki DB/ItemDisplayInfo: boots select feet 2000 + GeosetGroup[1] (2002 when
    /// it is 0), a shirt's GeosetGroup[1] the undershirt 1001+n, a chest's GeosetGroup[3]
    /// the torso 2201+n. Recruit's Boots 40 (all groups 0), Dirt-Trodden Boots 4936
    /// (GeosetGroup 0/1), Sacredite's Research Tunic 241267 (shirt 1/3), Empyrial
    /// Breastplate 151576 (GeosetGroup[3] 1), Apprentice's Robe 56 (1/0/1/0).
    #[test]
    fn item_geoset_groups_select_their_body_groups() {
        let overrides = |slot, item| resolve(vec![entry(slot, item)]).outfit.geoset_overrides;
        assert!(overrides(EquipmentVisualSlot::Feet, 40).contains(&(20, 2)));
        assert!(
            !overrides(EquipmentVisualSlot::Feet, 40)
                .iter()
                .any(|(g, _)| *g == 5)
        );
        assert!(overrides(EquipmentVisualSlot::Feet, 4936).contains(&(20, 1)));
        let tunic = overrides(EquipmentVisualSlot::Shirt, 241267);
        assert!(
            tunic.contains(&(8, 2)) && tunic.contains(&(10, 4)),
            "{tunic:?}"
        );
        assert!(overrides(EquipmentVisualSlot::Chest, 151576).contains(&(22, 2)));
        assert!(
            !overrides(EquipmentVisualSlot::Chest, 56)
                .iter()
                .any(|(g, _)| *g == 22)
        );
    }

    /// Ancestral Chieftain's Greatbelt 168296 (display 180643): buckle 2429565 in model
    /// column 0 and the Tauren collection (male 2429554) in column 1; Fanciful Corsage
    /// 190091: a bracer collection, now a runtime model of the wrist.
    #[test]
    fn belt_keeps_buckle_and_collection_and_bracers_have_models() {
        let tauren = |entries| {
            resolve_equipment_appearance(&EquipmentAppearance { entries }, outfit_data(), 6, 0)
                .unwrap()
                .runtime_models
        };
        let belt: Vec<_> = tauren(vec![entry(EquipmentVisualSlot::Waist, 168296)])
            .iter()
            .map(|model| (model.slot, model.fdid))
            .collect();
        assert_eq!(
            belt,
            [
                (EquipmentSlot::Waist, 2_429_565),
                (EquipmentSlot::Waist, 2_429_554)
            ]
        );
        let bracer = tauren(vec![entry(EquipmentVisualSlot::Wrist, 190091)]);
        assert_eq!(bracer.len(), 1);
        assert_eq!(bracer[0].slot, EquipmentSlot::Wrist);
        let collection = Path::new("item/objectcomponents/collections/belt.m2");
        assert!(slot_uses_bound_joints(
            EquipmentSlot::Waist,
            collection,
            &[],
            [1801]
        ));
        assert!(slot_uses_bound_joints(
            EquipmentSlot::Back,
            collection,
            &[],
            [1501]
        ));
        assert!(!slot_uses_bound_joints(
            EquipmentSlot::Waist,
            Path::new("item/objectcomponents/waist/buckle.m2"),
            &[],
            [0]
        ));
    }

    #[test]
    fn shoulders_helm_and_cloak_keep_authored_decisions() {
        let shoulders = resolve(vec![EquippedAppearanceEntry {
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: Some(7004),
            ..entry(EquipmentVisualSlot::Shoulder, 1)
        }]);
        assert_eq!(shoulders.runtime_models.len(), 2);
        assert_ne!(
            shoulders.runtime_models[0].fdid,
            shoulders.runtime_models[1].fdid
        );
        let helm = resolve(vec![EquippedAppearanceEntry {
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: Some(1128),
            ..entry(EquipmentVisualSlot::Head, 1)
        }]);
        assert_eq!(helm.runtime_models.len(), 1);
        assert!(helm.hidden_character_geoset_groups.contains(&0));
        let cloak = resolve(vec![EquippedAppearanceEntry {
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: Some(192786),
            ..entry(EquipmentVisualSlot::Back, 188846)
        }]);
        assert_eq!(cloak.merged_cape_texture_fdid, Some(4046074));
        assert!(cloak.runtime_models.is_empty());
        assert!(cloak.outfit.geoset_overrides.contains(&(15, 2)));
    }
}
