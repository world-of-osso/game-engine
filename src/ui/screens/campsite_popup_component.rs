use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;

use super::campsite_component::{
    CAMPSITE_PANEL_TOP_OFFSET, CAMPSITE_PANEL_WIDTH, campsite_panel, campsite_panel_height,
};
use super::char_select_component::CampsiteState;

pub const CAMPSITE_POPUP_ROOT: FrameName = FrameName("CampsitePopupRoot");
pub const CAMPSITE_POPUP_MOUNT: FrameName = FrameName("CampsitePopupMount");

pub fn campsite_popup_screen(ctx: &SharedContext) -> Element {
    let campsite = ctx
        .get::<CampsiteState>()
        .expect("CampsitePopup screen requires CampsiteState");
    let mount_height = campsite_panel_height(campsite.scenes.len()) + CAMPSITE_PANEL_TOP_OFFSET;

    rsx! {
        r#frame {
            name: CAMPSITE_POPUP_ROOT,
            stretch: true,
            background_color: "0.03,0.02,0.01,1.0",
            strata: FrameStrata::Background,
            r#frame {
                name: CAMPSITE_POPUP_MOUNT,
                width: CAMPSITE_PANEL_WIDTH,
                height: mount_height,
                pos_type: "absolute",
                left: "50%",
                top: "50%",
                translate_x: "-50%",
                translate_y: "-50%",
                margin_top: {-29.0},
                {campsite_panel(campsite)}
            }
        }
    }
}

#[cfg(test)]
#[path = "menu_character_layout_test_support.rs"]
mod layout_support;

#[cfg(test)]
mod tests {
    use super::*;

    use ui_toolkit::screen::Screen;

    use crate::ui::registry::FrameRegistry;
    use crate::ui::screens::char_select_component::{CampsiteEntry, CampsiteState};

    #[test]
    fn popup_screen_renders_visible_campsite_panel() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(CampsiteState {
            scenes: vec![CampsiteEntry {
                id: 1,
                name: "Adventurer's Rest".to_string(),
                preview_image: Some("data/ui/campsites/adventurers-rest.ktx2".to_string()),
            }],
            panel_visible: true,
            selected_id: Some(1),
        });
        Screen::new(campsite_popup_screen).sync(&shared, &mut reg);

        layout_support::compute_layout(&mut reg);
        assert!(reg.get_by_name("CampsitePopupRoot").is_some());
        let mount_id = reg
            .get_by_name("CampsitePopupMount")
            .expect("CampsitePopupMount");
        let panel_id = reg.get_by_name("CampsitePanel").expect("CampsitePanel");
        let panel = reg.get(panel_id).expect("panel frame");
        assert!(!panel.hidden);
        assert_eq!(panel.parent_id, Some(mount_id));
        let panel_rect = panel.layout_rect.as_ref().expect("native panel bounds");
        let mount_rect = reg.get(mount_id).unwrap().layout_rect.as_ref().unwrap();
        assert!((panel_rect.y - mount_rect.y - 58.0).abs() < 1.0);
        assert!(
            (panel_rect.x + panel_rect.width * 0.5 - mount_rect.x - mount_rect.width * 0.5).abs()
                < 1.0
        );
        assert_eq!(panel.resolved_width(), 470.0);
    }
}
