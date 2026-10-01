use super::outfit_data::{OutfitData, OutfitResult};
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
    pub skin_fdids: [u32; 3],
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedEquipmentAppearance {
    pub outfit: OutfitResult,
    pub runtime_models: Vec<RuntimeModelAppearance>,
    pub merged_cape_texture_fdid: Option<u32>,
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
    let mut failure = None;
    let resolved =
        resolve_equipment_appearance_with_errors(appearance, outfit_data, race, sex, |error| {
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
    mut report_error: impl FnMut(String),
) -> ResolvedEquipmentAppearance {
    let mut resolved = ResolvedEquipmentAppearance::default();
    let mut body = BodyDisplays::default();
    for entry in &appearance.entries {
        resolved.explicit_slots.insert(entry.slot);
        if entry.hidden {
            continue;
        }
        let display_info_id = match (entry.display_info_id, entry.item_id) {
            (Some(display_id), _) => display_id,
            (None, Some(item_id)) => match outfit_data.resolve_item_display_id(item_id) {
                Ok(display_id) => display_id,
                Err(error) => {
                    report_error(format!(
                        "Equipment {:?} item {item_id}: {error}",
                        entry.slot
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
            outfit_data,
            race,
            sex,
        ) {
            Ok(textures) => body.record(entry.slot, display_info_id, textures),
            Err(error) => report_error(format!(
                "Equipment {:?} display {display_info_id}: {error}",
                entry.slot
            )),
        }
    }
    apply_body_geosets(&mut resolved, &body, outfit_data);
    layer_item_textures(&mut resolved, &body, outfit_data);
    resolved
}

/// The displays of the slots whose items paint the body and pick its sleeve, robe and
/// leg geosets.
#[derive(Default)]
struct BodyDisplays {
    /// Texture-bearing `CCharacterComponent` slot rows (see [`ITEM_PRIORITIES`]) with
    /// their displays and body textures, in equip order.
    painted: Vec<(usize, u32, Vec<(u8, u32)>)>,
    chest: Option<u32>,
    legs: Option<u32>,
    hands: Option<u32>,
}

impl BodyDisplays {
    fn record(&mut self, slot: EquipmentVisualSlot, display_id: u32, textures: Vec<(u8, u32)>) {
        match slot {
            EquipmentVisualSlot::Chest => self.chest = Some(display_id),
            EquipmentVisualSlot::Legs => self.legs = Some(display_id),
            EquipmentVisualSlot::Hands => self.hands = Some(display_id),
            _ => {}
        }
        if let Some(row) = component_slot_row(slot) {
            self.painted.push((row, display_id, textures));
        }
    }
}

/// The robe and sleeves decided across slots, after every item's own overrides, in the
/// order of build 12340's `CCharacterComponent` (solarityclient
/// `character_component/geoset.rs` `apply_equipment_geosets`): gloves (GeosetGroup[0])
/// take the arms, else the chest's GeosetGroup[0] picks sleeves 801+n; a chest robe
/// (GeosetGroup[2], inventory type 20), else a legs one, hides boots 5xx, kneepads
/// 902-999 and pants 11xx and shows skirt 1301+n in place of the pants' trousers.
fn apply_body_geosets(
    resolved: &mut ResolvedEquipmentAppearance,
    body: &BodyDisplays,
    data: &OutfitData,
) {
    let group = |display: Option<u32>, index| {
        display.and_then(|display| data.display_geoset_variant(display, index))
    };
    let overrides = &mut resolved.outfit.geoset_overrides;
    let mut set = |geoset: u16, variant: u16| {
        overrides.retain(|(existing, _)| *existing != geoset);
        overrides.push((geoset, variant));
    };
    if group(body.hands, 0).is_none()
        && let Some(sleeves) = group(body.chest, 0)
    {
        set(8, sleeves);
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
fn layer_item_textures(
    resolved: &mut ResolvedEquipmentAppearance,
    body: &BodyDisplays,
    data: &OutfitData,
) {
    let priority = |texture: &(u8, u32)| {
        body.painted
            .iter()
            .rev()
            .find(|(_, _, textures)| textures.contains(texture))
            .and_then(|&(row, display, _)| {
                item_texture_priority(row, usize::from(texture.0), display, data)
            })
            .unwrap_or(i8::MAX)
    };
    resolved.outfit.item_textures.sort_by_key(priority);
}

fn apply_visible_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> Result<Vec<(u8, u32)>, String> {
    let mut display = outfit_data
        .try_resolve_display_info(display_info_id, race, sex)?
        .ok_or_else(|| format!("display {display_info_id} missing"))?;
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
        if let Some((fdid, skin_fdids)) =
            outfit_data.try_resolve_runtime_model(display_info_id, race, sex)?
        {
            if !has_vis_data {
                resolved.hidden_character_geoset_groups.insert(0);
            }
            resolved.runtime_models.push(RuntimeModelAppearance {
                slot: EquipmentSlot::Head,
                fdid,
                skin_fdids,
            });
        }
        return Ok(display.item_textures);
    }
    if slot == EquipmentVisualSlot::Back {
        if let Some(fdid) = outfit_data.cape_texture_fdid(display_info_id, race, sex) {
            resolved.merged_cape_texture_fdid = Some(fdid);
            resolved.texture_fdids.push(fdid);
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
        let model = match runtime_slot {
            EquipmentSlot::ShoulderLeft => {
                outfit_data.resolve_shoulder_runtime_model(display_info_id, 0, race, sex)
            }
            EquipmentSlot::ShoulderRight => {
                outfit_data.resolve_shoulder_runtime_model(display_info_id, 1, race, sex)
            }
            _ => outfit_data.try_resolve_runtime_model(display_info_id, race, sex)?,
        };
        if let Some((fdid, skin_fdids)) = model {
            resolved
                .texture_fdids
                .extend(skin_fdids.into_iter().filter(|fdid| *fdid != 0));
            resolved.runtime_models.push(RuntimeModelAppearance {
                slot: runtime_slot,
                fdid,
                skin_fdids,
            });
        }
    }
    Ok(display.item_textures)
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

fn slot_geoset_overrides(
    slot: EquipmentVisualSlot,
    display_id: u32,
    data: &OutfitData,
) -> Option<Vec<(u16, u16)>> {
    match slot {
        EquipmentVisualSlot::Chest => {
            single_geoset_override(22, data.chest_geoset_variant(display_id))
        }
        EquipmentVisualSlot::Hands => {
            single_geoset_override(4, data.hand_geoset_variant(display_id))
        }
        EquipmentVisualSlot::Waist => {
            single_geoset_override(18, data.hand_geoset_variant(display_id))
        }
        EquipmentVisualSlot::Legs => legs_geoset_overrides(display_id, data),
        EquipmentVisualSlot::Back => {
            single_geoset_override(15, data.cape_geoset_variant(display_id))
        }
        EquipmentVisualSlot::Tabard => {
            single_geoset_override(12, data.tabard_geoset_variant(display_id))
        }
        EquipmentVisualSlot::Feet => data
            .boot_geoset_variant(display_id)
            .map(|variant| vec![(5, variant), (20, variant)]),
        _ => None,
    }
}

fn legs_geoset_overrides(display_id: u32, data: &OutfitData) -> Option<Vec<(u16, u16)>> {
    let mut overrides = Vec::new();
    push_optional_geoset_override(&mut overrides, 11, data.pants_geoset_variant(display_id));
    push_optional_geoset_override(&mut overrides, 9, data.kneepad_geoset_variant(display_id));
    push_optional_geoset_override(&mut overrides, 13, data.trouser_geoset_variant(display_id));
    (!overrides.is_empty()).then_some(overrides)
}

fn single_geoset_override(group: u16, variant: Option<u16>) -> Option<Vec<(u16, u16)>> {
    variant.map(|value| vec![(group, value)])
}

fn push_optional_geoset_override(
    overrides: &mut Vec<(u16, u16)>,
    group: u16,
    variant: Option<u16>,
) {
    if let Some(value) = variant {
        overrides.push((group, value));
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

pub fn slot_uses_bound_joints(slot: EquipmentSlot, m2_path: &Path) -> bool {
    matches!(
        slot,
        EquipmentSlot::Chest | EquipmentSlot::Hands | EquipmentSlot::Legs | EquipmentSlot::Feet
    ) || (slot == EquipmentSlot::Head && is_collection_model(m2_path))
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
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type: 0,
            hidden: false,
        }
    }

    fn resolve(entries: Vec<EquippedAppearanceEntry>) -> ResolvedEquipmentAppearance {
        resolve_equipment_appearance(
            &EquipmentAppearance { entries },
            &OutfitData::load(&data_dir()),
            1,
            0,
        )
        .unwrap()
    }

    #[test]
    fn starter_items_match_their_explicit_display_and_keep_model_fdids() {
        let data = OutfitData::load(&data_dir());
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
        let display = OutfitData::load(&data_dir())
            .resolve_item_display_id(25)
            .unwrap();
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
        let data = OutfitData::load(&data_dir());
        let appearance = EquipmentAppearance {
            entries: vec![entry(EquipmentVisualSlot::OffHand, u32::MAX)],
        };
        let error = resolve_equipment_appearance(&appearance, &data, 1, 0).unwrap_err();
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
        assert!(slot_uses_bound_joints(EquipmentSlot::Head, collection));
        assert!(!slot_uses_bound_joints(EquipmentSlot::Head, sword));
        assert!(slot_uses_bound_joints(EquipmentSlot::Chest, sword));
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

    #[test]
    fn shoulders_helm_and_cloak_keep_authored_decisions() {
        let shoulders = resolve(vec![EquippedAppearanceEntry {
            display_info_id: Some(7004),
            ..entry(EquipmentVisualSlot::Shoulder, 1)
        }]);
        assert_eq!(shoulders.runtime_models.len(), 2);
        assert_ne!(
            shoulders.runtime_models[0].fdid,
            shoulders.runtime_models[1].fdid
        );
        let helm = resolve(vec![EquippedAppearanceEntry {
            display_info_id: Some(1128),
            ..entry(EquipmentVisualSlot::Head, 1)
        }]);
        assert_eq!(helm.runtime_models.len(), 1);
        assert!(helm.hidden_character_geoset_groups.contains(&0));
        let cloak = resolve(vec![EquippedAppearanceEntry {
            display_info_id: Some(192786),
            ..entry(EquipmentVisualSlot::Back, 188846)
        }]);
        assert_eq!(cloak.merged_cape_texture_fdid, Some(4046074));
        assert!(cloak.runtime_models.is_empty());
        assert!(cloak.outfit.geoset_overrides.contains(&(15, 2)));
    }
}
