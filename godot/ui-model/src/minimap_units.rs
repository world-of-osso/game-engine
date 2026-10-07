//! Party positions remain available through GroupMemberStates outside replica interest.
use game_engine_core::minimap_data::{MapMask, MinimapView};

use super::{BLIP_SIZE, BlipKind, MinimapBlip};
use crate::group_state::GroupState;

pub fn group_minimap_blips(
    group: &GroupState,
    local_name: Option<&str>,
    view: &MinimapView,
    map_size: f32,
) -> Vec<MinimapBlip> {
    group
        .members
        .iter()
        .filter_map(|member| {
            if !member.online || Some(member.name.as_str()) == local_name {
                return None;
            }
            let live = group.live.get(&member.name)?;
            let (offset, edge) = member_offset(view, [live.position.x, live.position.z], map_size);
            Some(MinimapBlip {
                unit: member.character_id,
                kind: BlipKind::Member {
                    class: member.class,
                    raid: group.is_raid,
                    edge,
                },
                offset,
            })
        })
        .collect()
}

/// North-up screen direction, clamped along its ray to the skin's boundary. Leave
/// half the icon inside the rim; use an arrow only outside the actual map mask.
fn member_offset(view: &MinimapView, [x, z]: [f32; 2], map_size: f32) -> ([f32; 2], bool) {
    let right = (z - view.center[1]) / view.diameter;
    let down = (view.center[0] - x) / view.diameter;
    let distance = match view.mask {
        MapMask::Round => right.hypot(down),
        MapMask::Square => right.abs().max(down.abs()),
    };
    let inset_edge = 0.5 - BLIP_SIZE / (2.0 * map_size);
    let scale = inset_edge / distance.max(inset_edge);
    (
        [right * scale, down * scale],
        !view.mask.contains(right, down),
    )
}

pub fn target_minimap_blip(
    unit: u64,
    position: [f32; 2],
    view: &MinimapView,
) -> Option<MinimapBlip> {
    Some(MinimapBlip {
        unit,
        kind: BlipKind::Target,
        offset: view.blip_offset(position)?,
    })
}
