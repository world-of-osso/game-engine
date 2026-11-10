//! Spell ground batches draped off the main thread. The main thread copies everything a
//! kit's columns read (collision floors, terrain heights, render surfaces) into
//! [`DrapeInputs`]; a worker runs the same drape over that copy, so the draped arrays equal
//! a main-thread drape of the same frame.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use game_engine_core::ground_drape::GroundDrape;
use glam::{Affine3A, Vec3};

use crate::drape_batch::{BatchArrays, Drape};
use crate::ground::FootprintFloors;
use crate::terrain::objects::{SurfaceScratch, SurfaceSnapshot};
use crate::terrain::streaming::TerrainHeights;

/// What a kit's drape reads from the world, copied on the main thread.
pub(crate) struct DrapeInputs {
    pub floors: FootprintFloors,
    pub heights: TerrainHeights,
    pub surfaces: SurfaceSnapshot,
    /// The map is one global WMO, whose terrain is not collision ground.
    pub global_wmo: bool,
    /// The kit's position; its height bounds each column's collision floor search.
    pub anchor: Vec3,
    pub drape: GroundDrape,
    pub window: f32,
}

impl DrapeInputs {
    /// The collision floor under (x, z), as the main-thread drape computes it.
    pub fn floor_at(&self, x: f32, z: f32) -> Option<f32> {
        let terrain = self.heights.height_at(x, z);
        let ground_terrain = if self.global_wmo { None } else { terrain };
        let collision =
            self.floors
                .collision_floor(Vec3::new(x, self.anchor.y, z), ground_terrain, None);
        terrain.into_iter().chain(collision).max_by(f32::total_cmp)
    }

    /// The kit's own floor, which places its batches' flat plane.
    pub fn ground_y(&self) -> f32 {
        self.floor_at(self.anchor.x, self.anchor.z)
            .unwrap_or(self.anchor.y)
    }

    fn drape(&self, arrays: &BatchArrays, placement: Affine3A) -> Drape {
        let mut scratch = SurfaceScratch::default();
        let mut surface = |x: f32, z: f32, ceiling: f32, floor: f32| {
            let terrain = self.heights.height_at(x, z);
            self.surfaces
                .visible_surface(&mut scratch, terrain, x, z, floor, ceiling - floor)
        };
        arrays.drape(
            &self.drape,
            placement,
            self.ground_y(),
            self.window,
            &|x, z| self.floor_at(x, z),
            &mut surface,
        )
    }
}

/// One batch to drape: `kit` and `batch` identify it to the caller.
pub(crate) struct DrapeJob {
    pub kit: u64,
    pub batch: usize,
    pub inputs: Arc<DrapeInputs>,
    pub arrays: Arc<BatchArrays>,
    pub placement: Affine3A,
}

pub(crate) struct DrapeDone {
    pub kit: u64,
    pub batch: usize,
    pub result: Drape,
}

/// Threads draping [`DrapeJob`]s in submission order.
pub(crate) struct DrapeWorkers {
    jobs: Sender<DrapeJob>,
    done: Receiver<DrapeDone>,
}

impl DrapeWorkers {
    pub fn new(threads: usize) -> Self {
        let (jobs, queue) = mpsc::channel::<DrapeJob>();
        let queue = Arc::new(Mutex::new(queue));
        let (finished, done) = mpsc::channel();
        for index in 0..threads.max(1) {
            let queue = Arc::clone(&queue);
            let finished = finished.clone();
            // Dropping the workers closes the queue; extension shutdown joins the threads.
            let handle = std::thread::Builder::new()
                .name(format!("ground-drape-{index}"))
                .spawn(move || {
                    loop {
                        let job = match queue.lock().expect("drape queue").recv() {
                            Ok(job) => job,
                            Err(_) => return,
                        };
                        let result = job.inputs.drape(&job.arrays, job.placement);
                        let done = DrapeDone {
                            kit: job.kit,
                            batch: job.batch,
                            result,
                        };
                        if finished.send(done).is_err() {
                            return;
                        }
                    }
                })
                .expect("spawn ground drape worker");
            game_engine_core::worker_shutdown::track(handle);
        }
        Self { jobs, done }
    }

    pub fn submit(&self, job: DrapeJob) {
        self.jobs.send(job).expect("ground drape workers alive");
    }

    /// Batches finished since the last call.
    pub fn finished(&self) -> Vec<DrapeDone> {
        self.done.try_iter().collect()
    }
}
