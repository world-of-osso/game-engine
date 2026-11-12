//! The graphics quality presets: one row of distance and shadow settings per level, read
//! by the consumers that used to hold them as constants. Retail keeps the same layer as
//! data (Part A §1.2 of the retail-optimisation mapping: 14 blocks of `{name, int32}`
//! CVar pairs), and its preset writes CVars rather than scaling them.
//!
//! `High`, the default, is this client's distances and splits before presets existed,
//! with retail's largest shadow atlas and its unit shadow radius. The lower rows scale
//! each `High` distance by the ratio of two retail rows: block 11 (`shadowMode` 2,
//! `shadowNumCascades` 2, the cascade count `High` already used) to block 2 (`Medium`)
//! or block 0 (`Low`). The retail values are VERIFIED (decoded blocks); applying their
//! ratios to this client's own distances is INFERRED.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GraphicsQuality {
    Low,
    Medium,
    #[default]
    High,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityPreset {
    /// World camera far plane, in yards.
    pub far_clip: f32,
    /// Distance from the camera within which WDL horizon tiles draw, in yards.
    pub horizon_range: f32,
    /// Directional shadow splits; 0 casts no sun shadows.
    pub shadow_cascades: u8,
    /// Directional shadow atlas size, in texels.
    pub shadow_map_size: u32,
    /// Distance from the camera within which drawn remote units cast shadows, in yards.
    pub unit_shadow_radius: f32,
    /// Distance from the camera within which opaque scenery doodads cast shadows, in
    /// yards; independent of their fade band.
    pub scenery_shadow_radius: f32,
}

/// Retail block 11 `farClip` and `horizonStart`, the anchor of the `High` row.
const RETAIL_ANCHOR: (f32, f32) = (6000.0, 1400.0);

const HIGH: QualityPreset = QualityPreset {
    // Bevy's perspective far plane, kept by the Godot port (godot/rust/src/camera.rs).
    far_clip: 1000.0,
    // `farclip` 777 × `horizonFarclipScale` 4 (solarityclient cvar definitions).
    horizon_range: 777.0 * 4.0,
    // Retail writes `shadowNumCascades 2` for shadow quality Medium
    // (`graphicsShadowQuality 2`, `_retail_/WTF/Config.wtf`).
    shadow_cascades: 2,
    // Retail `shadowTextureSize` 2048 at shadowMode 3-4, its largest validated size
    // (Part A §2.1; Godot's default, which the client inherited, is 4096).
    shadow_map_size: 2048,
    // Retail `entityShadowFadeScale` default 50 (Part A §10.1, VERIFIED); that it is a
    // radius in yards is INFERRED, its consumer was not located.
    unit_shadow_radius: 50.0,
    // Retail keeps the scenery fade (`lodObjectFadeScale` 100) and the shadow fade
    // (`entityShadowFadeScale` 50) as separate scales (Part A §1.8/§10.1, VERIFIED).
    // Applying the shadow radius to scenery is INFERRED: at the receipt poses it removes
    // 195 (outdoor) and 517 (indoor) of ~920 shadow draws with no visible ground change.
    scenery_shadow_radius: 50.0,
};

/// `High` with retail's `farClip`/`horizonStart` ratio to its anchor, the shadow
/// mode's cascade count and map size (`shadowMode` 0: blob shadows, which this client
/// does not draw; 1: one band, 1024) and the unit shadow radius.
const fn scaled(
    far_clip: f32,
    horizon_start: f32,
    shadow_cascades: u8,
    unit_shadow_radius: f32,
) -> QualityPreset {
    QualityPreset {
        far_clip: HIGH.far_clip * far_clip / RETAIL_ANCHOR.0,
        horizon_range: HIGH.horizon_range * horizon_start / RETAIL_ANCHOR.1,
        shadow_cascades,
        shadow_map_size: 1024,
        unit_shadow_radius,
        scenery_shadow_radius: HIGH.scenery_shadow_radius,
    }
}

impl GraphicsQuality {
    pub const fn preset(self) -> QualityPreset {
        match self {
            // Retail block 0: farClip 2000, horizonStart 600, shadowMode 0; the lowest
            // preset's `entityShadowFadeScale` 10 (Part A §10.1).
            Self::Low => scaled(2000.0, 600.0, 0, 10.0),
            // Retail block 2: farClip 5000, horizonStart 1000, shadowMode 1. Its
            // `entityShadowFadeScale` is not decoded per block: the default 50.
            Self::Medium => scaled(5000.0, 1000.0, 1, 50.0),
            Self::High => HIGH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_row_keeps_the_client_distances_and_splits() {
        assert_eq!(
            GraphicsQuality::default().preset(),
            QualityPreset {
                far_clip: 1000.0,
                horizon_range: 3108.0,
                shadow_cascades: 2,
                shadow_map_size: 2048,
                unit_shadow_radius: 50.0,
                scenery_shadow_radius: 50.0,
            }
        );
    }

    #[test]
    fn lower_rows_follow_retail_ratios() {
        let medium = GraphicsQuality::Medium.preset();
        assert!((medium.far_clip - 833.333).abs() < 1e-2);
        assert!((medium.horizon_range - 2220.0).abs() < 1e-2);
        assert_eq!((medium.shadow_cascades, medium.shadow_map_size), (1, 1024));
        let low = GraphicsQuality::Low.preset();
        assert!((low.far_clip - 333.333).abs() < 1e-2);
        assert!((low.horizon_range - 1332.0).abs() < 1e-2);
        assert_eq!(low.shadow_cascades, 0);
    }
}
