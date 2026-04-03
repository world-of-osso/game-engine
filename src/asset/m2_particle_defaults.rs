use super::{EmitterHeaderCore, M2ParticleEmitter};

macro_rules! zeroed_emitter {
    () => {
        M2ParticleEmitter {
            flags: 0,
            position: [0.0; 3],
            bone_index: 0,
            texture_index: 0,
            texture_fdid: None,
            blend_type: 0,
            emitter_type: 0,
            particle_type: 0,
            head_or_tail: 0,
            tile_rows: 0,
            tile_cols: 0,
            emission_speed: 0.0,
            speed_variation: 0.0,
            vertical_range: 0.0,
            horizontal_range: 0.0,
            gravity: 0.0,
            gravity_vector: [0.0; 3],
            lifespan: 0.0,
            lifespan_variation: 0.0,
            emission_rate: 0.0,
            emission_rate_variation: 0.0,
            area_length: 0.0,
            area_width: 0.0,
            z_source: 0.0,
            tail_length: 1.0,
            drag: 0.0,
            scale_variation: 0.0,
            scale_variation_y: 0.0,
            base_spin: 0.0,
            base_spin_variation: 0.0,
            spin: 0.0,
            spin_variation: 0.0,
            wind_vector: [0.0; 3],
            wind_time: 0.0,
            colors: [[0.0; 3]; 3],
            color_keys: Vec::new(),
            opacity: [0.0; 3],
            opacity_keys: Vec::new(),
            scales: [[0.0; 2]; 3],
            scale_keys: Vec::new(),
            twinkle_speed: 0.0,
            twinkle_percent: 0.0,
            twinkle_scale_min: 0.0,
            twinkle_scale_max: 0.0,
            head_cell_track: [0; 3],
            tail_cell_track: [0; 3],
            burst_multiplier: 0.0,
            mid_point: 0.0,
        }
    };
}

struct EmitterTrackDefaults {
    emission_speed: f32,
    speed_variation: f32,
    vertical_range: f32,
    horizontal_range: f32,
    gravity: f32,
    gravity_vector: [f32; 3],
    lifespan: f32,
    lifespan_variation: f32,
    emission_rate: f32,
    emission_rate_variation: f32,
    area_length: f32,
    area_width: f32,
    z_source: f32,
    tail_length: f32,
    drag: f32,
    scale_variation: f32,
    scale_variation_y: f32,
    base_spin: f32,
    base_spin_variation: f32,
    spin: f32,
    spin_variation: f32,
    wind_vector: [f32; 3],
    wind_time: f32,
}

impl Default for EmitterTrackDefaults {
    fn default() -> Self {
        Self {
            emission_speed: 0.0,
            speed_variation: 0.0,
            vertical_range: 0.0,
            horizontal_range: 0.0,
            gravity: 0.0,
            gravity_vector: [0.0; 3],
            lifespan: 0.0,
            lifespan_variation: 0.0,
            emission_rate: 0.0,
            emission_rate_variation: 0.0,
            area_length: 0.0,
            area_width: 0.0,
            z_source: 0.0,
            tail_length: 1.0,
            drag: 0.0,
            scale_variation: 0.0,
            scale_variation_y: 0.0,
            base_spin: 0.0,
            base_spin_variation: 0.0,
            spin: 0.0,
            spin_variation: 0.0,
            wind_vector: [0.0; 3],
            wind_time: 0.0,
        }
    }
}

struct EmitterVisualDefaults {
    colors: [[f32; 3]; 3],
    color_keys: Vec<(f32, [f32; 3])>,
    opacity: [f32; 3],
    opacity_keys: Vec<(f32, f32)>,
    scales: [[f32; 2]; 3],
    scale_keys: Vec<(f32, [f32; 2])>,
    twinkle_speed: f32,
    twinkle_percent: f32,
    twinkle_scale_min: f32,
    twinkle_scale_max: f32,
    head_cell_track: [u16; 3],
    tail_cell_track: [u16; 3],
    burst_multiplier: f32,
    mid_point: f32,
}

impl Default for EmitterVisualDefaults {
    fn default() -> Self {
        Self {
            colors: [[0.0; 3]; 3],
            color_keys: Vec::new(),
            opacity: [1.0; 3],
            opacity_keys: Vec::new(),
            scales: [[1.0; 2]; 3],
            scale_keys: Vec::new(),
            twinkle_speed: 0.0,
            twinkle_percent: 0.0,
            twinkle_scale_min: 1.0,
            twinkle_scale_max: 1.0,
            head_cell_track: [0; 3],
            tail_cell_track: [0; 3],
            burst_multiplier: 1.0,
            mid_point: 0.5,
        }
    }
}

pub(super) fn build_emitter_header(header: EmitterHeaderCore) -> M2ParticleEmitter {
    emitter_from_parts(
        header,
        EmitterTrackDefaults::default(),
        EmitterVisualDefaults::default(),
    )
}

pub(super) fn default_emitter_state() -> M2ParticleEmitter {
    let mut emitter = zeroed_emitter_state();
    apply_visual_defaults(&mut emitter, EmitterVisualDefaults::default());
    emitter
}

fn emitter_from_parts(
    header: EmitterHeaderCore,
    tracks: EmitterTrackDefaults,
    visuals: EmitterVisualDefaults,
) -> M2ParticleEmitter {
    let mut emitter = zeroed_emitter_state();
    apply_header_core(&mut emitter, header);
    apply_track_defaults(&mut emitter, tracks);
    apply_visual_defaults(&mut emitter, visuals);
    emitter
}

fn zeroed_emitter_state() -> M2ParticleEmitter {
    let mut emitter = emitter_shell();
    apply_header_core(&mut emitter, EmitterHeaderCore::default());
    apply_track_defaults(&mut emitter, zeroed_track_defaults());
    apply_visual_defaults(&mut emitter, zeroed_visual_defaults());
    emitter
}

fn emitter_shell() -> M2ParticleEmitter {
    zeroed_emitter!()
}

fn apply_header_core(emitter: &mut M2ParticleEmitter, header: EmitterHeaderCore) {
    emitter.flags = header.flags;
    emitter.position = header.position;
    emitter.bone_index = header.bone_index;
    emitter.texture_index = header.texture_index;
    emitter.blend_type = header.blend_type;
    emitter.emitter_type = header.emitter_type;
    emitter.particle_type = header.particle_type;
    emitter.head_or_tail = header.head_or_tail;
    emitter.tile_rows = header.tile_rows;
    emitter.tile_cols = header.tile_cols;
}

fn apply_track_defaults(emitter: &mut M2ParticleEmitter, tracks: EmitterTrackDefaults) {
    emitter.emission_speed = tracks.emission_speed;
    emitter.speed_variation = tracks.speed_variation;
    emitter.vertical_range = tracks.vertical_range;
    emitter.horizontal_range = tracks.horizontal_range;
    emitter.gravity = tracks.gravity;
    emitter.gravity_vector = tracks.gravity_vector;
    emitter.lifespan = tracks.lifespan;
    emitter.lifespan_variation = tracks.lifespan_variation;
    emitter.emission_rate = tracks.emission_rate;
    emitter.emission_rate_variation = tracks.emission_rate_variation;
    emitter.area_length = tracks.area_length;
    emitter.area_width = tracks.area_width;
    emitter.z_source = tracks.z_source;
    emitter.tail_length = tracks.tail_length;
    emitter.drag = tracks.drag;
    emitter.scale_variation = tracks.scale_variation;
    emitter.scale_variation_y = tracks.scale_variation_y;
    emitter.base_spin = tracks.base_spin;
    emitter.base_spin_variation = tracks.base_spin_variation;
    emitter.spin = tracks.spin;
    emitter.spin_variation = tracks.spin_variation;
    emitter.wind_vector = tracks.wind_vector;
    emitter.wind_time = tracks.wind_time;
}

fn apply_visual_defaults(emitter: &mut M2ParticleEmitter, visuals: EmitterVisualDefaults) {
    emitter.colors = visuals.colors;
    emitter.color_keys = visuals.color_keys;
    emitter.opacity = visuals.opacity;
    emitter.opacity_keys = visuals.opacity_keys;
    emitter.scales = visuals.scales;
    emitter.scale_keys = visuals.scale_keys;
    emitter.twinkle_speed = visuals.twinkle_speed;
    emitter.twinkle_percent = visuals.twinkle_percent;
    emitter.twinkle_scale_min = visuals.twinkle_scale_min;
    emitter.twinkle_scale_max = visuals.twinkle_scale_max;
    emitter.head_cell_track = visuals.head_cell_track;
    emitter.tail_cell_track = visuals.tail_cell_track;
    emitter.burst_multiplier = visuals.burst_multiplier;
    emitter.mid_point = visuals.mid_point;
}

fn zeroed_track_defaults() -> EmitterTrackDefaults {
    EmitterTrackDefaults::default()
}

fn zeroed_visual_defaults() -> EmitterVisualDefaults {
    EmitterVisualDefaults {
        opacity: [0.0; 3],
        scales: [[0.0; 2]; 3],
        twinkle_scale_min: 0.0,
        twinkle_scale_max: 0.0,
        burst_multiplier: 0.0,
        mid_point: 0.0,
        ..EmitterVisualDefaults::default()
    }
}
