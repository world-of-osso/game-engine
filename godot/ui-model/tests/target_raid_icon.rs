//! TargetFrame `RaidTargetIcon` (Blizzard_UnitFrame/Mainline/TargetFrame.xml:275-279,
//! TargetFrame.lua:672-686): 26×26 `UI-RaidTargetingIcons` cell centred on the portrait's
//! TOP, shown only while the target carries an icon.

use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, RAID_TARGET_ICONS_FDID, UnitFrameMenuState, UnitFrameState,
    inworld_unit_frames_screen, raid_target_tex_coords,
};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn target_frames(raid_target: Option<u8>) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Fbraidicons"),
        target: Some(UnitFrameState {
            raid_target,
            ..UnitFrameState::named("Hogger")
        }),
        target_of_target: None,
        focus: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

/// `SetSpriteSheetCell(index, 4, 4)`: cells run left to right, top to bottom.
#[test]
fn sprite_sheet_cells_follow_the_retail_icon_order() {
    assert_eq!(raid_target_tex_coords(1), [0.0, 0.25, 0.0, 0.25]); // Star
    assert_eq!(raid_target_tex_coords(4), [0.75, 1.0, 0.0, 0.25]); // Triangle
    assert_eq!(raid_target_tex_coords(5), [0.0, 0.25, 0.25, 0.5]); // Moon
    assert_eq!(raid_target_tex_coords(8), [0.75, 1.0, 0.25, 0.5]); // Skull
}

/// Portrait 58×58 TOPRIGHT (-26, -19) of the 232×100 frame: its TOP centre is (177, 19),
/// (157, -7) of the portrait-off art, so the 26px icon spans 144..170 × -20..6.
#[test]
fn skull_sits_on_the_portrait_top() {
    let registry = target_frames(Some(8));
    let icon = frame(&registry, "TargetRaidTargetIcon");
    assert!(!icon.hidden);
    match icon.widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => {
            assert_eq!(
                texture.source,
                TextureSource::FileDataId(RAID_TARGET_ICONS_FDID)
            );
            assert_eq!(texture.tex_coords, [0.75, 1.0, 0.25, 0.5]);
        }
        other => panic!("TargetRaidTargetIcon is not a Texture: {other:?}"),
    }
    assert_eq!(
        (
            icon.position.left,
            icon.position.top,
            icon.width,
            icon.height
        ),
        (
            Val::Px(144.0),
            Val::Px(-20.0),
            Dimension::Fixed(26.0),
            Dimension::Fixed(26.0)
        )
    );
}

#[test]
fn unmarked_target_hides_the_icon() {
    let registry = target_frames(None);
    assert!(frame(&registry, "TargetRaidTargetIcon").hidden);
}
