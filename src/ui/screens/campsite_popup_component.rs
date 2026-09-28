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
    use crate::ui::screens::char_select_component::{
        CampsiteEntry, CampsitePreview, CampsiteState,
    };

    #[test]
    fn popup_screen_renders_visible_campsite_panel() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(CampsiteState {
            scenes: vec![CampsiteEntry {
                id: 1,
                name: "Adventurer's Rest".to_string(),
                preview_image: Some(CampsitePreview {
                    fdid: 6_375_814,
                    tex_coords: [1.0 / 1024.0, 215.0 / 1024.0, 1.0 / 1024.0, 174.0 / 1024.0],
                }),
            }],
            panel_visible: true,
            selected_id: Some(1),
            page: 0,
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

    fn rect_of(reg: &FrameRegistry, name: &str) -> Option<crate::ui::layout::LayoutRect> {
        let frame = reg.get(reg.get_by_name(name)?)?;
        frame.layout_rect.clone()
    }

    /// Character select at 1280x720 with the eight authored campsites on `page`.
    fn char_select_campsites_at_720(page: usize) -> FrameRegistry {
        use crate::ui::screens::char_select_component::{
            CharSelectState, char_select_screen, size_char_select_root,
        };
        let mut reg = FrameRegistry::new(1280.0, 720.0);
        let mut shared = SharedContext::new();
        shared.insert(CharSelectState::default());
        // Hosts first build an empty campsite list, then fill it from the catalog.
        shared.insert(CampsiteState::default());
        let mut screen = Screen::new(char_select_screen);
        screen.sync(&shared, &mut reg);
        shared.insert(CampsiteState {
            scenes: (1..=8)
                .map(|id| CampsiteEntry {
                    id,
                    name: format!("Camp {id}"),
                    preview_image: None,
                })
                .collect(),
            panel_visible: true,
            selected_id: Some(1),
            page,
        });
        screen.sync(&shared, &mut reg);
        size_char_select_root(&mut reg);
        layout_support::compute_layout(&mut reg);
        reg
    }

    #[test]
    fn every_campsite_is_reachable_inside_a_720px_window() {
        let mut seen = Vec::new();
        for page in 0..2 {
            let reg = char_select_campsites_at_720(page);
            let panel = rect_of(&reg, "CampsitePanel").expect("panel rect");
            assert!(
                panel.y >= 0.0 && panel.y + panel.height <= 720.0,
                "{panel:?}"
            );
            let mut visible = vec!["CampsitePageText", "CampsitePrevPage", "CampsiteNextPage"]
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            for id in 1..=8 {
                let name = format!("CampsiteScene_{id}");
                if reg.get_by_name(&name).is_some() {
                    seen.push(id);
                    visible.push(name);
                }
            }
            for name in visible {
                let r = rect_of(&reg, &name).unwrap_or_else(|| panic!("{name} rect"));
                assert!(
                    r.x >= panel.x
                        && r.y >= panel.y
                        && r.x + r.width <= panel.x + panel.width
                        && r.y + r.height <= panel.y + panel.height,
                    "{name} {r:?} outside panel {panel:?} on page {page}"
                );
                if name.starts_with("CampsiteScene_") {
                    let paging = rect_of(&reg, "CampsitePaging").expect("paging rect");
                    assert!(
                        r.y + r.height <= paging.y,
                        "{name} overlaps paging controls"
                    );
                }
            }
        }
        assert_eq!(seen, (1..=8).collect::<Vec<_>>());
    }
}
