use super::*;
use crate::aura_display_data::DebuffType;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn aura(instance_id: u32, is_debuff: bool, mine: bool, from_player: bool) -> AuraInstance {
    AuraInstance {
        instance_id,
        spell_id: 100 + instance_id,
        name: String::new(),
        description: String::new(),
        icon_fdid: 1000 + instance_id,
        source: String::new(),
        from_local_player: mine,
        from_player,
        duration: 8.0,
        remaining: 6.0,
        stacks: 1,
        is_debuff,
        debuff_type: DebuffType::None,
    }
}

const HOSTILE_NPC: TargetAuraView = TargetAuraView {
    player_is_target: false,
    friendly: false,
    hostile_npc: true,
};

fn ids(auras: &[&AuraInstance]) -> Vec<u32> {
    auras.iter().map(|aura| aura.instance_id).collect()
}

#[test]
fn hostile_npc_shows_own_and_npc_debuffs_own_first_but_not_other_players() {
    let auras = [
        aura(1, true, false, false), // the NPC's own debuff
        aura(2, true, false, true),  // another player's Frostbolt
        aura(3, true, true, true),   // the local player's Polymorph
        aura(4, false, false, false),
    ];
    let (buffs, debuffs) = target_frame_auras(&auras, HOSTILE_NPC);
    assert_eq!(ids(&debuffs), [3, 1]);
    assert_eq!(ids(&buffs), [4]);
    let self_target = TargetAuraView {
        player_is_target: true,
        friendly: true,
        hostile_npc: false,
    };
    let (_, debuffs) = target_frame_auras(&auras, self_target);
    assert_eq!(ids(&debuffs), [3, 1, 2]);
}

#[test]
fn icons_are_large_for_the_players_auras_and_swipe_their_elapsed_time() {
    let mine = target_aura_icon(&aura(1, true, true, true));
    assert!(mine.large);
    assert_eq!(mine.elapsed, Some(0.25));
    assert_eq!(mine.spell_id, 101);
    assert_eq!(mine.dispel_color.as_deref(), Some("0.8,0.0,0.0,1.0"));
    let permanent = target_aura_icon(&AuraInstance {
        duration: 0.0,
        ..aura(2, false, false, false)
    });
    assert!(!permanent.large);
    assert_eq!((permanent.elapsed, permanent.dispel_color), (None, None));
}

struct Frames(UnitFrameState);

fn container(ctx: &SharedContext) -> Element {
    target_auras(&ctx.get::<Frames>().unwrap().0, (0.0, 0.0))
}

fn registry(state: UnitFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(Frames(state));
    Screen::new(container).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> (f32, f32, f32) {
    let rect = reg
        .get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .expect(name);
    (rect.x, rect.y, rect.width)
}

#[test]
fn icons_flow_into_122_px_lines_and_the_second_group_starts_a_new_line() {
    // The player's large debuff leads, then small ones 20 px apart; an icon wraps once the
    // line plus its size passes 122.
    let mut state = UnitFrameState::named("Spy");
    let auras: Vec<AuraInstance> = std::iter::once(aura(1, true, true, true))
        .chain((2..=8).map(|id| aura(id, true, false, false)))
        .chain(std::iter::once(aura(9, false, false, false)))
        .collect();
    set_target_auras(&mut state, &auras, HOSTILE_NPC);
    let reg = registry(state);
    let origin = rect(&reg, "TargetFrameAuras");
    let at = |name: &str| {
        let (x, y, w) = rect(&reg, name);
        (x - origin.0, y - origin.1, w)
    };
    assert_eq!(at("TargetDebuffIcon0"), (0.0, 0.0, 21.0));
    assert_eq!(at("TargetDebuffIcon1"), (24.0, 0.0, 17.0));
    // 104 + 17 = 121 still fits; 124 + 17 wraps under the 21 px line.
    assert_eq!(at("TargetDebuffIcon5"), (104.0, 0.0, 17.0));
    assert_eq!(at("TargetDebuffIcon6"), (0.0, 24.0, 17.0));
    assert_eq!(at("TargetDebuffIcon7"), (20.0, 24.0, 17.0));
    // Buffs: new line after the 17 px second line.
    assert_eq!(at("TargetBuffIcon0"), (0.0, 44.0, 17.0));
}
