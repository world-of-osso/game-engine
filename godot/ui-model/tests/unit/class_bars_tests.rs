use super::*;
use crate::status::{ClassBar, ClassBarResource};
#[cfg(feature = "dev")]
use crate::ui::layout::LayoutRect;
use crate::ui::registry::FrameRegistry;
#[cfg(feature = "dev")]
#[path = "../../src/ui/screens/menu_character_layout_test_support.rs"]
mod layout_test_support;
use crate::ui::widgets::texture::{TextureData, TextureSource};
use class_bars::{ClassBarAnimator, settled_view};
#[cfg(feature = "dev")]
use layout_test_support::compute_layout;
use ui_toolkit::screen::Screen;

fn resource(bar: ClassBar, current: u8, max: u8) -> ClassBarResource {
    ClassBarResource {
        bar,
        current,
        max,
        tenths: u16::from(current) * 10,
        spec: None,
        dynamics: crate::status::ClassBarDynamics {
            regen_per_sec: if bar == ClassBar::Essence { 0.2 } else { 0.0 },
            runes: (bar == ClassBar::Runes).then(|| crate::status::ClassBarRunes {
                duration_ms: 10000,
                ready_in_ms: (0..6)
                    .map(|index| {
                        if index < current {
                            0
                        } else {
                            u32::from(index - current + 1) * 10000
                        }
                    })
                    .collect(),
            }),
            ..Default::default()
        },
        in_combat: false,
    }
}

#[test]
fn class_bar_visibility_follows_class_spec_level_and_display_power() {
    use crate::status::{ClassBarPlayer, power_display_modifier};
    use PowerType::{Energy, Mana, Rage, RunicPower};
    use shared::components::{PowerEntry, PowerType, UnitPowers};
    let cases = [
        (4, None, 1, Energy, true),
        (11, Some(103), 20, Energy, true),
        (11, Some(103), 20, Mana, false),
        (11, Some(104), 20, Rage, false),
        (10, Some(269), 20, Energy, true),
        (10, Some(268), 20, Energy, false),
        (10, None, 20, Energy, false),
        (9, Some(267), 9, Mana, false),
        (9, Some(267), 10, Mana, true),
        (13, None, 20, Mana, true),
        (6, Some(250), 20, RunicPower, true),
        (2, None, 1, Mana, true),
        (8, Some(62), 20, Mana, true),
        (8, Some(63), 20, Mana, false),
    ];
    for (class, spec, level, primary, shown) in cases {
        let bar = ClassBar::for_class(class).unwrap();
        let modifier = power_display_modifier(bar.power());
        let powers = UnitPowers {
            entries: vec![
                PowerEntry {
                    power: primary,
                    current: 100,
                    max: 100,
                    partial: 0,
                    regen_per_sec: 0.0,
                },
                PowerEntry {
                    power: bar.power(),
                    current: 3 * modifier,
                    max: 5 * modifier,
                    partial: 0,
                    regen_per_sec: 0.0,
                },
            ],
            ..Default::default()
        };
        let player = ClassBarPlayer {
            class,
            spec,
            level,
            in_combat: false,
        };
        let value = ClassBarResource::for_player(&powers, None, &player);
        assert_eq!(
            value.is_some(),
            shown,
            "class {class}, spec {spec:?}, level {level}, primary {primary:?}"
        );
        let mut animator = ClassBarAnimator::default();
        assert_eq!(
            animator
                .update_received(42, value.as_ref(), 100.0)
                .is_some(),
            shown
        );
    }
}

#[test]
fn missing_rune_timing_clears_old_swipes_instead_of_inventing_cooldowns() {
    let mut runes = resource(ClassBar::Runes, 6, 6);
    let mut animator = ClassBarAnimator::default();
    animator.update_received(42, Some(&runes), 100.0);
    runes.current = 0;
    runes.dynamics.runes = None;
    let view = animator.update_received(42, Some(&runes), 101.0).unwrap();
    assert!(view.textures.iter().all(|texture| texture.swipe.is_none()));
    for pip in 0..6 {
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}Rune_Active"),
            0.0,
        );
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}BG_Inactive"),
            1.0,
        );
    }
}

fn texture<'a>(view: &'a ClassBarView, name: &str) -> &'a TextureView {
    view.textures
        .iter()
        .find(|texture| texture.name == name)
        .unwrap_or_else(|| panic!("no {name} in {:?}", names(view)))
}

fn names(view: &ClassBarView) -> Vec<&str> {
    view.textures
        .iter()
        .map(|texture| texture.name.as_str())
        .collect()
}

fn alpha(view: &ClassBarView, name: &str) -> f32 {
    let texture = texture(view, name);
    if texture.shown { texture.alpha } else { 0.0 }
}

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 1e-4
}

#[track_caller]
fn assert_alpha(view: &ClassBarView, name: &str, expected: f32) {
    let actual = alpha(view, name);
    assert!(
        close(actual, expected),
        "{name} alpha {actual}, expected {expected}"
    );
}

fn centre(view: &ClassBarView, name: &str) -> (f32, f32) {
    let (x, y, width, height) = texture(view, name).rect;
    (x + width / 2.0, y + height / 2.0)
}

/// Frames `animator` through `steps` of `(power, seconds)`, returning the last view.
fn run(animator: &mut ClassBarAnimator, steps: &[(&ClassBarResource, f64)]) -> ClassBarView {
    let mut view = None;
    for (resource, now) in steps {
        view = animator.update(Some(resource), *now);
    }
    view.expect("a shown bar")
}

fn registry_with(view: ClassBarView) -> FrameRegistry {
    let mut state = InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: false,
        player: UnitFrameState::named("Player"),
        target: None,
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    };
    state.player.power = Some(PowerBarState {
        power: shared::components::PowerType::Mana,
        current: 1000,
        max: 1000,
    });
    state.player.class_bar = Some(view);
    let mut context = SharedContext::new();
    context.insert(state);
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&context, &mut reg);
    #[cfg(feature = "dev")]
    compute_layout(&mut reg);
    reg
}

#[cfg(feature = "dev")]
fn rect_by_name(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .expect(name)
}

fn registry_texture<'a>(reg: &'a FrameRegistry, name: &str) -> &'a TextureData {
    match reg
        .get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(ui_toolkit::frame::WidgetData::Texture(texture)) => texture,
        _ => panic!("{name} is not a texture"),
    }
}

fn hidden(reg: &FrameRegistry, name: &str) -> bool {
    reg.get(reg.get_by_name(name).expect(name)).unwrap().hidden
}

/// Retail Arcane Charges: container top 4 px below the mana bar plus `topPadding` 7, centred
/// 1 px left of the bar (PlayerFrame.lua:716,758; MageArcaneChargesBar.xml:134), four 21 px
/// charges 10 px apart (MageArcaneChargesBar.xml:6,125).
#[cfg(feature = "dev")]
#[test]
fn arcane_charges_hang_below_the_mana_bar_clear_of_its_text() {
    let view = settled_view(&resource(ClassBar::ArcaneCharges, 0, 4)).unwrap();
    let reg = registry_with(view);
    let mana = rect_by_name(&reg, "PlayerManaBar");
    let row = rect_by_name(&reg, "PlayerSecondaryResourceRow");
    assert_eq!(row.y, mana.y + mana.height + 11.0);
    assert_eq!(row.x + row.width / 2.0, mana.x + mana.width / 2.0 - 1.0);
    assert_eq!((row.width, row.height), (4.0 * 21.0 + 3.0 * 10.0, 21.0));
}

/// `ArcaneChargeTemplate` (MageArcaneChargesBar.xml:5-53): shadow 2.5 px low, background
/// and orb on every charge, the arcane icon only on active ones once their groups end.
#[test]
fn settled_arcane_charges_draw_shadow_background_orb_and_active_icons() {
    let view = settled_view(&resource(ClassBar::ArcaneCharges, 1, 4)).unwrap();
    for pip in 0..4 {
        for part in ["ArcaneBGShadow", "ArcaneBG", "Orb"] {
            assert_alpha(
                &view,
                &format!("PlayerSecondaryResourcePip{pip}{part}"),
                1.0,
            );
        }
    }
    assert_alpha(&view, "PlayerSecondaryResourcePip0ArcaneIcon", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip1ArcaneIcon", 0.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip0FrameGlow", 0.0);
    let (_, bg_y) = centre(&view, "PlayerSecondaryResourcePip0ArcaneBG");
    let (_, shadow_y) = centre(&view, "PlayerSecondaryResourcePip0ArcaneBGShadow");
    assert!(close(shadow_y - bg_y, 2.5));
    let reg = registry_with(view);
    let orb = registry_texture(&reg, "PlayerSecondaryResourcePip0Orb");
    assert_eq!(orb.source, TextureSource::FileDataId(5_045_210));
    assert!(hidden(&reg, "PlayerSecondaryResourcePip1ArcaneIcon"));
    assert!(!hidden(&reg, "PlayerSecondaryResourcePip0ArcaneIcon"));
}

/// activateAnim at 0.75 s: MagicCirc half way through 1 → 0 over 0.5..1.0, the icon at 1,
/// ShockFX on flipbook frame 21 of 28 (column 3, row 3 of the 50×45 cells). A charge that
/// did not change keeps its settled state; the bar appearing animates every charge.
#[test]
fn an_activating_arcane_charge_draws_its_retail_keyframes() {
    let mut animator = ClassBarAnimator::default();
    let empty = resource(ClassBar::ArcaneCharges, 0, 4);
    let one = resource(ClassBar::ArcaneCharges, 1, 4);
    let appearing = run(&mut animator, &[(&empty, 10.0), (&empty, 10.04)]);
    // deactivateAnim FrameGlow 0 → 1 over 0.08 s on every charge.
    assert_alpha(&appearing, "PlayerSecondaryResourcePip3FrameGlow", 0.5);
    let view = run(
        &mut animator,
        &[(&empty, 20.0), (&one, 20.0), (&one, 20.75)],
    );
    assert_alpha(&view, "PlayerSecondaryResourcePip0ArcaneCircle", 0.5);
    assert_alpha(&view, "PlayerSecondaryResourcePip0ArcaneIcon", 1.0);
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip0FrameGlow",
        0.45 + 0.25 * (0.58 / 0.6),
    );
    let shock = texture(&view, "PlayerSecondaryResourcePip0FBArcaneFX");
    assert_eq!(shock.art.rect, (151.0, 201.0, 136.0, 181.0));
    assert_eq!((shock.rect.2, shock.rect.3), (50.0, 45.0));
    assert_alpha(&view, "PlayerSecondaryResourcePip1ArcaneCircle", 0.0);
    // activateAnim ends with FrameGlow at 0.97 + 0.2 s.
    let done = run(&mut animator, &[(&one, 21.2)]);
    assert_alpha(&done, "PlayerSecondaryResourcePip0FrameGlow", 0.0);
    assert_alpha(&done, "PlayerSecondaryResourcePip0ArcaneIcon", 1.0);
    // deactivateAnim: the icon fades over 0.15 s.
    let spent = run(&mut animator, &[(&empty, 30.0), (&empty, 30.075)]);
    assert_alpha(&spent, "PlayerSecondaryResourcePip0ArcaneIcon", 0.5);
}

/// `RogueComboPointTemplate`: full points show the red icon on the active background,
/// empty ones the disabled background; the shadow sits 4 px low; points past five pull
/// the bar 20 px left each (RogueComboPointBar.lua:1-33).
#[test]
fn settled_rogue_combo_points_and_their_padding() {
    let view = settled_view(&resource(ClassBar::RogueComboPoints, 2, 5)).unwrap();
    assert_alpha(&view, "PlayerSecondaryResourcePip1IconUncharged", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip1BGActive", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip1BGInactive", 0.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip2IconUncharged", 0.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip2BGInactive", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip4IconCharged", 0.0);
    let (_, bg) = centre(&view, "PlayerSecondaryResourcePip0BGActive");
    let (_, shadow) = centre(&view, "PlayerSecondaryResourcePip0BGShadow");
    assert!(close(shadow - bg, 4.0));
    assert_eq!(view.size, (5.0 * 20.0 + 4.0 * 4.0, 20.0));
    assert_eq!(view.padding, (10.0, 0.0));
    let seven = settled_view(&resource(ClassBar::RogueComboPoints, 0, 7)).unwrap();
    assert_eq!(seven.padding, (10.0, -40.0));
}

/// `unchargedEmptyToUnchargedFull` 0.3 s in: slash frame 8 of 17 (column 2, row 1 of the
/// 43×43 cells), the icon half way up its second key; `unchargedFullToUnchargedEmpty`
/// 0.25 s in: FrameGlow half faded.
#[test]
fn rogue_combo_points_transition_with_their_retail_groups() {
    let mut animator = ClassBarAnimator::default();
    let two = resource(ClassBar::RogueComboPoints, 2, 5);
    let three = resource(ClassBar::RogueComboPoints, 3, 5);
    let one = resource(ClassBar::RogueComboPoints, 1, 5);
    let view = run(
        &mut animator,
        &[(&two, 0.0), (&two, 10.0), (&three, 10.0), (&three, 10.3)],
    );
    let slash = texture(&view, "PlayerSecondaryResourcePip2SlashFBUncharged");
    assert_eq!(slash.art.rect, (87.0, 130.0, 175.0, 218.0));
    assert_alpha(&view, "PlayerSecondaryResourcePip2SlashFBUncharged", 1.0);
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip2IconUncharged",
        0.5 + 0.5 * (0.03 / 0.27),
    );
    assert_alpha(&view, "PlayerSecondaryResourcePip2BGInactive", 1.0);
    let spent = run(
        &mut animator,
        &[(&three, 20.0), (&one, 20.0), (&one, 20.25)],
    );
    for pip in [1, 2] {
        assert_alpha(
            &spent,
            &format!("PlayerSecondaryResourcePip{pip}FrameGlow"),
            0.5,
        );
        assert_alpha(
            &spent,
            &format!("PlayerSecondaryResourcePip{pip}BGActive"),
            1.0 - 0.05 / 0.17,
        );
    }
    assert_alpha(&spent, "PlayerSecondaryResourcePip0FrameGlow", 0.0);
}

/// `DruidComboPointTemplate` activateAnim 0.3 s in: the active background swapped in at
/// 0.27 s, FB_Slash on frame 6 of 20; deactivateAnim 0.28 s in: the smoke 3.5 px up its
/// 7 px rise. A spent point keeps the active background (the groups never restore it).
#[test]
fn druid_combo_points_activate_and_deactivate() {
    let mut animator = ClassBarAnimator::default();
    let none = resource(ClassBar::DruidComboPoints, 0, 5);
    let one = resource(ClassBar::DruidComboPoints, 1, 5);
    let view = run(
        &mut animator,
        &[(&none, 0.0), (&none, 10.0), (&one, 10.0), (&one, 10.3)],
    );
    assert_alpha(&view, "PlayerSecondaryResourcePip0BG_Active", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip0BG_Inactive", 0.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip0Point_Icon", 0.5);
    let slash = texture(&view, "PlayerSecondaryResourcePip0FB_Slash");
    assert_eq!(slash.art.rect, (157.0, 183.0, 1.0, 42.0));
    assert!(slash.shown);
    assert!(!texture(&view, "PlayerSecondaryResourcePip1FB_Slash").shown);
    let (_, rest) = centre(
        &run(&mut animator, &[(&one, 20.0)]),
        "PlayerSecondaryResourcePip0Smoke",
    );
    let spent = run(&mut animator, &[(&none, 30.0), (&none, 30.28)]);
    let (_, rising) = centre(&spent, "PlayerSecondaryResourcePip0Smoke");
    assert!(close(rest - rising, 3.5), "smoke {rest} → {rising}");
    assert_alpha(&spent, "PlayerSecondaryResourcePip0Smoke", 1.0);
    let settled = run(&mut animator, &[(&none, 40.0)]);
    assert_alpha(&settled, "PlayerSecondaryResourcePip0BG_Active", 1.0);
    assert_alpha(&settled, "PlayerSecondaryResourcePip0BG_Inactive", 0.0);
    assert_alpha(&settled, "PlayerSecondaryResourcePip0Point_Icon", 0.0);
}

/// `MonkLightEnergyTemplate`: chi orbs 3 apart below six, 2 from six (MonkHarmonyBar.lua);
/// activate 0.6 s in has the icon on its second key and the background gone; Chi_FX_2
/// turns -65° over 0.43 s.
#[test]
fn chi_orbs_space_and_animate_like_retail() {
    let four = settled_view(&resource(ClassBar::Chi, 2, 4)).unwrap();
    assert_eq!(four.size.0, 4.0 * 21.0 + 3.0 * 3.0);
    let six = settled_view(&resource(ClassBar::Chi, 2, 6)).unwrap();
    assert_eq!(six.size.0, 6.0 * 21.0 + 5.0 * 2.0);
    assert_alpha(&four, "PlayerSecondaryResourcePip0Chi_Icon", 1.0);
    assert_alpha(&four, "PlayerSecondaryResourcePip0Chi_BG_Active", 1.0);
    assert_alpha(&four, "PlayerSecondaryResourcePip0Chi_BG", 0.0);
    assert_alpha(&four, "PlayerSecondaryResourcePip3Chi_Icon", 0.0);
    assert_alpha(&four, "PlayerSecondaryResourcePip3Chi_BG", 1.0);
    assert_alpha(&four, "PlayerSecondaryResourcePip3Orb_Gleam", 1.0);

    let mut animator = ClassBarAnimator::default();
    let none = resource(ClassBar::Chi, 0, 4);
    let one = resource(ClassBar::Chi, 1, 4);
    let turning = run(
        &mut animator,
        &[(&none, 0.0), (&none, 10.0), (&one, 10.0), (&one, 10.215)],
    );
    assert!(close(
        texture(&turning, "PlayerSecondaryResourcePip0Chi_FX_2").rotation,
        -32.5
    ));
    let reg = registry_with(turning.clone());
    assert!(close(
        registry_texture(&reg, "PlayerSecondaryResourcePip0Chi_FX_2").rotation,
        -32.5,
    ));
    assert_alpha(&turning, "PlayerSecondaryResourcePip0FB_Wind_FX", 1.0);
    let view = run(&mut animator, &[(&one, 10.6)]);
    assert_alpha(&view, "PlayerSecondaryResourcePip0Chi_Icon", 0.2 / 0.43);
    assert_alpha(&view, "PlayerSecondaryResourcePip0Chi_BG", 0.0);
    let spent = run(&mut animator, &[(&none, 20.0), (&none, 20.25)]);
    let deplete = texture(&spent, "PlayerSecondaryResourcePip0Chi_Deplete");
    assert!(close(deplete.rotation, -15.0));
    assert_alpha(&spent, "PlayerSecondaryResourcePip0Chi_Deplete", 0.5);
    assert_alpha(&spent, "PlayerSecondaryResourcePip0Chi_BG", 1.0);
}

fn shards(tenths: u16, spec: u32, in_combat: bool) -> ClassBarResource {
    ClassBarResource {
        tenths,
        spec: Some(spec),
        in_combat,
        ..resource(ClassBar::SoulShards, (tenths / 10) as u8, 5)
    }
}

/// Destruction draws soul shard fragments: 3.7 shards fill the fourth shard with
/// `UF-SoulShard-Inc7` 5 px low; other specs floor to whole shards (ShardBar.lua:23-26).
#[test]
fn destruction_draws_soul_shard_fragments() {
    let view = settled_view(&shards(37, 267, false)).unwrap();
    for pip in 0..3 {
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}Shard_Icon"),
            1.0,
        );
    }
    assert_alpha(&view, "PlayerSecondaryResourcePip3Shard_Icon", 0.0);
    let fill = view
        .textures
        .iter()
        .find(|texture| {
            texture.name == "PlayerSecondaryResourcePip3FillIncrementFill" && texture.shown
        })
        .expect("a shown increment");
    assert_eq!(fill.art.rect, (90.0, 113.0, 194.0, 223.0));
    let (_, holder) = centre(&view, "PlayerSecondaryResourcePip3Background");
    let fill_y = fill.rect.1 + fill.rect.3 / 2.0;
    assert!(close(fill_y - holder, 0.5), "Inc7 at y -5, holder at -4.5");
    let affliction = settled_view(&shards(37, 265, false)).unwrap();
    assert!(
        affliction
            .textures
            .iter()
            .filter(|texture| texture.name.ends_with("FillIncrementFill"))
            .all(|texture| !texture.shown)
    );
    assert_eq!(view.padding, (-2.0, 5.0));
    assert_eq!(view.size, (5.0 * 23.0 + 4.0, 30.0));
}

/// A shard emptying from 0.7 plays DepleteC, from 0.4 DepleteB, from 0.1 DepleteA
/// (ShardBar.lua:93-103); FrameGlow fades 1 → 0 over the 0.5 s group.
#[test]
fn emptied_soul_shards_pick_their_deplete_flipbook() {
    for (from, book) in [
        (37, "FB_DepleteC"),
        (34, "FB_DepleteB"),
        (31, "FB_DepleteA"),
    ] {
        let mut animator = ClassBarAnimator::default();
        let view = run(
            &mut animator,
            &[
                (&shards(from, 267, false), 0.0),
                (&shards(from, 267, false), 10.0),
                (&shards(30, 267, false), 10.0),
                (&shards(30, 267, false), 10.25),
            ],
        );
        for other in ["FB_DepleteA", "FB_DepleteB", "FB_DepleteC"] {
            let shown = texture(&view, &format!("PlayerSecondaryResourcePip3{other}")).shown;
            assert_eq!(shown, other == book, "{from}: {other}");
        }
        assert_alpha(&view, "PlayerSecondaryResourcePip3Frame_Glow", 0.5);
    }
}

/// In combat with every shard full, `readyLoopAnim` bounces IconGlow 0 ↔ 1 and FrameGlow
/// 0.25 ↔ 0.6 over 0.65 s (ShardBar.xml:131-134).
#[test]
fn full_soul_shards_pulse_in_combat() {
    let mut animator = ClassBarAnimator::default();
    let rest = shards(50, 265, false);
    let fight = shards(50, 265, true);
    let view = run(
        &mut animator,
        &[
            (&rest, 0.0),
            (&rest, 10.0),
            (&fight, 10.0),
            (&fight, 10.325),
        ],
    );
    assert_alpha(&view, "PlayerSecondaryResourcePip0Shard_IconGlow", 0.5);
    assert_alpha(&view, "PlayerSecondaryResourcePip0Frame_Glow", 0.425);
    assert_alpha(&view, "PlayerSecondaryResourcePip0Shard_Icon", 1.0);
    let back = run(&mut animator, &[(&fight, 10.975)]);
    assert_alpha(&back, "PlayerSecondaryResourcePip4Shard_IconGlow", 0.5);
    assert!(!texture(&back, "PlayerSecondaryResourcePip0Shard_Soul").shown);
}

/// EssencePlayerFrame: full points show EssenceFillDone; the next point fills over
/// `FillingAnimationTime` 5 s (IconProg_B 0 → 0.8 over 2.2..2.7 s, the timer spinner half
/// way through its -360° turn at 2.5 s) then shows EssenceFillDone.
#[test]
fn essence_fills_the_next_point_over_five_seconds() {
    let full = settled_view(&resource(ClassBar::Essence, 5, 5)).unwrap();
    for pip in 0..5 {
        assert_alpha(
            &full,
            &format!("PlayerSecondaryResourcePip{pip}FillDoneEssenceIcon"),
            1.0,
        );
    }
    let mut animator = ClassBarAnimator::default();
    let mut three = resource(ClassBar::Essence, 3, 5);
    three.dynamics.received_at = 100.0;
    let mut four = resource(ClassBar::Essence, 4, 5);
    four.dynamics.received_at = 105.2;
    let view = run(&mut animator, &[(&three, 100.0), (&three, 102.5)]);
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip3FillingIconProg_B",
        0.8 * 0.3 / 0.5,
    );
    assert_alpha(&view, "PlayerSecondaryResourcePip3FillingTimerSpinner", 1.0);
    let spinner = texture(&view, "PlayerSecondaryResourcePip3FillingTimerSpinner");
    assert!(close(spinner.rotation, -180.0), "{}", spinner.rotation);
    assert_alpha(&view, "PlayerSecondaryResourcePip2FillDoneEssenceIcon", 1.0);
    assert!(!texture(&view, "PlayerSecondaryResourcePip4FillingEssenceBG").shown);
    let done = run(&mut animator, &[(&three, 105.125)]);
    assert_alpha(&done, "PlayerSecondaryResourcePip3FillDoneEssenceIcon", 0.5);
    let next = run(&mut animator, &[(&four, 105.2), (&four, 107.7)]);
    assert_alpha(
        &next,
        "PlayerSecondaryResourcePip4FillingIconProg_B",
        0.8 * 0.3 / 0.5,
    );
}

/// Spending essence plays EssenceDepleting's AnimIn on the emptied points after the new
/// filling point: 0.4 s in the icon is at 0.2 and the smoke at full alpha.
#[test]
fn spent_essence_depletes() {
    let mut animator = ClassBarAnimator::default();
    let five = resource(ClassBar::Essence, 5, 5);
    let mut two = resource(ClassBar::Essence, 2, 5);
    two.dynamics.received_at = 10.0;
    let view = run(
        &mut animator,
        &[(&five, 0.0), (&five, 10.0), (&two, 10.0), (&two, 10.4)],
    );
    for pip in [3, 4] {
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}DepletingEssenceIcon"),
            0.2,
        );
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}DepletingFXSmoke"),
            1.0,
        );
    }
    assert_alpha(&view, "PlayerSecondaryResourcePip2FillingEssenceBG", 1.0);
    assert!(!texture(&view, "PlayerSecondaryResourcePip2FillDoneCircBG").shown);
}

/// RuneFrame at scale 0.95: six ready runes show the spec skull on the active background.
/// Spending two queues them behind each other (10 s a rune): the first swipes its
/// `-LevelBar` and fills, the second waits empty; both play the deplete flipbook at the
/// first non-ready positions, after the ready runes.
#[test]
fn runes_spend_recharge_and_sort_like_retail() {
    let mut blood = resource(ClassBar::Runes, 6, 6);
    blood.spec = Some(250);
    let view = settled_view(&blood).unwrap();
    assert!(close(view.size.0, (6.0 * 24.0 - 5.0) * 0.95));
    assert_eq!(view.padding, (6.0 * 0.95, -5.0 * 0.95));
    for pip in 0..6 {
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}Rune_Active"),
            1.0,
        );
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}BG_Active"),
            1.0,
        );
        assert_alpha(
            &view,
            &format!("PlayerSecondaryResourcePip{pip}BG_Inactive"),
            0.0,
        );
    }
    // UF-DKRunes-Blood-SkullActive (19121).
    let skull = texture(&view, "PlayerSecondaryResourcePip0Rune_Active");
    assert_eq!(skull.art.rect, (30.0, 46.0, 234.0, 251.0));

    let mut four = blood.clone();
    four.current = 4;
    four.dynamics.received_at = 100.0;
    four.dynamics.runes.as_mut().unwrap().ready_in_ms = vec![0, 0, 0, 0, 10000, 20000];
    let mut animator = ClassBarAnimator::default();
    let spent = run(
        &mut animator,
        &[
            (&blood, 90.0),
            (&blood, 100.0),
            (&four, 100.0),
            (&four, 100.05),
        ],
    );
    for pip in 0..4 {
        assert_alpha(
            &spent,
            &format!("PlayerSecondaryResourcePip{pip}Rune_Active"),
            1.0,
        );
        assert!(
            !texture(
                &spent,
                &format!("PlayerSecondaryResourcePip{pip}FB_RuneDeplete")
            )
            .shown
        );
    }
    for pip in [4, 5] {
        assert!(
            texture(
                &spent,
                &format!("PlayerSecondaryResourcePip{pip}FB_RuneDeplete")
            )
            .shown
        );
        assert_alpha(
            &spent,
            &format!("PlayerSecondaryResourcePip{pip}Rune_Active"),
            0.0,
        );
    }
    let half = run(&mut animator, &[(&four, 105.0)]);
    let swipe = texture(&half, "PlayerSecondaryResourcePip4Cooldown");
    assert_eq!((swipe.swipe, swipe.shown), (Some(0.5), true));
    assert_eq!(swipe.art.rect, (59.0, 86.0, 147.0, 174.0));
    assert!(!texture(&half, "PlayerSecondaryResourcePip5Cooldown").shown);
    // CooldownEndingAnim starts 0.67 s before the rune is back: Rune_Active at 0.77 s.
    let ending = run(&mut animator, &[(&four, 109.4), (&four, 110.2)]);
    assert_alpha(&ending, "PlayerSecondaryResourcePip4Rune_Active", 1.0);
    let mut five = four.clone();
    five.current = 5;
    five.dynamics.received_at = 110.3;
    five.dynamics.runes.as_mut().unwrap().ready_in_ms = vec![0, 0, 0, 0, 0, 10000];
    let back = run(&mut animator, &[(&five, 110.3), (&five, 115.0)]);
    let swipe = texture(&back, "PlayerSecondaryResourcePip5Cooldown");
    assert!(close(swipe.swipe.unwrap(), 4.7 / 10.0), "{:?}", swipe.swipe);
}

#[test]
fn charged_combo_points_drive_full_and_empty_charged_layers() {
    let mut charged = resource(ClassBar::RogueComboPoints, 2, 5);
    charged.dynamics.charged_points = vec![2, 4];
    let mut animator = ClassBarAnimator::default();
    let view = run(&mut animator, &[(&charged, 100.0), (&charged, 102.0)]);
    assert_alpha(&view, "PlayerSecondaryResourcePip1IconCharged", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip1IconUncharged", 0.0);
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip3ChargedFrameInactive",
        1.0,
    );
    assert_alpha(&view, "PlayerSecondaryResourcePip0IconUncharged", 1.0);
    let mut uncharged = charged.clone();
    uncharged.dynamics.charged_points.clear();
    let view = run(&mut animator, &[(&uncharged, 103.0), (&uncharged, 105.0)]);
    assert_alpha(&view, "PlayerSecondaryResourcePip1IconCharged", 0.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip1IconUncharged", 1.0);
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip3ChargedFrameInactive",
        0.0,
    );
}

#[test]
fn charged_combo_points_play_blue_slash_and_charged_empty_keyframes() {
    let empty = resource(ClassBar::RogueComboPoints, 0, 5);
    let mut charged = resource(ClassBar::RogueComboPoints, 1, 5);
    charged.dynamics.charged_points = vec![1, 4];
    let mut animator = ClassBarAnimator::default();
    let view = run(
        &mut animator,
        &[
            (&empty, 0.0),
            (&empty, 10.0),
            (&charged, 10.0),
            (&charged, 10.3),
        ],
    );
    let slash = texture(&view, "PlayerSecondaryResourcePip0SlashFBCharged");
    assert_eq!(slash.art.fdid, 4902605);
    assert_eq!(slash.art.rect, (87.0, 130.0, 44.0, 87.0));
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip0IconCharged",
        0.5 + 0.5 * 0.03 / 0.27,
    );
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip0ChargedFrameActive",
        0.1 / 0.17,
    );
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip3ChargedFrameInactive",
        1.0,
    );
    assert_alpha(
        &view,
        "PlayerSecondaryResourcePip3ChargedFrameGlow",
        1.0 - 0.13 / 0.33,
    );
    assert_alpha(&view, "PlayerSecondaryResourcePip3IconCharged", 0.0);
}

#[test]
fn essence_starts_from_received_fraction_at_received_rate() {
    let mut three = resource(ClassBar::Essence, 3, 5);
    three.dynamics.partial = 500;
    three.dynamics.regen_per_sec = 0.4;
    three.dynamics.received_at = 100.0;
    let mut animator = ClassBarAnimator::default();
    // Half filled at receipt, 60% at first presentation, not restarted from zero.
    let view = run(&mut animator, &[(&three, 100.25)]);
    assert!(close(
        texture(&view, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation,
        -244.8
    ));
    let view = run(&mut animator, &[(&three, 100.75)]);
    assert!(close(
        texture(&view, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation,
        -331.2
    ));
    let view = run(&mut animator, &[(&three, 101.375)]);
    assert_alpha(&view, "PlayerSecondaryResourcePip3FillDoneEssenceIcon", 0.5);
}

#[test]
fn received_rune_clock_survives_pool_count_arriving_before_rune_component() {
    let mut runes = resource(ClassBar::Runes, 4, 6);
    runes.dynamics.runes = Some(crate::status::ClassBarRunes {
        duration_ms: 8000,
        ready_in_ms: vec![0, 0, 0, 0, 2000, 6000],
    });
    let mut animator = ClassBarAnimator::default();
    animator.update_received(42, Some(&runes), 100.0);
    // UnitPowers and UnitRunes are distinct components: count alone is not a timer receipt.
    runes.current = 5;
    let view = animator.update_received(42, Some(&runes), 101.0).unwrap();
    let swipes: Vec<_> = view.textures.iter().filter_map(|t| t.swipe).collect();
    assert_eq!(swipes, vec![0.875, 0.375]);
    runes.dynamics.runes.as_mut().unwrap().ready_in_ms = vec![0, 0, 0, 0, 0, 5000];
    let view = animator.update_received(42, Some(&runes), 101.5).unwrap();
    let swipes: Vec<_> = view.textures.iter().filter_map(|t| t.swipe).collect();
    assert_eq!(swipes, vec![0.375]);
}

#[test]
fn received_resource_clock_advances_across_repeated_frames_and_combat_events() {
    let mut essence = resource(ClassBar::Essence, 3, 5);
    essence.dynamics.partial = 500;
    let mut animator = ClassBarAnimator::default();
    let first = animator.update_received(42, Some(&essence), 100.0).unwrap();
    assert!(close(
        texture(&first, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation,
        -180.0
    ));
    let next = animator.update_received(42, Some(&essence), 101.0).unwrap();
    assert!(close(
        texture(&next, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation,
        -295.2
    ));
    essence.in_combat = true;
    let fight = animator.update_received(42, Some(&essence), 101.5).unwrap();
    assert!(close(
        texture(&fight, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation,
        -331.2
    ));
    essence.dynamics.partial = 100;
    let correction = animator.update_received(42, Some(&essence), 102.0).unwrap();
    assert!(close(
        texture(
            &correction,
            "PlayerSecondaryResourcePip3FillingTimerSpinner"
        )
        .rotation,
        -7.2
    ));
    let later = animator.update_received(42, Some(&essence), 103.0).unwrap();
    assert!(close(
        texture(&later, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation,
        -64.8
    ));
    let other_player = animator.update_received(43, Some(&essence), 103.0).unwrap();
    assert!(close(
        texture(
            &other_player,
            "PlayerSecondaryResourcePip3FillingTimerSpinner"
        )
        .rotation,
        -7.2
    ));
}

#[test]
fn essence_rate_change_retimes_the_existing_partial_point() {
    let mut essence = resource(ClassBar::Essence, 3, 5);
    essence.dynamics.partial = 500;
    essence.dynamics.regen_per_sec = 0.4;
    essence.dynamics.received_at = 100.0;
    let mut animator = ClassBarAnimator::default();
    animator.update(Some(&essence), 100.0);
    essence.dynamics.partial = 900;
    essence.dynamics.regen_per_sec = 0.1;
    essence.dynamics.received_at = 101.0;
    let view = run(&mut animator, &[(&essence, 101.0), (&essence, 101.5)]);
    assert!(close(
        texture(&view, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation,
        -358.2
    ));
    assert!(!texture(&view, "PlayerSecondaryResourcePip3FillDoneEssenceIcon").shown);
}

/// `GetPowerRegenForPowerType` returning 0 reads as 0.2 essence per second
/// (EssenceFramePlayer.lua:33-36): a half-filled point keeps filling, 3.5 s into the 5 s
/// timer one second later (IN_OUT -360° at 0.7 is -295.2°).
#[test]
fn essence_with_zero_regen_fills_at_the_retail_default_rate() {
    let mut essence = resource(ClassBar::Essence, 3, 5);
    essence.dynamics.partial = 500;
    essence.dynamics.regen_per_sec = 0.0;
    essence.dynamics.received_at = 100.0;
    let mut animator = ClassBarAnimator::default();
    let initial = run(&mut animator, &[(&essence, 100.0)]);
    let later = run(&mut animator, &[(&essence, 101.0)]);
    let spin = |view: &ClassBarView| {
        texture(view, "PlayerSecondaryResourcePip3FillingTimerSpinner").rotation
    };
    assert!(close(spin(&initial), -180.0), "{}", spin(&initial));
    assert!(close(spin(&later), -295.2), "{}", spin(&later));
    assert!(
        later
            .textures
            .iter()
            .all(|t| t.alpha.is_finite() && t.rotation.is_finite())
    );
}

#[test]
fn rune_swipes_use_independent_received_cooldowns_not_pool_count() {
    let mut runes = resource(ClassBar::Runes, 4, 6);
    runes.dynamics.received_at = 100.0;
    runes.dynamics.runes = Some(crate::status::ClassBarRunes {
        duration_ms: 8000,
        ready_in_ms: vec![0, 0, 0, 0, 2000, 6000],
    });
    let mut animator = ClassBarAnimator::default();
    let view = run(&mut animator, &[(&runes, 100.5)]);
    let swipes: Vec<_> = view.textures.iter().filter_map(|t| t.swipe).collect();
    assert_eq!(swipes, vec![0.8125, 0.3125]);
    assert!(
        view.textures
            .iter()
            .filter(|t| t.swipe.is_some())
            .all(|t| t.shown)
    );
    // A fresh timing event with the same whole count corrects both timers.
    runes.dynamics.received_at = 101.0;
    runes.dynamics.runes.as_mut().unwrap().ready_in_ms = vec![0, 0, 0, 0, 1000, 5000];
    let view = run(&mut animator, &[(&runes, 101.5)]);
    let swipes: Vec<_> = view.textures.iter().filter_map(|t| t.swipe).collect();
    assert_eq!(swipes, vec![0.9375, 0.4375]);
}

/// PaladinPowerBarFrame: the 150×43 holder with each rune centred on its LEFT anchor; lit
/// runes show their active art; from three holy power the holder's readyAnim hands over to
/// readyLoopAnim (ThinGlow 0.7 → 1 over 0.5 s).
#[test]
fn holy_power_runes_light_and_pulse_when_spell_ready() {
    let two = resource(ClassBar::HolyPower, 2, 5);
    let view = settled_view(&two).unwrap();
    assert_eq!((view.size, view.padding), ((150.0, 43.0), (-3.0, 5.0)));
    assert_alpha(&view, "PlayerSecondaryResourceHolderBackground", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourceHolderActiveTexture", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip1ActiveTexture", 1.0);
    assert_alpha(&view, "PlayerSecondaryResourcePip2ActiveTexture", 0.0);
    // rune1: LEFT (18, -1), 18×18 → centre (27, 22.5); its 23×23 active art around it.
    assert_eq!(
        texture(&view, "PlayerSecondaryResourcePip0ActiveTexture").rect,
        (15.5, 11.0, 23.0, 23.0)
    );

    let mut animator = ClassBarAnimator::default();
    let three = resource(ClassBar::HolyPower, 3, 5);
    let ready = run(
        &mut animator,
        &[(&two, 0.0), (&two, 10.0), (&three, 10.0), (&three, 10.2)],
    );
    assert_alpha(&ready, "PlayerSecondaryResourcePip0Glow", 0.07 / 0.23);
    let looping = run(&mut animator, &[(&three, 10.92)]);
    assert_alpha(&looping, "PlayerSecondaryResourceHolderThinGlow", 0.85);
    let none = resource(ClassBar::HolyPower, 0, 5);
    let spent = run(&mut animator, &[(&none, 20.0), (&none, 20.1)]);
    let book = texture(&spent, "PlayerSecondaryResourcePip2DepleteFlipbook");
    // UF-HolyPower-DepleteRune3 6×5 of 27×43 cells: frame 2 at 0.1 of 0.87 s.
    assert_eq!(
        book.art.rect,
        (1.0 + 2.0 * 27.0, 1.0 + 3.0 * 27.0, 223.0, 266.0)
    );
    assert_alpha(&spent, "PlayerSecondaryResourcePip2DepleteFlipbook", 1.0);
    assert_alpha(&spent, "PlayerSecondaryResourcePip2ActiveTexture", 0.0);
}
