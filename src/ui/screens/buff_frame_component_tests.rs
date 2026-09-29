use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::screen::Screen;

const SCREEN_W: f32 = 1920.0;

fn aura(spell_id: u32, is_debuff: bool, remaining: f32) -> AuraInstance {
    AuraInstance {
        instance_id: spell_id,
        spell_id,
        name: format!("Spell {spell_id}"),
        description: String::new(),
        icon_fdid: 135987,
        source: String::new(),
        from_local_player: true,
        from_player: true,
        duration: 3600.0,
        remaining,
        stacks: 1,
        is_debuff,
        debuff_type: DebuffType::None,
    }
}

fn registry(state: BuffFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(SCREEN_W, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(buff_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn frame<'a>(reg: &'a FrameRegistry, name: &str) -> &'a ui_toolkit::frame::Frame {
    reg.get(reg.get_by_name(name).expect(name)).expect(name)
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    frame(reg, name)
        .layout_rect
        .clone()
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

fn right_edge(rect: &LayoutRect) -> f32 {
    rect.x + rect.width
}

fn font_color(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    match &frame(reg, name).widget_data {
        Some(WidgetData::FontString(fs)) => fs.color,
        other => panic!("{name} is not a fontstring: {other:?}"),
    }
}

fn tex_coords(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    match &frame(reg, name).widget_data {
        Some(WidgetData::Texture(texture)) => texture.tex_coords,
        other => panic!("{name} is not a texture: {other:?}"),
    }
}

fn auras(list: Vec<AuraInstance>) -> BuffFrameState {
    BuffFrameState::from_auras(&list, false)
}

fn state_with(buffs: usize, debuffs: usize) -> BuffFrameState {
    auras(
        (0..buffs)
            .map(|i| aura(i as u32 + 1, false, 300.0))
            .chain((0..debuffs).map(|i| aura(i as u32 + 100, true, 30.0)))
            .collect(),
    )
}

#[test]
fn buffs_start_at_the_retail_anchor_and_grow_left_eleven_per_row() {
    let reg = registry(state_with(12, 0));
    let root = rect(&reg, "BuffFrame");
    assert_eq!(
        (right_edge(&root), root.y, root.width, root.height),
        (SCREEN_W - 255.0, 10.0, 400.0, 135.0)
    );
    let first = rect(&reg, "BuffButton0");
    assert_eq!((first.width, first.height), (30.0, 40.0));
    // Right of the first icon is the 15-wide collapse button slot.
    assert_eq!((right_edge(&first), first.y), (SCREEN_W - 270.0, 10.0));
    let second = rect(&reg, "BuffButton1");
    assert_eq!((first.x - second.x, second.y), (35.0, 10.0));
    let eleventh = rect(&reg, "BuffButton10");
    assert_eq!(eleventh.x, first.x - 10.0 * 35.0);
    let wrapped = rect(&reg, "BuffButton11");
    assert_eq!((wrapped.x, wrapped.y), (first.x, 55.0));
}

#[test]
fn debuffs_keep_their_own_anchor_eight_per_row_whatever_the_buff_count() {
    for buffs in [0, 23] {
        let reg = registry(state_with(buffs, 9));
        let root = rect(&reg, "DebuffFrame");
        assert_eq!(
            (right_edge(&root), root.y, root.width, root.height),
            (SCREEN_W - 270.0, 155.0, 280.0, 90.0)
        );
        let first = rect(&reg, "DebuffButton0");
        assert_eq!((right_edge(&first), first.y), (SCREEN_W - 270.0, 155.0));
        let eighth = rect(&reg, "DebuffButton7");
        assert_eq!((eighth.x, eighth.y), (first.x - 7.0 * 35.0, 155.0));
        let wrapped = rect(&reg, "DebuffButton8");
        assert_eq!((wrapped.x, wrapped.y), (first.x, 200.0));
    }
}

#[test]
fn duration_sits_under_the_icon_and_turns_white_below_ninety_seconds() {
    let mut permanent = aura(4, false, 0.0);
    permanent.duration = 0.0;
    let reg = registry(auras(vec![
        aura(1, false, 300.0),
        aura(2, false, 89.5),
        aura(3, true, 60.0),
        permanent,
    ]));
    let yellow = [1.0, 0.82, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    assert_eq!(fontstring_text(&reg, "BuffButton0Duration"), "5 m");
    assert_eq!(font_color(&reg, "BuffButton0Duration"), yellow);
    assert_eq!(fontstring_text(&reg, "BuffButton1Duration"), "89 s");
    assert_eq!(font_color(&reg, "BuffButton1Duration"), white);
    assert_eq!(fontstring_text(&reg, "DebuffButton0Duration"), "60 s");
    assert_eq!(font_color(&reg, "DebuffButton0Duration"), white);
    assert_eq!(fontstring_text(&reg, "BuffButton2Duration"), "");
    let icon = rect(&reg, "BuffButton0Icon");
    let duration = rect(&reg, "BuffButton0Duration");
    assert_eq!(duration.y, icon.y + 30.0);
    assert_eq!(duration.x + duration.width / 2.0, icon.x + icon.width / 2.0);
}

#[test]
fn debuff_border_uses_the_dispel_type_atlas_and_buffs_have_none() {
    let typed = |spell_id, debuff_type| AuraInstance {
        debuff_type,
        ..aura(spell_id, true, 20.0)
    };
    let reg = registry(auras(vec![
        aura(1, false, 300.0),
        typed(10, DebuffType::Magic),
        typed(11, DebuffType::Poison),
        typed(12, DebuffType::None),
    ]));
    assert!(reg.get_by_name("BuffButton0Border").is_none());
    let coords = |left: f32, right: f32, top: f32, bottom: f32| {
        [left / 256.0, right / 256.0, top / 128.0, bottom / 128.0]
    };
    // ui-debuff-border-magic-icon, -poison-icon, -default-noicon.
    assert_eq!(
        tex_coords(&reg, "DebuffButton0Border"),
        coords(85.0, 125.0, 43.0, 83.0)
    );
    assert_eq!(
        tex_coords(&reg, "DebuffButton1Border"),
        coords(127.0, 167.0, 1.0, 41.0)
    );
    assert_eq!(
        tex_coords(&reg, "DebuffButton2Border"),
        coords(43.0, 83.0, 43.0, 83.0)
    );
    let icon = rect(&reg, "DebuffButton0Icon");
    let border = rect(&reg, "DebuffButton0Border");
    assert_eq!(
        (border.x, border.y, border.width, border.height),
        (icon.x - 5.0, icon.y - 5.0, 40.0, 40.0)
    );
}

#[test]
fn count_shows_above_one_stack_and_colorblind_mode_adds_the_dispel_symbol() {
    let poison = AuraInstance {
        debuff_type: DebuffType::Poison,
        stacks: 3,
        ..aura(200, true, 30.0)
    };
    let list = vec![aura(1, false, 300.0), poison];
    let reg = registry(auras(list.clone()));
    assert_eq!(fontstring_text(&reg, "BuffButton0Count"), "");
    assert_eq!(fontstring_text(&reg, "DebuffButton0Count"), "3");
    assert_eq!(fontstring_text(&reg, "DebuffButton0Symbol"), "");
    let colorblind = registry(BuffFrameState::from_auras(&list, true));
    assert_eq!(fontstring_text(&colorblind, "DebuffButton0Symbol"), "Po");
    assert_eq!(fontstring_text(&colorblind, "BuffButton0Symbol"), "");
}

#[test]
fn only_auras_under_thirty_one_seconds_flash_between_point_three_and_one() {
    assert_eq!(aura_warning_alpha(0.0, None), 1.0);
    assert_eq!(aura_warning_alpha(0.0, Some(31.0)), 1.0);
    assert_eq!(aura_warning_alpha(0.0, Some(30.9)), 0.3);
    assert_eq!(aura_warning_alpha(0.75, Some(5.0)), 1.0);
    assert!((aura_warning_alpha(0.375, Some(5.0)) - 0.65).abs() < 1e-5);
    assert!((aura_warning_alpha(1.125, Some(5.0)) - 0.65).abs() < 1e-5);
    assert_eq!(aura_warning_alpha(1.5, Some(5.0)), 0.3);
}

#[test]
fn caps_buffs_and_debuffs() {
    let state = state_with(40, 20);
    assert_eq!(
        (state.buffs.len(), state.debuffs.len()),
        (MAX_BUFFS, MAX_DEBUFFS)
    );
}

#[test]
fn button_lookup_walks_up_from_children() {
    let reg = registry(state_with(1, 2));
    let icon = reg.get_by_name("DebuffButton1Icon").unwrap();
    assert_eq!(buff_button_at(&reg, icon), Some((true, 1)));
    let duration = reg.get_by_name("BuffButton0Duration").unwrap();
    assert_eq!(buff_button_at(&reg, duration), Some((false, 0)));
    let root = reg.get_by_name(BUFF_FRAME.0).unwrap();
    assert_eq!(buff_button_at(&reg, root), None);
}
