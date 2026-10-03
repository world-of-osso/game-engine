//! Original equipment-transform configuration, independent of the renderer.

use super::EquipmentSlot;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EquipmentTransformDef {
    #[serde(default)]
    pub translation: [f32; 3],
    #[serde(default)]
    pub rotation_deg: [f32; 3],
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
}

impl Default for EquipmentTransformDef {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation_deg: [0.0; 3],
            scale: default_scale(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EquipmentTransformConfig {
    #[serde(default)]
    slot_defaults: HashMap<EquipmentSlot, EquipmentTransformDef>,
    #[serde(default)]
    item_overrides: HashMap<String, EquipmentTransformDef>,
}

impl EquipmentTransformConfig {
    pub fn parse(content: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<Self>(content).map(Self::normalized)
    }

    pub fn normalized(mut self) -> Self {
        self.item_overrides = self
            .item_overrides
            .into_iter()
            .map(|(key, value)| (key.to_ascii_lowercase(), value))
            .collect();
        self
    }

    pub fn resolve(&self, slot: EquipmentSlot, path: &Path) -> EquipmentTransformDef {
        let key = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase);
        key.as_ref()
            .and_then(|key| self.item_overrides.get(key))
            .or_else(|| self.slot_defaults.get(&slot))
            .cloned()
            .unwrap_or_default()
    }
}

fn default_scale() -> [f32; 3] {
    [1.0; 3]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::EquipmentSlot;
    use super::{EquipmentTransformConfig, EquipmentTransformDef};

    #[test]
    fn absent_fields_resolve_to_identity() {
        let config = EquipmentTransformConfig::parse("()").unwrap();
        let transform = config.resolve(EquipmentSlot::Head, Path::new("helm.m2"));
        assert_eq!(transform.translation, [0.0, 0.0, 0.0]);
        assert_eq!(transform.rotation_deg, [0.0, 0.0, 0.0]);
        assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn item_filename_override_precedes_slot_and_ignores_case() {
        let config = EquipmentTransformConfig::parse(
            r#"(
                slot_defaults: {
                    MainHand: (translation: (1.0, 2.0, 3.0), rotation_deg: (1.0, 2.0, 3.0), scale: (2.0, 2.0, 2.0)),
                },
                item_overrides: {
                    "Club_1H_Torch_A_01": (translation: (4.0, 5.0, 6.0), rotation_deg: (10.0, 20.0, 30.0), scale: (3.0, 4.0, 5.0)),
                },
            )"#,
        )
        .unwrap();
        let transform = config.resolve(
            EquipmentSlot::MainHand,
            Path::new("data/models/CLUB_1H_TORCH_A_01.M2"),
        );
        assert_eq!(transform.translation, [4.0, 5.0, 6.0]);
        assert_eq!(transform.rotation_deg, [10.0, 20.0, 30.0]);
        assert_eq!(transform.scale, [3.0, 4.0, 5.0]);
    }

    #[test]
    fn slot_default_applies_to_other_filename_and_identity_to_other_slot() {
        let config = EquipmentTransformConfig::parse(
            r#"(slot_defaults: { OffHand: (translation: (1.0, 2.0, 3.0)) })"#,
        )
        .unwrap();
        assert_eq!(
            config
                .resolve(EquipmentSlot::OffHand, Path::new("other.m2"))
                .translation,
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            config.resolve(EquipmentSlot::Head, Path::new("other.m2")),
            EquipmentTransformDef::default()
        );
    }

    #[test]
    fn invalid_ron_returns_error() {
        assert!(EquipmentTransformConfig::parse("not ron").is_err());
    }
}
