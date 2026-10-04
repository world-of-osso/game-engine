use game_engine_ui_model::aura_display_data::DebuffType;
use game_engine_ui_model::buff_frame_component::{
    BuffFrameState, BuffIconState, buff_frame_screen, buff_frame_texture_fdids,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

const STATIC_FLIGHT_ICON: u32 = 5_142_726;

fn icon(fdid: u32, dispel: Option<DebuffType>) -> BuffIconState {
    BuffIconState {
        icon_fdid: fdid,
        timer_text: String::new(),
        timer_warning: false,
        stacks: 1,
        dispel,
        symbol: "",
    }
}

#[test]
fn buff_frame_cache_requests_cover_static_flight_and_drawn_debuff_art() {
    let state = BuffFrameState {
        buffs: vec![icon(STATIC_FLIGHT_ICON, None)],
        debuffs: vec![icon(136_116, Some(DebuffType::Magic))],
    };
    let requested = buff_frame_texture_fdids(&state);
    assert!(requested.contains(&STATIC_FLIGHT_ICON));
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(state.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(buff_frame_screen).sync(&shared, &mut registry);
        let mut drawn: Vec<u32> = registry
            .frames_iter()
            .filter_map(|frame| match frame.widget_data.as_ref()? {
                WidgetData::Texture(texture) => match texture.source {
                    TextureSource::FileDataId(fdid) => Some(fdid),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        drawn.sort_unstable();
        let mut cached = requested.clone();
        cached.sort_unstable();
        assert_eq!(cached, drawn);
        assert_eq!(drawn, vec![136_116, STATIC_FLIGHT_ICON, 7_553_349]);
    }
    assert!(buff_frame_texture_fdids(&BuffFrameState::default()).is_empty());
}
