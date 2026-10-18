//! Non-mutating layer census emitted while existing golden comparisons run.
use std::collections::BTreeSet;
use ui_toolkit::registry::FrameRegistry;

pub(super) fn record(registry: &FrameRegistry, id: u64) {
    let frame = registry.get(id).expect("golden root");
    if frame.parent_id.is_some() {
        return;
    }
    let layers: BTreeSet<_> = registry
        .frames_iter()
        .map(|frame| {
            (
                frame.strata,
                frame.frame_level,
                frame.raise_order,
                frame.draw_layer,
            )
        })
        .collect();
    eprintln!(
        "GOLDEN_LAYER_CENSUS scene={:?} skin={:?} frames={} layers={}",
        frame.name,
        ui_toolkit::atlas::thread_skin(),
        registry.frame_count(),
        layers.len()
    );
}
