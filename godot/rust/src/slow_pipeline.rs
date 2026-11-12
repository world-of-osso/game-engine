//! Frames that compiled pipelines, the hitch class retail measures. Retail times each
//! pipeline-state creation, logs `Slow Pipeline Create: %lldms` from 125,000,000 input units
//! (125 ms, nanoseconds INFERRED) and accumulates a total (retail-optimisation Part C §6.3,
//! `0x14032aa00`). Godot 4.7 compiles pipelines in worker tasks and queues shader code to
//! the render thread, so one pipeline's time is not observable from the extension: a frame
//! whose draw-time or specialization compilation counters rose is charged its whole
//! interval instead (a per-frame proxy, INFERRED).

use godot::{
    classes::{RenderingServer, rendering_server::RenderingInfo},
    obj::Singleton,
};

/// Retail's slow-create threshold.
pub(crate) const SLOW_FRAME_MS: f64 = 125.0;

/// Godot's cumulative pipeline compilation counters, in `RenderingInfo` order: canvas,
/// mesh, surface, draw, specialization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Compilations([u64; 5]);

const DRAW: usize = 3;
const SPECIALIZATION: usize = 4;

impl Compilations {
    /// The counters now; reads render-side values without syncing with the render thread
    /// (servers/rendering/rendering_server_default.cpp `get_rendering_info`).
    pub(crate) fn read() -> Self {
        let server = RenderingServer::singleton();
        Self(
            [
                RenderingInfo::PIPELINE_COMPILATIONS_CANVAS,
                RenderingInfo::PIPELINE_COMPILATIONS_MESH,
                RenderingInfo::PIPELINE_COMPILATIONS_SURFACE,
                RenderingInfo::PIPELINE_COMPILATIONS_DRAW,
                RenderingInfo::PIPELINE_COMPILATIONS_SPECIALIZATION,
            ]
            .map(|info| server.get_rendering_info(info)),
        )
    }
}

/// A frame interval in which the render compiled pipelines it had to wait for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CompilingFrame {
    pub frame_ms: f64,
    pub draw: u64,
    pub specialization: u64,
}

#[derive(Default)]
pub(crate) struct SlowPipelines {
    previous: Option<Compilations>,
    /// Frame intervals with draw-time or specialization compilations, and their total time.
    pub compiling_frames: u64,
    pub compiling_ms: f64,
    /// Of those, the intervals of at least `SLOW_FRAME_MS`.
    pub slow_frames: u64,
}

impl SlowPipelines {
    /// Charges `frame_ms`, the interval that ended with `now`, when it compiled pipelines;
    /// the compiling frame when it did.
    pub(crate) fn observe(&mut self, now: Compilations, frame_ms: f64) -> Option<CompilingFrame> {
        let previous = self.previous.replace(now)?;
        let rose = |index: usize| now.0[index].saturating_sub(previous.0[index]);
        let frame = CompilingFrame {
            frame_ms,
            draw: rose(DRAW),
            specialization: rose(SPECIALIZATION),
        };
        if frame.draw == 0 && frame.specialization == 0 {
            return None;
        }
        self.compiling_frames += 1;
        self.compiling_ms += frame_ms;
        if frame_ms >= SLOW_FRAME_MS {
            self.slow_frames += 1;
        }
        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(draw: u64, specialization: u64) -> Compilations {
        Compilations([7, 3, 2, draw, specialization])
    }

    #[test]
    fn charges_only_intervals_whose_draw_or_specialization_counters_rose() {
        let mut slow = SlowPipelines::default();
        assert_eq!(slow.observe(counts(4, 1), 300.0), None, "no baseline yet");
        assert_eq!(slow.observe(counts(4, 1), 8.0), None);
        // Canvas, mesh and surface compilations do not stall a draw.
        assert_eq!(slow.observe(Compilations([9, 5, 6, 4, 1]), 9.0), None);
        assert_eq!(
            slow.observe(Compilations([9, 5, 6, 6, 1]), 140.0),
            Some(CompilingFrame {
                frame_ms: 140.0,
                draw: 2,
                specialization: 0,
            })
        );
        assert_eq!(
            slow.observe(Compilations([9, 5, 6, 6, 4]), 20.0),
            Some(CompilingFrame {
                frame_ms: 20.0,
                draw: 0,
                specialization: 3,
            })
        );
        assert_eq!(
            (slow.compiling_frames, slow.compiling_ms, slow.slow_frames),
            (2, 160.0, 1)
        );
    }
}
