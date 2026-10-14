//! M2 ribbon sampling and world-space strip geometry.
//! WebWowViewerCpp animationManager::calcRibbonEmitters / CRibbonEmitter.
use crate::{
    asset::m2_format::{m2_anim, m2_ribbon::M2RibbonEmitter},
    m2::AnimTime,
};
use glam::Vec3;
use std::collections::VecDeque;

/// Ribbon material UV lookup; kept at the core boundary for real-byte regressions.
pub fn texture_transform_index(
    model: &crate::m2::Model,
    source: &M2RibbonEmitter,
    _material_slot: usize,
) -> Result<Option<usize>, String> {
    if source.texture_transform_lookup < 0 {
        return Ok(None);
    }
    let lookup = usize::from(source.texture_transform_lookup as u8);
    let index = *model
        .uv_animation_lookup
        .get(lookup)
        .ok_or_else(|| format!("Ribbon UV lookup{lookup} missing"))?;
    if index >= 0 {
        return Err(format!(
            "Ribbon animated UV transform{index} remains unsupported"
        ));
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug)]
pub struct RibbonValues {
    pub color: [f32; 4],
    pub above: f32,
    pub below: f32,
    pub slot: u16,
    pub enabled: bool,
}

/// Unauthored tracks use calcRibbonEmitters' initialized values, never replacement assets.
pub fn sample(emitter: &M2RibbonEmitter, time: &AnimTime<'_>) -> RibbonValues {
    let (sequence, ms) = time.track_time(&emitter.color);
    let rgb = m2_anim::evaluate_vec3_track(&emitter.color, sequence, ms).unwrap_or([1.0; 3]);
    let (sequence, ms) = time.track_time(&emitter.alpha);
    let alpha = m2_anim::evaluate_i16_track(&emitter.alpha, sequence, ms)
        .map_or(1.0, |value| (f32::from(value) / 32767.0).clamp(0.0, 1.0));
    let scalar = |track: &m2_anim::AnimTrack<f32>| {
        let (sequence, ms) = time.track_time(track);
        m2_anim::evaluate_f32_track(track, sequence, ms).unwrap_or(1.0)
    };
    let (sequence, ms) = time.track_time(&emitter.texture_slot);
    let slot = m2_anim::evaluate_u16_track(&emitter.texture_slot, sequence, ms).unwrap_or(0);
    let (sequence, ms) = time.track_time(&emitter.visibility);
    let enabled = m2_anim::evaluate_u8_track(&emitter.visibility, sequence, ms).unwrap_or(1) != 0;
    RibbonValues {
        color: [rgb[0], rgb[1], rgb[2], alpha],
        above: scalar(&emitter.height_above),
        below: scalar(&emitter.height_below),
        slot,
        enabled,
    }
}

/// Source SetPos columns2/1, already transformed into engine world coordinates.
#[derive(Clone, Copy, Debug)]
pub struct RibbonFrame {
    pub position: Vec3,
    pub direction: Vec3,
    pub vertical: Vec3,
}

#[derive(Clone, Copy)]
struct Edge {
    born: f64,
    below: Vec3,
    above: Vec3,
    color: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct RibbonVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub struct RibbonTrail {
    interval: f64,
    lifetime: f64,
    gravity: f32,
    rows: u16,
    cols: u16,
    clock: f64,
    next_edge: f64,
    slot: u16,
    updated: bool,
    previous: Option<RibbonFrame>,
    head: Option<Edge>,
    edges: VecDeque<Edge>,
}

impl RibbonTrail {
    pub fn new(emitter: &M2RibbonEmitter) -> Result<Self, String> {
        if !emitter.edges_per_second.is_finite()
            || emitter.edges_per_second <= 0.0
            || !emitter.edge_lifetime.is_finite()
            || emitter.edge_lifetime <= 0.0
        {
            return Err(format!("Ribbon{} has invalid density/lifetime", emitter.id));
        }
        if emitter.texture_rows == 0 || emitter.texture_cols == 0 {
            return Err(format!("Ribbon{} has empty texture tiles", emitter.id));
        }
        // CRibbonEmitter::Initialize rounds density and enforces its minimum lifetime.
        let interval = 1.0 / f64::from(emitter.edges_per_second.ceil());
        Ok(Self {
            interval,
            lifetime: f64::from(emitter.edge_lifetime.max(0.25)),
            gravity: emitter.gravity,
            rows: emitter.texture_rows,
            cols: emitter.texture_cols,
            clock: 0.0,
            next_edge: interval,
            slot: 0,
            updated: false,
            previous: None,
            head: None,
            edges: VecDeque::new(),
        })
    }

    pub fn advance(
        &mut self,
        seconds: f32,
        frame: RibbonFrame,
        values: RibbonValues,
        emitting: bool,
    ) -> Result<(), String> {
        if u32::from(values.slot) >= u32::from(self.rows) * u32::from(self.cols) {
            return Err(format!(
                "Ribbon texture slot{} exceeds {}x{}",
                values.slot, self.rows, self.cols
            ));
        }
        let start = self.clock;
        self.update_clock(seconds);
        self.slot = values.slot;
        self.expire_edges();
        self.head = None;
        if !emitting || !values.enabled {
            self.previous = None;
            self.next_edge = self.clock;
            return Ok(());
        }
        self.emit_edges(start, frame, values);
        self.head = Some(interpolate_edge(frame, frame, values, 1.0, self.clock));
        self.previous = Some(frame);
        Ok(())
    }

    fn update_clock(&mut self, seconds: f32) {
        // The first source Update warms one edge; later long frames clamp, not skip.
        const FIRST_UPDATE_EPSILON: f64 = 0.0001;
        let delta = if self.updated {
            f64::from(seconds).max(0.0)
        } else {
            self.interval + FIRST_UPDATE_EPSILON
        };
        self.updated = true;
        self.clock += delta.min(self.lifetime);
    }

    fn expire_edges(&mut self) {
        while self
            .edges
            .front()
            .is_some_and(|edge| self.clock - edge.born > self.lifetime)
        {
            self.edges.pop_front();
        }
    }

    fn emit_edges(&mut self, start: f64, frame: RibbonFrame, values: RibbonValues) {
        // SetPos initializes the previous frame to the first authored position.
        let previous = self.previous.unwrap_or(frame);
        self.skip_expired_samples();
        while self.next_edge <= self.clock {
            let fraction = if self.clock > start {
                ((self.next_edge - start) / (self.clock - start)).clamp(0.0, 1.0) as f32
            } else {
                1.0
            };
            self.edges.push_back(interpolate_edge(
                previous,
                frame,
                values,
                fraction,
                self.next_edge,
            ));
            self.next_edge += self.interval;
        }
    }

    fn skip_expired_samples(&mut self) {
        let oldest = self.clock - self.lifetime;
        if self.next_edge < oldest {
            let skipped = ((oldest - self.next_edge) / self.interval).ceil();
            self.next_edge += skipped * self.interval;
        }
    }

    /// Explicit triangle vertices; previous edges never move with their owner.
    pub fn triangles(&self, fade: f32) -> Vec<RibbonVertex> {
        let edges: Vec<_> = self.edges.iter().chain(self.head.iter()).collect();
        edges
            .windows(2)
            .flat_map(|pair| {
                let a = self.vertices(pair[0], fade);
                let b = self.vertices(pair[1], fade);
                [a[0], a[1], b[0], b[0], a[1], b[1]]
            })
            .collect()
    }

    fn vertices(&self, edge: &Edge, fade: f32) -> [RibbonVertex; 2] {
        let age = (self.clock - edge.born).max(0.0) as f32;
        // CRibbonEmitter::Update advances world Z by g*(new_age²-old_age²).
        let displacement = Vec3::Y * self.gravity * age * age;
        let column = self.slot % self.cols;
        let row = self.slot / self.cols;
        let u = (f32::from(column) + age / self.lifetime as f32) / f32::from(self.cols);
        let color = [
            edge.color[0],
            edge.color[1],
            edge.color[2],
            edge.color[3] * fade,
        ];
        [
            RibbonVertex {
                position: (edge.below + displacement).to_array(),
                uv: [u, f32::from(row) / f32::from(self.rows)],
                color,
            },
            RibbonVertex {
                position: (edge.above + displacement).to_array(),
                uv: [u, f32::from(row + 1) / f32::from(self.rows)],
                color,
            },
        ]
    }
}

fn interpolate_edge(
    previous: RibbonFrame,
    current: RibbonFrame,
    values: RibbonValues,
    fraction: f32,
    born: f64,
) -> Edge {
    let distance = previous.position.distance(current.position);
    let bend = (previous.direction - current.direction) * distance * fraction * (1.0 - fraction);
    let interpolate = |height: f32| {
        (previous.position + previous.vertical * height)
            .lerp(current.position + current.vertical * height, fraction)
            + bend
    };
    Edge {
        born,
        below: interpolate(-values.below),
        above: interpolate(values.above),
        color: values.color,
    }
}
