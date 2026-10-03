//! Retail PetActionBar (Blizzard_ActionBar/Mainline/PetActionBar.xml, Shared/PetActionBar.lua)
//! with the hunter bar the server sends.

use game_engine_ui_model::pet_action_bar_component::{
    PET_ACTION_BAR, PetActionBarState, PetActionSlot, PetAutocast, apply_pet_action_bar_postsetup,
    attack_flash_shown, parse_pet_action_button, pet_action_bar_screen, pet_autocast_toggle,
    pet_bar_buttons,
};
use shared::protocol::{
    ACT_COMMAND, ACT_DISABLED, ACT_ENABLED, ACT_PASSIVE, ACT_REACTION, COMMAND_ATTACK,
    COMMAND_FOLLOW, COMMAND_MOVE_TO, COMMAND_STAY, PetSpells, REACT_ASSIST, REACT_DEFENSIVE,
    REACT_PASSIVE, pet_action_button,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

const WOLF: u64 = 0x0000_0002_0000_0031;
/// `SpellMisc.SpellIconFileDataID` of Dash, Bite and Growl.
const DASH_ICON: u32 = 132_120;
const BITE_ICON: u32 = 132_127;
const GROWL_ICON: u32 = 132_270;

fn wolf_bar(command_state: u32, react_state: u32) -> PetSpells {
    PetSpells {
        pet: WOLF,
        command_state,
        react_state,
        action_buttons: [
            pet_action_button(COMMAND_ATTACK, ACT_COMMAND),
            pet_action_button(COMMAND_FOLLOW, ACT_COMMAND),
            pet_action_button(COMMAND_MOVE_TO, ACT_COMMAND),
            pet_action_button(61_684, ACT_ENABLED),
            pet_action_button(17_253, ACT_ENABLED),
            pet_action_button(2_649, ACT_ENABLED),
            0,
            pet_action_button(REACT_ASSIST, ACT_REACTION),
            pet_action_button(REACT_DEFENSIVE, ACT_REACTION),
            pet_action_button(REACT_PASSIVE, ACT_REACTION),
        ],
    }
}

fn spell_icon(spell_id: u32) -> u32 {
    match spell_id {
        61_684 => DASH_ICON,
        17_253 => BITE_ICON,
        2_649 => GROWL_ICON,
        _ => 0,
    }
}

fn hotkeys() -> [String; 10] {
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"].map(|key| format!("c-{key}"))
}

fn shown(spells: &PetSpells, pet_in_combat: bool, attack_flash: bool) -> FrameRegistry {
    let state = PetActionBarState {
        visible: true,
        buttons: pet_bar_buttons(spells, pet_in_combat, attack_flash, spell_icon, &hotkeys()),
        shine_texture: None,
    };
    build(state)
}

fn build(state: PetActionBarState) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(state.clone());
    Screen::new(pet_action_bar_screen).sync(&shared, &mut registry);
    apply_pet_action_bar_postsetup(&state, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn texture(registry: &FrameRegistry, name: &str) -> (TextureSource, [f32; 4]) {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => (texture.source.clone(), texture.vertex_color),
        other => panic!("{name} is not a Texture: {other:?}"),
    }
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(font)) => font.text.clone(),
        other => panic!("{name} is not a FontString: {other:?}"),
    }
}

fn checked(registry: &FrameRegistry, button: usize) -> bool {
    !frame(registry, &format!("PetActionButton{button}CheckedTexture")).hidden
}

/// Ten 30×30 `SmallActionButtonTemplate` buttons 2 px apart (`IconPadding` 2), laid out by
/// `ActionBarMixin:UpdateGridLayout` into a 318×30 bar; in its default position the bar
/// stacks above MainActionBar: BOTTOMLEFT at UIParent BOTTOM + (-562/2, 45 + 45 + 5)
/// (`EditModeManagerFrameMixin:UpdateBottomActionBarPositions`).
#[test]
fn pet_bar_uses_retail_small_button_geometry_above_the_main_bar() {
    let registry = shown(&wolf_bar(COMMAND_FOLLOW, REACT_ASSIST), false, false);
    let bar = frame(&registry, PET_ACTION_BAR.0);
    assert!(!bar.hidden);
    assert_eq!(bar.width, Dimension::Fixed(318.0));
    assert_eq!(bar.height, Dimension::Fixed(30.0));
    assert_eq!(bar.position.left, Val::Percent(50.0));
    assert_eq!(bar.margin.left, Val::Px(-281.0));
    assert_eq!(bar.position.bottom, Val::Px(95.0));
    for index in 1..=10 {
        let button = frame(&registry, &format!("PetActionButton{index}"));
        assert_eq!(button.width, Dimension::Fixed(30.0));
        assert_eq!(button.height, Dimension::Fixed(30.0));
        assert_eq!(
            button.position.left,
            Val::Px((index - 1) as f32 * 32.0),
            "PetActionButton{index}"
        );
        assert_eq!(
            text(&registry, &format!("PetActionButton{index}HotKey")),
            hotkeys()[index - 1]
        );
        assert_eq!(
            parse_pet_action_button(button.onclick.as_deref().unwrap()),
            Some(index - 1)
        );
    }
}

/// `PET_*_TEXTURE` tokens (PetActionBar.lua:3-12) by their listfile FDIDs; spells show
/// their icons; the empty slot shows none.
#[test]
fn pet_bar_icons_follow_the_server_buttons() {
    let registry = shown(&wolf_bar(COMMAND_FOLLOW, REACT_ASSIST), false, false);
    let expected = [
        Some(132_152), // Ability_GhoulFrenzy
        Some(132_328), // Ability_Tracking
        Some(457_329), // Ability_Hunter_Pet_Goto
        Some(DASH_ICON),
        Some(BITE_ICON),
        Some(GROWL_ICON),
        None,
        Some(524_348), // Ability_Hunter_Pet_Assist
        Some(132_110), // Ability_Defend
        Some(132_311), // Ability_Seal
    ];
    for (index, fdid) in expected.into_iter().enumerate() {
        let name = format!("PetActionButton{}Icon", index + 1);
        match fdid {
            Some(fdid) => {
                assert!(!frame(&registry, &name).hidden, "{name}");
                assert_eq!(texture(&registry, &name).0, TextureSource::FileDataId(fdid));
            }
            None => assert!(frame(&registry, &name).hidden, "{name}"),
        }
    }
    assert_eq!(
        PetActionSlot::from_packed(pet_action_button(17_253, ACT_ENABLED)),
        PetActionSlot::Spell(17_253)
    );
    assert_eq!(PetActionSlot::from_packed(0), PetActionSlot::Empty);
}

/// `GetPetActionInfo` isActive: the command matching `CommandState`, the reaction
/// matching `ReactState`.
#[test]
fn follow_and_assist_are_checked() {
    let registry = shown(&wolf_bar(COMMAND_FOLLOW, REACT_ASSIST), false, false);
    let checked_buttons: Vec<usize> = (1..=10).filter(|&b| checked(&registry, b)).collect();
    assert_eq!(checked_buttons, vec![2, 8]);
    assert!(frame(&registry, "PetActionButton1Flash").hidden);
    let registry = shown(&wolf_bar(COMMAND_STAY, REACT_DEFENSIVE), false, false);
    let checked_buttons: Vec<usize> = (1..=10).filter(|&b| checked(&registry, b)).collect();
    assert_eq!(checked_buttons, vec![9], "no Stay button on this bar");
}

/// `UNIT_FLAG_PET_IN_COMBAT`: `IsPetAttackAction` → `StartFlash`, checked texture at alpha
/// 0.5 (PetActionBar.lua:166-172); the Flash toggles every `ATTACK_BUTTON_FLASH_TIME`.
#[test]
fn attack_is_checked_and_flashing_while_the_pet_attacks() {
    let spells = wolf_bar(COMMAND_FOLLOW, REACT_ASSIST);
    let registry = shown(&spells, true, true);
    assert!(checked(&registry, 1));
    assert_eq!(
        texture(&registry, "PetActionButton1CheckedTexture").1[3],
        0.5
    );
    assert_eq!(
        texture(&registry, "PetActionButton2CheckedTexture").1[3],
        1.0
    );
    assert!(!frame(&registry, "PetActionButton1Flash").hidden);
    let registry = shown(&spells, true, false);
    assert!(checked(&registry, 1));
    assert!(frame(&registry, "PetActionButton1Flash").hidden);
    assert!(attack_flash_shown(0.1));
    assert!(!attack_flash_shown(0.5));
    assert!(attack_flash_shown(0.9));
}

/// No `PetSpells` (or after `PetClearSpells`): PetActionBar hidden.
#[test]
fn no_pet_bar_without_pet_spells() {
    let registry = build(PetActionBarState::default());
    assert!(frame(&registry, PET_ACTION_BAR.0).hidden);
}

fn shown_or_hidden(registry: &FrameRegistry, part: &str) -> Vec<usize> {
    (1..=10)
        .filter(|button| !frame(registry, &format!("PetActionButton{button}{part}")).hidden)
        .collect()
}

/// `GetPetActionInfo` autoCastAllowed/autoCastEnabled → `AutoCastOverlay:SetShown` and
/// `ShowAutoCastEnabled` (PetActionBar.lua:154-155): every autocastable spell
/// (`ACT_ENABLED`/`ACT_DISABLED`) shows the overlay's Corners; the rotating Shine only
/// while autocast is on (`AutoCastOverlayMixin:UpdateShineAnim`).
#[test]
fn autocast_overlay_follows_the_spell_buttons_autocast_state() {
    let mut spells = wolf_bar(COMMAND_FOLLOW, REACT_PASSIVE);
    // Bite turned off; Growl not autocastable (`SPELL_ATTR1_NO_AUTOCAST_AI` → ACT_PASSIVE).
    spells.action_buttons[4] = pet_action_button(17_253, ACT_DISABLED);
    spells.action_buttons[5] = pet_action_button(2_649, ACT_PASSIVE);
    let buttons = pet_bar_buttons(&spells, false, false, spell_icon, &hotkeys());
    let autocast: Vec<PetAutocast> = buttons.iter().map(|button| button.autocast).collect();
    use PetAutocast::{Off, On, Unavailable};
    assert_eq!(
        autocast,
        [
            Unavailable,
            Unavailable,
            Unavailable,
            On,
            Off,
            Unavailable,
            Unavailable,
            Unavailable,
            Unavailable,
            Unavailable
        ]
    );
    let shine = ui_toolkit::widgets::texture::DynamicTextureId(7);
    let registry = build(PetActionBarState {
        visible: true,
        buttons,
        shine_texture: Some(shine),
    });
    assert_eq!(shown_or_hidden(&registry, "AutoCastCorners"), vec![4, 5]);
    assert_eq!(shown_or_hidden(&registry, "AutoCastShine"), vec![4]);
    // UI-HUD-ActionBar-PetAutoCast-Corners (UiTextureAtlasMember 25944, atlas FDID 5199404)
    // over the 31×31 overlay at CENTER + (0.5, -0.5).
    let corners = frame(&registry, "PetActionButton4AutoCastCorners");
    assert_eq!(corners.width, Dimension::Fixed(31.0));
    assert_eq!(
        texture(&registry, "PetActionButton4AutoCastCorners").0,
        TextureSource::FileDataId(5_199_404)
    );
    // The Shine is 10 px larger than the overlay (TOPLEFT -5,5 / BOTTOMRIGHT 5,-5) and
    // draws the host's rotated, masked ants.
    let shine_frame = frame(&registry, "PetActionButton4AutoCastShine");
    assert_eq!(shine_frame.width, Dimension::Fixed(41.0));
    assert_eq!(shine_frame.position.left, Val::Px(-5.0));
    assert_eq!(
        texture(&registry, "PetActionButton4AutoCastShine").0,
        TextureSource::Dynamic(shine)
    );
}

/// `TogglePetAutocast` (a right click, PetActionBar.lua:271): an autocastable spell flips
/// its autocast; commands, stances and spells that cannot autocast do nothing.
#[test]
fn right_click_toggles_autocast_of_autocastable_spells_only() {
    assert_eq!(
        pet_autocast_toggle(pet_action_button(17_253, ACT_ENABLED)),
        Some((17_253, false))
    );
    assert_eq!(
        pet_autocast_toggle(pet_action_button(17_253, ACT_DISABLED)),
        Some((17_253, true))
    );
    assert_eq!(
        pet_autocast_toggle(pet_action_button(2_649, ACT_PASSIVE)),
        None
    );
    assert_eq!(
        pet_autocast_toggle(pet_action_button(COMMAND_ATTACK, ACT_COMMAND)),
        None
    );
    assert_eq!(
        pet_autocast_toggle(pet_action_button(REACT_PASSIVE, ACT_REACTION)),
        None
    );
    assert_eq!(pet_autocast_toggle(0), None);
}
