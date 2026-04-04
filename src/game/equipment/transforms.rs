use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

use super::EquipmentSlot;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct EquipmentTransformDef {
    #[serde(default)]
    translation: [f32; 3],
    #[serde(default)]
    rotation_deg: [f32; 3],
    #[serde(default = "default_scale")]
    scale: [f32; 3],
}

impl Default for EquipmentTransformDef {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: default_scale(),
        }
    }
}

impl EquipmentTransformDef {
    pub(super) fn as_transform(&self) -> Transform {
        let [rx, ry, rz] = self.rotation_deg;
        Transform {
            translation: Vec3::new(
                self.translation[0],
                self.translation[1],
                self.translation[2],
            ),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                rx.to_radians(),
                ry.to_radians(),
                rz.to_radians(),
            ),
            scale: Vec3::new(self.scale[0], self.scale[1], self.scale[2]),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub(super) struct EquipmentTransformConfig {
    #[serde(default)]
    slot_defaults: HashMap<EquipmentSlot, EquipmentTransformDef>,
    #[serde(default)]
    item_overrides: HashMap<String, EquipmentTransformDef>,
}

#[derive(Resource, Debug, Clone)]
pub(super) struct EquipmentTransforms {
    slot_defaults: HashMap<EquipmentSlot, Transform>,
    item_overrides: HashMap<String, Transform>,
}

impl Default for EquipmentTransforms {
    fn default() -> Self {
        let mut slot_defaults = HashMap::new();
        slot_defaults.insert(EquipmentSlot::Head, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::ShoulderLeft, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::ShoulderRight, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::Back, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::Chest, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::Hands, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::Waist, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::Legs, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::Feet, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::MainHand, Transform::IDENTITY);
        slot_defaults.insert(EquipmentSlot::OffHand, Transform::IDENTITY);
        Self {
            slot_defaults,
            item_overrides: HashMap::new(),
        }
    }
}

impl EquipmentTransforms {
    pub(super) fn load_from_disk() -> Self {
        let path = Path::new("data/equipment_transforms.ron");
        let Ok(content) = std::fs::read_to_string(path) else {
            info!(
                "Equipment transform config not found at {}, using defaults",
                path.display()
            );
            return Self::default();
        };
        match ron::de::from_str::<EquipmentTransformConfig>(&content) {
            Ok(config) => Self::from_config(config),
            Err(e) => {
                warn!(
                    "Failed to parse {}: {e}. Using default equipment transforms",
                    path.display()
                );
                Self::default()
            }
        }
    }

    pub(super) fn from_config(config: EquipmentTransformConfig) -> Self {
        let mut result = Self::default();
        for (slot, def) in config.slot_defaults {
            result.slot_defaults.insert(slot, def.as_transform());
        }
        result.item_overrides = config
            .item_overrides
            .into_iter()
            .map(|(key, def)| (key.to_ascii_lowercase(), def.as_transform()))
            .collect();
        result
    }

    pub(super) fn resolve(&self, slot: EquipmentSlot, path: &Path) -> Transform {
        let key = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if let Some(transform) = self.item_overrides.get(&key) {
            return *transform;
        }
        self.slot_defaults
            .get(&slot)
            .copied()
            .unwrap_or(Transform::IDENTITY)
    }
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
