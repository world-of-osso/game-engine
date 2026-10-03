//! Character/paperdoll first slice: Modern base bytes and Camelot art/layout.
use std::fmt::Write;
use std::path::PathBuf;
use game_engine_ui_model::character_frame::{CharacterFrameView, LevelLine, PaperDollSlotView, StatLine, character_frame_screen, apply_character_frame_postsetup};
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn dump_frame(registry: &FrameRegistry, id: u64, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(out, "{:?} {:?} size={:?},{:?} pos={:?},{:?} anchor={:?} translate={:?} margin={:?} layout={:?} visibility={},{} alpha={},{} scale={},{} strata={:?} level={} raise={} layer={:?},{} input={},{},{:?} bg={:?} backdrop={:?} nine={:?} three={:?} border={:?} style={:?},{:?} behavior={},{},{} click={:?} flex={:?} data={:?}",
        f.name, f.widget_type, f.width, f.height, f.position, f.position_type, f.anchor,
        f.translation, f.margin, f.layout_rect, f.hidden, f.visible, f.alpha, f.effective_alpha,
        f.scale, f.effective_scale, f.strata, f.frame_level, f.raise_order, f.draw_layer,
        f.draw_sub_layer, f.mouse_enabled, f.keyboard_enabled, f.hit_rect_insets,
        f.background_color, f.backdrop, f.nine_slice, f.three_slice, f.border,
        f.panel_style, f.three_slice_style, f.clamped_to_screen, f.movable, f.resizable,
        f.onclick, f.flex_layout, f.widget_data).unwrap();
    for child in &f.children {
        dump_frame(registry, *child, out);
    }
}

fn view(populated: bool) -> CharacterFrameView {
    let mut view = CharacterFrameView {
        visible: populated,
        title: "Azerothia".into(),
        level: LevelLine { level: "Level 60 ".into(), class_text: "Protection Warrior".into(), class_color: "0.78,0.61,0.43,1.0".into() },
        slots: vec![PaperDollSlotView::default(); 18],
        item_level: populated.then(|| "42".into()),
        attributes: Vec::new(), enhancements: Vec::new(), race_id: 1, class_id: 1,
    };
    if populated {
        view.slots[0] = PaperDollSlotView { icon_fdid: 135274, quality_border: "0.0,1.0,0.0,1.0", count: 2, locked: true };
        view.attributes = vec![StatLine {label: "Strength:", value: "125".into()}, StatLine {label: "Stamina:", value: "98".into()}, StatLine {label: "Armor:", value: "2,450".into()}];
        view.enhancements = vec![StatLine {label: "Critical Strike:", value: "12%".into()}, StatLine {label: "Haste:", value: "5%".into()}];
    }
    view
}

fn build(view: CharacterFrameView) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0,1080.0);
    Screen::new(character_frame_screen).sync(&shared, &mut registry);
    apply_character_frame_postsetup(&mut registry);
    registry
}

fn modern_trees() -> String {
    let mut out = String::new();
    for populated in [false, true] {
        let registry = build(view(populated));
        dump_frame(&registry, registry.get_by_name("CharacterFrame").unwrap(), &mut out);
    }
    out
}

#[test]
fn capture_base_character_trees() {
    game_engine_ui_model::paths::set_data_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")).unwrap();
    set_active_skin(ActiveSkin::Modern);
    println!("BASE_CHARACTER_BEGIN\n{}BASE_CHARACTER_END", modern_trees());
}
