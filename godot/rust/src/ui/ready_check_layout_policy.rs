//! READY_CHECK_BODY_NO_OVERLAP_POLICY: user-requested allocation-based frame growth.
//! Retail source has no corresponding resize; ordinary single-line bounds are untouched.
use std::collections::HashMap;

use game_engine_ui_model::ready_check_frame_component::READY_CHECK_FRAME;
use ui_toolkit::{layout::LayoutRect, registry::FrameRegistry};

pub(super) fn apply(
    registry: &FrameRegistry,
    bounds: &mut HashMap<u64, LayoutRect>,
    extra_height: f32,
) -> Result<(), String> {
    if extra_height == 0.0 {
        return Ok(());
    }
    let root = registry
        .get_by_name(READY_CHECK_FRAME)
        .ok_or("ReadyCheck policy frame is absent")?;
    // The window retains its CENTER anchor. All child offsets stay parent-relative.
    shift_subtree(registry, bounds, root, -extra_height / 2.0)?;
    for name in [
        READY_CHECK_FRAME.to_owned(),
        format!("{READY_CHECK_FRAME}Background"),
        format!("{READY_CHECK_FRAME}NineSlice"),
    ] {
        named_bounds(registry, bounds, &name)?.height += extra_height;
    }
    for suffix in ["YesButton", "NoButton"] {
        named_bounds(registry, bounds, &format!("{READY_CHECK_FRAME}{suffix}"))?.y += extra_height;
    }
    Ok(())
}

fn shift_subtree(
    registry: &FrameRegistry,
    bounds: &mut HashMap<u64, LayoutRect>,
    root: u64,
    offset_y: f32,
) -> Result<(), String> {
    let mut descendants = vec![root];
    while let Some(id) = descendants.pop() {
        let frame = registry
            .get(id)
            .ok_or_else(|| format!("ReadyCheck policy frame {id} is absent"))?;
        bounds
            .get_mut(&id)
            .ok_or_else(|| format!("ReadyCheck policy bounds {id} are absent"))?
            .y += offset_y;
        descendants.extend(&frame.children);
    }
    Ok(())
}

fn named_bounds<'a>(
    registry: &FrameRegistry,
    bounds: &'a mut HashMap<u64, LayoutRect>,
    name: &str,
) -> Result<&'a mut LayoutRect, String> {
    let id = registry
        .get_by_name(name)
        .ok_or_else(|| format!("ReadyCheck policy node {name} is absent"))?;
    bounds
        .get_mut(&id)
        .ok_or_else(|| format!("ReadyCheck policy bounds {name} are absent"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_ui_model::ready_check_frame_component::{
        ReadyCheckFrameState, ready_check_frame,
    };
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        screen::{Screen, SharedContext},
    };

    #[test]
    fn ready_check_no_overlap_policy_keeps_block_gap_and_one_line_geometry() {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut shared = SharedContext::new();
            shared.insert(ReadyCheckFrameState {
                visible: true,
                initiator: "Ann".into(),
                difficulty_name: Some("10 Player".into()),
            });
            let mut registry =
                super::super::ui_parent::UiParent::for_viewport(1920.0, 1080.0).registry();
            Screen::new(|ctx| ready_check_frame(ctx.get::<ReadyCheckFrameState>().unwrap()))
                .sync(&shared, &mut registry);
            let body = registry.get_by_name("ReadyCheckFrameText").unwrap();
            let line = 17.0 / registry.ui_scale;
            for lines in [1, 2, 3] {
                let intrinsic = HashMap::from([(body, (240.0, line * lines as f32))]);
                let mut bounds =
                    super::super::layout::compute_layout_with_intrinsics(&registry, &intrinsic)
                        .unwrap();
                let before = bounds.clone();
                apply(&registry, &mut bounds, (lines - 1) as f32 * line).unwrap();
                if lines == 1 {
                    assert_eq!(bounds, before);
                }
                let panel = &bounds[&registry.get_by_name(READY_CHECK_FRAME).unwrap()];
                let text = &bounds[&body];
                assert!((panel.height - (100.0 + (lines - 1) as f32 * line)).abs() < 0.0001);
                assert!((text.y - panel.y - 37.0).abs() < 0.0001);
                let old_panel = &before[&registry.get_by_name(READY_CHECK_FRAME).unwrap()];
                assert!(
                    (panel.y + panel.height / 2.0 - old_panel.y - old_panel.height / 2.0).abs()
                        < 0.0001
                );
                for suffix in ["Background", "NineSlice"] {
                    let id = registry
                        .get_by_name(&format!("{READY_CHECK_FRAME}{suffix}"))
                        .unwrap();
                    assert!(
                        (bounds[&id].height - before[&id].height - (lines - 1) as f32 * line).abs()
                            < 0.0001
                    );
                }
                for suffix in ["YesButton", "NoButton"] {
                    let button = &bounds[&registry
                        .get_by_name(&format!("{READY_CHECK_FRAME}{suffix}"))
                        .unwrap()];
                    assert!(
                        (panel.y + panel.height - button.y - button.height - 16.0).abs() < 0.0001
                    );
                    assert!((button.y - text.y - text.height - (23.0 - line)).abs() < 0.0001);
                }
            }
        }
    }
}
