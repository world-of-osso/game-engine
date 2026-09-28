//! Authored light catalogs loaded on the terrain asset worker.

use std::{fs, path::Path};

use game_engine_core::{
    light_lookup_data::{
        LightEntry, LightParamsSlot, ZoneLight, light_params_blend, parse_zone_lights,
    },
    lighting_assets::{
        LightKeyframes, linear_to_authored_rgb, parse_light_csv, parse_light_data_csv,
    },
    retail_light_data::{RetailLightColors, RetailLightData, scene_light},
    sky_lightdata_data::{RetailFog, SkyColorSet, retail_fog, sample_light_blend},
};

pub(crate) struct LightingCatalog {
    lights: Vec<LightEntry>,
    zone_lights: Vec<ZoneLight>,
    keyframes: LightKeyframes,
}

pub(crate) struct LightingSample {
    pub retail: RetailLightData,
    pub fog: RetailFog,
    pub sky: SkyColorSet<[f32; 3]>,
}

impl LightingCatalog {
    pub fn read(data_root: &Path) -> Result<Self, String> {
        let lights_path = data_root.join("Light.csv");
        let keyframes_path = data_root.join("LightData.csv");
        let lights = parse_light_csv(&read_text(&lights_path)?)
            .map_err(|error| format!("{}: {error}", lights_path.display()))?;
        let keyframes = parse_light_data_csv(&read_text(&keyframes_path)?)
            .map_err(|error| format!("{}: {error}", keyframes_path.display()))?;
        let zone_lights = parse_zone_lights(
            &read_text(&data_root.join("ZoneLight.csv"))?,
            &read_text(&data_root.join("ZoneLightPoint.csv"))?,
        )
        .map_err(|error| format!("{}: {error}", data_root.display()))?;
        Ok(Self {
            lights,
            zone_lights,
            keyframes,
        })
    }

    pub fn sample(
        &self,
        map_id: u32,
        wow_position: [f32; 3],
        minutes: f32,
    ) -> Result<LightingSample, String> {
        let blend = light_params_blend(
            &self.lights,
            &self.zone_lights,
            map_id,
            wow_position,
            LightParamsSlot::Clear,
        );
        for light in &blend {
            if self
                .keyframes
                .get(&light.light_params_id)
                .is_none_or(Vec::is_empty)
            {
                return Err(format!(
                    "LightParams {} has no authored keyframes",
                    light.light_params_id
                ));
            }
        }
        let weights: Vec<_> = blend
            .iter()
            .map(|light| (light.light_params_id, light.weight))
            .collect();
        let sky = sample_light_blend(&self.keyframes, &weights, minutes, lerp_rgb)
            .ok_or_else(|| format!("No authored lighting for map {map_id} at {wow_position:?}"))?;
        let retail = scene_light(&retail_colors(&sky), minutes);
        let fog = retail_fog(&sky);
        Ok(LightingSample { retail, fog, sky })
    }
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))
}

fn retail_colors(sky: &SkyColorSet<[f32; 3]>) -> RetailLightColors {
    RetailLightColors {
        ambient: linear_to_authored_rgb(sky.ambient_color),
        horizon_ambient: linear_to_authored_rgb(sky.horizon_ambient_color),
        ground_ambient: linear_to_authored_rgb(sky.ground_ambient_color),
        direct: linear_to_authored_rgb(sky.direct_color),
        fog_color: linear_to_authored_rgb(sky.fog_color),
        fog_start: sky.fog_start,
        fog_end: sky.fog_end,
    }
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|index| a[index] + (b[index] - a[index]) * t)
}
