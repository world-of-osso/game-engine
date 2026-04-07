use bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder};

/// Cascade shadow configuration tuned for WoW-scale terrain.
/// 4 cascades covering 0.1–500 units, with the first cascade
/// providing high detail out to 15 units from the camera.
pub fn default_cascade_shadow_config() -> CascadeShadowConfig {
    CascadeShadowConfigBuilder {
        num_cascades: 4,
        minimum_distance: 0.1,
        maximum_distance: 500.0,
        first_cascade_far_bound: 15.0,
        overlap_proportion: 0.2,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cascade_config_has_four_cascades() {
        let config = default_cascade_shadow_config();
        assert_eq!(config.bounds.len(), 4);
    }

    #[test]
    fn cascade_bounds_increase_monotonically() {
        let config = default_cascade_shadow_config();
        for window in config.bounds.windows(2) {
            assert!(
                window[1] > window[0],
                "bounds not increasing: {} -> {}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn first_cascade_covers_near_range() {
        let config = default_cascade_shadow_config();
        assert!(
            config.bounds[0] >= 15.0,
            "first cascade should cover at least 15 units, got {}",
            config.bounds[0]
        );
    }

    #[test]
    fn last_cascade_reaches_max_distance() {
        let config = default_cascade_shadow_config();
        let last = *config.bounds.last().unwrap();
        assert!(
            (last - 500.0).abs() < 1.0,
            "last cascade should reach ~500 units, got {last}"
        );
    }

    #[test]
    fn ssao_default_quality_is_high() {
        use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
        let ssao = ScreenSpaceAmbientOcclusion::default();
        assert_eq!(
            ssao.quality_level,
            ScreenSpaceAmbientOcclusionQualityLevel::High
        );
        assert!(ssao.constant_object_thickness > 0.0);
    }

    #[test]
    fn taa_default_resets_history() {
        use bevy::anti_alias::taa::TemporalAntiAliasing;
        let taa = TemporalAntiAliasing::default();
        assert!(taa.reset, "TAA should reset history on first frame");
    }

    #[test]
    fn depth_of_field_default_has_reasonable_focal_distance() {
        use bevy::post_process::dof::DepthOfField;
        let dof = DepthOfField::default();
        assert!(dof.focal_distance > 0.0);
        assert!(dof.aperture_f_stops > 0.0);
    }

    #[test]
    fn graphics_options_dof_disabled_by_default() {
        let opts = crate::client_options::GraphicsOptions::default();
        assert!(!opts.depth_of_field);
    }

    #[test]
    fn anti_alias_default_is_taa() {
        use crate::client_options::AntiAliasMode;
        assert_eq!(AntiAliasMode::default(), AntiAliasMode::Taa);
    }

    #[test]
    fn graphics_options_aa_defaults_to_taa() {
        let opts = crate::client_options::GraphicsOptions::default();
        assert_eq!(opts.anti_alias, crate::client_options::AntiAliasMode::Taa);
    }
}
