use std::time::{Duration, Instant};

pub(crate) struct TileSpawnTimings {
    tile: (u32, u32),
    phase: &'static str,
    started: Option<Instant>,
    last_stage: Option<Instant>,
}

impl TileSpawnTimings {
    pub(crate) fn start(tile: (u32, u32), phase: &'static str) -> Self {
        let enabled = std::env::var_os("WOO_PERF_MOVEMENT").is_some();
        let started = enabled.then(Instant::now);
        Self {
            tile,
            phase,
            started,
            last_stage: started,
        }
    }

    pub(crate) fn record_stage(&mut self, stage: &'static str) {
        let Some(previous) = self.last_stage else {
            return;
        };
        let now = Instant::now();
        eprintln!(
            "{}",
            format_tile_spawn_stage(self.tile, self.phase, stage, now.duration_since(previous))
        );
        self.last_stage = Some(now);
    }

    pub(crate) fn finish(self) {
        if let Some(started) = self.started {
            eprintln!(
                "{}",
                format_tile_spawn_stage(self.tile, self.phase, "total", started.elapsed())
            );
        }
    }
}

fn format_tile_spawn_stage(
    tile: (u32, u32),
    phase: &str,
    stage: &str,
    elapsed: Duration,
) -> String {
    format!(
        "tile_spawn_perf tile_y={} tile_x={} phase={phase} stage={stage} elapsed_us={}",
        tile.0,
        tile.1,
        elapsed.as_micros()
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::format_tile_spawn_stage;

    #[test]
    fn stage_log_identifies_tile_phase_and_microseconds() {
        assert_eq!(
            format_tile_spawn_stage((31, 48), "objects", "doodads", Duration::from_micros(1250)),
            "tile_spawn_perf tile_y=31 tile_x=48 phase=objects stage=doodads elapsed_us=1250"
        );
    }
}
