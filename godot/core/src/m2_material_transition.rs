//! Base-animation material transitions. Snapshot storage is allocated once per model.
use crate::{
    m2::AnimTrack,
    m2_material::{self, BatchBinding, MaterialSample, MaterialTime, MaterialTracks},
};

#[derive(Clone, Copy)]
pub enum MaterialSource<'a> {
    Sequence(MaterialTime),
    Snapshot(&'a MaterialSnapshot),
}

#[derive(Clone, Copy)]
pub struct MaterialTransition<'a> {
    pub source: MaterialSource<'a>,
    /// Incoming contribution, identical to the bone transition weight.
    pub weight: f32,
}

#[derive(Clone, Copy)]
struct TransformChannels {
    translation: [f32; 3],
    scale: [f32; 3],
    rotation: [f32; 4],
}

impl Default for TransformChannels {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            scale: [1.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Raw channels retained on an interrupted transition, before alpha multiplication
/// and UV composition. Global and step channels never consume or update this storage.
pub struct MaterialSnapshot {
    colors: Vec<([f32; 3], f32)>,
    weights: Vec<f32>,
    transforms: Vec<TransformChannels>,
}

impl MaterialSnapshot {
    pub fn new(tracks: &MaterialTracks) -> Self {
        Self {
            colors: vec![([1.0; 3], 1.0); tracks.colors.len()],
            weights: vec![1.0; tracks.weights.len()],
            transforms: vec![TransformChannels::default(); tracks.transforms.len()],
        }
    }

    /// Capture the current blend into existing storage. `None` means the previous
    /// outgoing pose was itself this snapshot; each channel is read before writing.
    pub fn capture(
        &mut self,
        tracks: &MaterialTracks,
        time: MaterialTime,
        outgoing: Option<MaterialTime>,
        weight: f32,
    ) {
        self.capture_colors(tracks, time, outgoing, weight);
        self.capture_weights(tracks, time, outgoing, weight);
        self.capture_transforms(tracks, time, outgoing, weight);
    }

    fn capture_colors(
        &mut self,
        tracks: &MaterialTracks,
        time: MaterialTime,
        outgoing: Option<MaterialTime>,
        weight: f32,
    ) {
        for index in 0..self.colors.len() {
            let sampled = self.sampler(tracks, time, outgoing, weight).color(index);
            let track = &tracks.colors[index];
            if eligible(&track.color) {
                self.colors[index].0 = sampled.0;
            }
            if eligible(&track.opacity) {
                self.colors[index].1 = sampled.1;
            }
        }
    }

    fn capture_weights(
        &mut self,
        tracks: &MaterialTracks,
        time: MaterialTime,
        outgoing: Option<MaterialTime>,
        weight: f32,
    ) {
        for index in 0..self.weights.len() {
            if eligible(&tracks.weights[index]) {
                self.weights[index] = self.sampler(tracks, time, outgoing, weight).weight(index);
            }
        }
    }

    fn capture_transforms(
        &mut self,
        tracks: &MaterialTracks,
        time: MaterialTime,
        outgoing: Option<MaterialTime>,
        weight: f32,
    ) {
        for index in 0..self.transforms.len() {
            let sampled = self
                .sampler(tracks, time, outgoing, weight)
                .transform(index);
            let track = &tracks.transforms[index];
            if eligible(&track.translation) {
                self.transforms[index].translation = sampled.translation;
            }
            if eligible(&track.scale) {
                self.transforms[index].scale = sampled.scale;
            }
            if eligible(&track.rotation) {
                self.transforms[index].rotation = sampled.rotation;
            }
        }
    }

    fn sampler<'a>(
        &'a self,
        tracks: &'a MaterialTracks,
        time: MaterialTime,
        outgoing: Option<MaterialTime>,
        weight: f32,
    ) -> Sampler<'a> {
        Sampler {
            tracks,
            time,
            transition: Some(MaterialTransition {
                source: outgoing.map_or(MaterialSource::Snapshot(self), MaterialSource::Sequence),
                weight,
            }),
        }
    }
}

fn eligible<T>(track: &AnimTrack<T>) -> bool {
    track.global_sequence < 0 && track.interpolation_type != 0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn lerp_vec3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| lerp(a[i], b[i], t))
}

fn slerp(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot = (0..4).map(|i| a[i] * b[i]).sum::<f32>();
    if dot < 0.0 {
        b = b.map(|v| -v);
        dot = -dot;
    }
    let (wa, wb) = if dot > 0.9995 {
        (1.0 - t, t)
    } else {
        let angle = dot.clamp(-1.0, 1.0).acos();
        (
            ((1.0 - t) * angle).sin() / angle.sin(),
            (t * angle).sin() / angle.sin(),
        )
    };
    let q = [0, 1, 2, 3].map(|i| a[i] * wa + b[i] * wb);
    let length = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    q.map(|v| v / length)
}

struct Sampler<'a> {
    tracks: &'a MaterialTracks,
    time: MaterialTime,
    transition: Option<MaterialTransition<'a>>,
}

impl Sampler<'_> {
    fn channel<T, V: Copy>(
        &self,
        track: &AnimTrack<T>,
        default: V,
        snapshot: impl Fn(&MaterialSnapshot) -> V,
        sample: impl Fn(&AnimTrack<T>, &MaterialTracks, MaterialTime) -> Option<V>,
        blend: impl Fn(V, V, f32) -> V,
    ) -> V {
        let current = sample(track, self.tracks, self.time).unwrap_or(default);
        let Some(transition) = self.transition.filter(|_| eligible(track)) else {
            return current;
        };
        let previous = match transition.source {
            MaterialSource::Sequence(time) => sample(track, self.tracks, time).unwrap_or(default),
            MaterialSource::Snapshot(values) => snapshot(values),
        };
        // Preserve exact endpoints, including repeated zero-elapsed interruptions.
        match transition.weight {
            0.0 => previous,
            1.0 => current,
            weight => blend(previous, current, weight),
        }
    }

    fn color(&self, index: usize) -> ([f32; 3], f32) {
        let color = &self.tracks.colors[index];
        (
            self.channel(
                &color.color,
                [1.0; 3],
                |s| s.colors[index].0,
                m2_material::sample_vec3,
                lerp_vec3,
            ),
            self.channel(
                &color.opacity,
                1.0,
                |s| s.colors[index].1,
                m2_material::sample_fixed16,
                lerp,
            ),
        )
    }

    fn weight(&self, index: usize) -> f32 {
        self.channel(
            &self.tracks.weights[index],
            1.0,
            |s| s.weights[index],
            m2_material::sample_fixed16,
            lerp,
        )
    }

    fn transform(&self, index: usize) -> TransformChannels {
        let transform = &self.tracks.transforms[index];
        TransformChannels {
            translation: self.channel(
                &transform.translation,
                [0.0; 3],
                |s| s.transforms[index].translation,
                m2_material::sample_vec3,
                lerp_vec3,
            ),
            scale: self.channel(
                &transform.scale,
                [1.0; 3],
                |s| s.transforms[index].scale,
                m2_material::sample_vec3,
                lerp_vec3,
            ),
            rotation: self.channel(
                &transform.rotation,
                [0.0, 0.0, 0.0, 1.0],
                |s| s.transforms[index].rotation,
                m2_material::sample_quat,
                slerp,
            ),
        }
    }
}

pub fn sample_material_transition(
    tracks: &MaterialTracks,
    binding: &BatchBinding,
    time: MaterialTime,
    transition: Option<MaterialTransition<'_>>,
) -> MaterialSample {
    let sampler = Sampler {
        tracks,
        time,
        transition,
    };
    let (mesh_color, alpha) = binding
        .color
        .map_or(([1.0; 3], 1.0), |index| sampler.color(index));
    let texture_weights = binding
        .texture_weights
        .map(|index| index.map_or(1.0, |index| sampler.weight(index)));
    MaterialSample {
        mesh_color,
        opacity: (alpha
            * if binding.apply_weight {
                texture_weights[0]
            } else {
                1.0
            })
        .clamp(0.0, 1.0),
        texture_weights,
        texture_matrices: binding.texture_transforms.map(|index| {
            index.map_or(m2_material::IDENTITY_MATRIX, |index| {
                let transform = sampler.transform(index);
                m2_material::texture_matrix_channels(
                    transform.rotation,
                    transform.scale,
                    transform.translation,
                )
            })
        }),
    }
}
