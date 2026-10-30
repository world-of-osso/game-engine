//! Opt-in bounded per-frame player movement diagnostics.

use std::collections::VecDeque;

use serde::Serialize;

const TRACE_CAPACITY: usize = 2048;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Support {
    pub height: f32,
    pub source: &'static str,
    pub wmo_placement: Option<u32>,
    pub wmo_group: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Frame {
    pub sequence: u64,
    pub timestamp_ms: u128,
    pub before: [f32; 3],
    pub after: [f32; 3],
    pub input_direction: [f32; 3],
    pub input_speed: f32,
    pub input_vertical: f32,
    pub input_jump: bool,
    pub input_yaw: f32,
    pub delta_seconds: f32,
    pub ground_rejection: Option<&'static str>,
    pub support_before: Option<Support>,
    pub support_after: Option<Support>,
    pub vertical_velocity: f32,
    pub grounded: bool,
    pub falling: bool,
}

#[derive(Default)]
pub(crate) struct MovementTrace {
    enabled: bool,
    sequence: u64,
    frames: VecDeque<Frame>,
}

impl MovementTrace {
    pub fn start(&mut self) {
        self.enabled = true;
        self.sequence = 0;
        self.frames.clear();
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn push(&mut self, mut frame: Frame) {
        frame.sequence = self.sequence;
        self.sequence += 1;
        if self.frames.len() == TRACE_CAPACITY {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    pub fn json(&self) -> Result<String, String> {
        serde_json::to_string(&self.frames).map_err(|error| format!("movement trace: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_trace_keeps_the_latest_frames_and_resets() {
        let mut trace = MovementTrace::default();
        assert!(!trace.enabled());
        trace.start();
        for index in 0..TRACE_CAPACITY + 1 {
            trace.push(Frame {
                sequence: 0,
                timestamp_ms: index as u128,
                before: [1.0, 2.0, 3.0],
                after: [1.0, 1.0, 3.0],
                input_direction: [1.0, 0.0, 0.0],
                input_speed: 7.0,
                input_vertical: 0.0,
                input_jump: false,
                input_yaw: 1.5,
                delta_seconds: 0.016,
                ground_rejection: Some("wall_clamp"),
                support_before: Some(Support {
                    height: 2.0,
                    source: "wmo",
                    wmo_placement: Some(7),
                    wmo_group: Some(8),
                }),
                support_after: None,
                vertical_velocity: -3.0,
                grounded: false,
                falling: true,
            });
        }
        let frames: serde_json::Value = serde_json::from_str(&trace.json().unwrap()).unwrap();
        assert_eq!(frames.as_array().unwrap().len(), TRACE_CAPACITY);
        assert_eq!(frames[0]["sequence"], 1);
        assert_eq!(frames[0]["support_before"]["wmo_group"], 8);
        assert_eq!(frames[0]["falling"], true);
        assert_eq!(
            frames[0]["input_direction"],
            serde_json::json!([1.0, 0.0, 0.0])
        );
        assert_eq!(frames[0]["ground_rejection"], "wall_clamp");
        trace.start();
        assert_eq!(trace.json().unwrap(), "[]");
    }
}
