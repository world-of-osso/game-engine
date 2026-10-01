use std::collections::HashMap;

use shared::components::EquipmentVisualSlot;

use super::DebugCharacterConfig;

fn config(vars: &[(&str, &str)]) -> Result<DebugCharacterConfig, String> {
    let vars: HashMap<String, String> = vars
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    DebugCharacterConfig::from_lookup(|name| vars.get(name).cloned())
}

fn displays(config: &DebugCharacterConfig, left: bool) -> Vec<(EquipmentVisualSlot, u32)> {
    let side = if left { &config.left } else { &config.right };
    config
        .equipment(side)
        .entries
        .iter()
        .map(|entry| (entry.slot, entry.display_info_id.unwrap()))
        .collect()
}

#[test]
fn defaults_are_the_original_human_warrior_and_displays() {
    let config = config(&[]).unwrap();
    assert_eq!((config.race, config.class, config.appearance.sex), (1, 1, 0));
    assert_eq!(config.appearance.hair_style, 4);
    assert_eq!(
        displays(&config, true),
        [
            (EquipmentVisualSlot::Head, 1128),
            (EquipmentVisualSlot::Shoulder, 148865),
            (EquipmentVisualSlot::Back, 181925),
            (EquipmentVisualSlot::Chest, 175942),
            (EquipmentVisualSlot::Hands, 510),
            (EquipmentVisualSlot::Waist, 109162),
            (EquipmentVisualSlot::Legs, 159629),
            (EquipmentVisualSlot::Feet, 154620),
        ]
    );
    assert_eq!(
        displays(&config, false)[4..7],
        [
            (EquipmentVisualSlot::Hands, 154616),
            (EquipmentVisualSlot::Waist, 160997),
            (EquipmentVisualSlot::Legs, 73783),
        ]
    );
}

#[test]
fn variables_override_and_zero_empties_a_slot() {
    let config = config(&[
        ("DEBUG_CHARACTER_RACE", "4"),
        ("DEBUG_CHARACTER_SEX", "1"),
        ("DEBUG_CHARACTER_RIGHT_HEAD_DISPLAY", "0"),
        ("DEBUG_CHARACTER_BACK_DISPLAY", "0"),
    ])
    .unwrap();
    assert_eq!((config.race, config.appearance.sex), (4, 1));
    let right = displays(&config, false);
    assert!(!right.iter().any(|(slot, _)| *slot == EquipmentVisualSlot::Head));
    assert!(!right.iter().any(|(slot, _)| *slot == EquipmentVisualSlot::Back));
    assert_eq!(right.len(), 6);
}

#[test]
fn unparsable_values_are_errors() {
    assert!(config(&[("DEBUG_CHARACTER_RACE", "human")]).is_err());
    assert!(config(&[("DEBUG_CHARACTER_SEX", "300")]).is_err());
}
