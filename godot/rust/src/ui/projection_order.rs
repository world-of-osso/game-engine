//! Dense native layers preserve the registry's strata/level precedence without z overflow.
use std::collections::{BTreeMap, BTreeSet};
use ui_toolkit::{
    frame::Frame,
    strata::{DrawLayer, FrameStrata},
};

type EffectiveLayer = (FrameStrata, i32, i32, DrawLayer);

fn effective_layer(frame: &Frame) -> EffectiveLayer {
    (
        frame.strata,
        frame.frame_level,
        frame.raise_order,
        frame.draw_layer,
    )
}

pub(super) fn rank_layers<'a>(
    frames: impl Iterator<Item = &'a Frame>,
    capacity: usize,
) -> Result<BTreeMap<EffectiveLayer, i32>, String> {
    let layers: BTreeSet<_> = frames.map(effective_layer).collect();
    if layers.len() > capacity {
        return Err(format!(
            "Native UI needs {} distinct render layers; only {capacity} fit below highlight overlays",
            layers.len()
        ));
    }
    Ok(layers
        .into_iter()
        .enumerate()
        .map(|(rank, layer)| {
            (
                layer,
                i32::try_from(rank).expect("native z capacity fits i32"),
            )
        })
        .collect())
}

pub(super) fn native_z(layers: &BTreeMap<EffectiveLayer, i32>, frame: &Frame) -> i32 {
    layers[&effective_layer(frame)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(strata: FrameStrata, level: i32, raise: i32) -> Frame {
        Frame {
            strata,
            frame_level: level,
            raise_order: raise,
            ..Frame::default()
        }
    }

    #[test]
    fn projection_order_strata_precedes_unbounded_levels_and_raise_is_preserved() {
        let frames = [
            frame(FrameStrata::Dialog, i32::MAX, 0),
            frame(FrameStrata::Tooltip, i32::MIN, 0),
            frame(FrameStrata::Dialog, 20, 1),
            frame(FrameStrata::Dialog, 20, 0),
        ];
        let layers = rank_layers(frames.iter(), 4).unwrap();
        assert!(native_z(&layers, &frames[1]) > native_z(&layers, &frames[0]));
        assert!(native_z(&layers, &frames[2]) > native_z(&layers, &frames[3]));
        assert_eq!(layers.values().copied().collect::<Vec<_>>(), [0, 1, 2, 3]);
    }

    #[test]
    fn projection_order_many_frames_share_layers_and_capacity_is_explicit() {
        let repeated = vec![frame(FrameStrata::Dialog, 1000, 0); 10000];
        let layers = rank_layers(repeated.iter(), 1).unwrap();
        assert_eq!(layers.len(), 1);
        assert_eq!(native_z(&layers, &repeated[9999]), 0);
        let distinct = [
            frame(FrameStrata::Dialog, 0, 0),
            frame(FrameStrata::Dialog, 1, 0),
        ];
        assert_eq!(
            rank_layers(distinct.iter(), 1).unwrap_err(),
            "Native UI needs 2 distinct render layers; only 1 fit below highlight overlays"
        );
        assert_eq!(rank_layers(std::iter::empty(), 0).unwrap().len(), 0);
    }
}
