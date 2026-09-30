//! M2 point lights as retail evaluates them: WebWowViewerCpp
//! `AnimationManager::calcLights` (`managers/animationManager.cpp:1119-1253`) and
//! `M2Object::collectLights` (`objects/m2/m2Object.cpp:1294-1330`).
use crate::asset::m2_format::{
    m2_anim::{self, AnimTrack},
    m2_light::{M2_LIGHT_TYPE_POINT, M2Light},
};

/// Retail replaces authored attenuation unless the model sets global flag 0x8000
/// (`animationManager.cpp:1238-1241`); build 12340 ignores it outright and uses fixed
/// falloff constants (solarityclient `lighting/m2_light.rs:14`).
const AUTHORED_ATTENUATION: u32 = 0x8000;
const RETAIL_ATTENUATION_START: f32 = 1.6666;
const RETAIL_ATTENUATION_END: f32 = 5.266_660_2;

/// One point light at a model time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    /// Diffuse colour x diffuse intensity, in authored (gamma) space.
    pub color: [f32; 3],
    /// Full brightness within `start` yards, none beyond `end`, before model scale.
    pub attenuation_start: f32,
    pub attenuation_end: f32,
    pub visible: bool,
}

impl PointLight {
    /// Attenuation start as a fraction of the end: where the linear ramp begins
    /// (WebWowViewerCpp `pointLight.frag.slang:51`). A zero-reach light has none.
    pub fn start_fraction(&self) -> f32 {
        if self.attenuation_end > 0.0 {
            (self.attenuation_start / self.attenuation_end).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// The playing sequence and clocks a light's tracks read.
#[derive(Clone, Copy, Debug)]
pub struct LightTime<'a> {
    pub sequence: usize,
    pub time_ms: u32,
    /// Elapsed time for global-sequence tracks.
    pub global_ms: u64,
    pub global_sequences: &'a [u32],
}

impl LightTime<'_> {
    fn sample<T: Copy>(
        &self,
        track: &AnimTrack<T>,
        evaluate: impl Fn(&AnimTrack<T>, usize, u32) -> Option<T>,
    ) -> Option<T> {
        let Ok(global) = usize::try_from(track.global_sequence) else {
            return evaluate(track, self.sequence, self.time_ms);
        };
        let duration = u64::from(*self.global_sequences.get(global)?);
        let time = if duration == 0 {
            0
        } else {
            (self.global_ms % duration) as u32
        };
        evaluate(track, 0, time)
    }
}

/// Light `light` of a model with global `model_flags` at `time`; `None` for a
/// directional light. Unauthored tracks read 1 (`animateTrackWithBlend` defaults).
pub fn point_light(light: &M2Light, model_flags: u32, time: &LightTime) -> Option<PointLight> {
    if light.light_type != M2_LIGHT_TYPE_POINT {
        return None;
    }
    let color = time
        .sample(&light.diffuse_color, m2_anim::evaluate_vec3_track)
        .unwrap_or([1.0; 3]);
    let intensity = time
        .sample(&light.diffuse_intensity, m2_anim::evaluate_f32_track)
        .unwrap_or(1.0);
    let (start, end) = if model_flags & AUTHORED_ATTENUATION == 0 {
        (RETAIL_ATTENUATION_START, RETAIL_ATTENUATION_END)
    } else {
        let start = time
            .sample(&light.attenuation_start, m2_anim::evaluate_f32_track)
            .unwrap_or(1.0);
        let end = time
            .sample(&light.attenuation_end, m2_anim::evaluate_f32_track)
            .unwrap_or(1.0);
        (start, if end < start { start + 1.0 } else { end })
    };
    let visible = time
        .sample(&light.visibility, m2_anim::evaluate_u8_track)
        .unwrap_or(1)
        != 0;
    Some(PointLight {
        color: color.map(|channel| channel * intensity),
        attenuation_start: start,
        attenuation_end: end,
        visible,
    })
}

/// Whether any track of `light` changes over time.
pub fn light_animates(light: &M2Light) -> bool {
    !(crate::m2::track_is_constant(&light.diffuse_color)
        && crate::m2::track_is_constant(&light.diffuse_intensity)
        && crate::m2::track_is_constant(&light.attenuation_start)
        && crate::m2::track_is_constant(&light.attenuation_end)
        && crate::m2::track_is_constant(&light.visibility))
}
