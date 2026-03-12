use game_engine::asset::m2;
use std::path::Path;

#[test]
fn inspect_render_flags() {
    let path = Path::new("data/models/5932799.m2");
    let m2 = m2::load_m2(path, &[0; 3]).unwrap();
    for (i, b) in m2.batches.iter().enumerate() {
        eprintln!(
            "batch {i:2}: render_flags={:#04x} blend_mode={}",
            b.render_flags, b.blend_mode
        );
    }
}
