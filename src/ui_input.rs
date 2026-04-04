use game_engine::ui::registry::FrameRegistry;

pub fn walk_up_for_onclick(reg: &FrameRegistry, mut id: u64) -> Option<String> {
    loop {
        if let Some(frame) = reg.get(id) {
            if let Some(ref action) = frame.onclick {
                return Some(action.clone());
            }
            if let Some(parent) = frame.parent_id {
                id = parent;
                continue;
            }
        }
        return None;
    }
}
