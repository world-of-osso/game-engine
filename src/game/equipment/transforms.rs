use std::path::Path;

use bevy::prelude::*;

use super::EquipmentSlot;
#[path = "equipment_transform_data.rs"]
mod equipment_transform_data;
pub(super) use equipment_transform_data::{EquipmentTransformConfig, EquipmentTransformDef};

impl EquipmentTransformDef {
    pub(super) fn as_transform(&self) -> Transform {
        let [rx, ry, rz] = self.rotation_deg;
        Transform {
            translation: Vec3::from_array(self.translation),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                rx.to_radians(),
                ry.to_radians(),
                rz.to_radians(),
            ),
            scale: Vec3::from_array(self.scale),
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub(super) struct EquipmentTransforms(EquipmentTransformConfig);

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
        match EquipmentTransformConfig::parse(&content) {
            Ok(config) => Self::from_config(config),
            Err(error) => {
                warn!(
                    "Failed to parse {}: {error}. Using default equipment transforms",
                    path.display()
                );
                Self::default()
            }
        }
    }

    pub(super) fn from_config(config: EquipmentTransformConfig) -> Self {
        Self(config.normalized())
    }

    pub(super) fn resolve(&self, slot: EquipmentSlot, path: &Path) -> Transform {
        self.0.resolve(slot, path).as_transform()
    }
}
