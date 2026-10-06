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

    /// Ordinary slot transform, or identity when that slot has no customization.
    pub fn resolve_slot_default(&self, slot: EquipmentSlot) -> EquipmentTransformDef {
        self.slot_defaults.get(&slot).cloned().unwrap_or_default()
    }

    /// A known filename may customize the slot; unnamed models need no artificial path.
    pub fn resolve_optional_path(
        &self,
        slot: EquipmentSlot,
        path: Option<&Path>,
    ) -> EquipmentTransformDef {
        let key = path
            .and_then(Path::file_stem)
            .and_then(|stem| stem.to_str())
            .map(str::to_ascii_lowercase);
        key.as_ref()
            .and_then(|key| self.item_overrides.get(key))
            .cloned()
            .unwrap_or_else(|| self.resolve_slot_default(slot))
    }

    pub fn resolve(&self, slot: EquipmentSlot, path: &Path) -> EquipmentTransformDef {
        self.resolve_optional_path(slot, Some(path))
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
    fn shoulder_policy_transforms_without_filename_use_slot_default_or_identity() {
        let config = EquipmentTransformConfig::parse(
            r#"(
                slot_defaults: { ShoulderLeft: (translation: (1.0, 2.0, 3.0)) },
                item_overrides: { "shoulder_special": (translation: (4.0, 5.0, 6.0)) },
            )"#,
        )
        .unwrap();
        let expected = EquipmentTransformDef {
            translation: [1.0, 2.0, 3.0],
            ..Default::default()
        };
        assert_eq!(
            config.resolve_slot_default(EquipmentSlot::ShoulderLeft),
            expected
        );
        assert_eq!(
            config.resolve_optional_path(EquipmentSlot::ShoulderLeft, None),
            expected
        );
        assert_eq!(
            config.resolve_slot_default(EquipmentSlot::ShoulderRight),
            EquipmentTransformDef::default()
        );
        assert_eq!(
            config.resolve_optional_path(EquipmentSlot::ShoulderRight, None),
            EquipmentTransformDef::default()
        );
        let named = Path::new("item/objectcomponents/shoulder/SHOULDER_SPECIAL.M2");
        let override_transform =
            config.resolve_optional_path(EquipmentSlot::ShoulderLeft, Some(named));
        assert_eq!(override_transform.translation, [4.0, 5.0, 6.0]);
        assert_eq!(
            override_transform,
            config.resolve(EquipmentSlot::ShoulderLeft, named)
        );
        assert_eq!(
            config.resolve_optional_path(EquipmentSlot::ShoulderLeft, Some(Path::new("other.m2"))),
            expected
        );
    }

    #[test]
    fn invalid_ron_returns_error() {
        assert!(EquipmentTransformConfig::parse("not ron").is_err());
    }
}
