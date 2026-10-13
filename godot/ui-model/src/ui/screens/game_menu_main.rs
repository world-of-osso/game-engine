//! MainMenuFrameTemplates.xml and DialogTemplates.xml; section spacing from MainMenuFrameMixin.
use crate::quest_art::{DynName, named_atlas_texture};
use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

pub const GAME_MENU_ROOT: FrameName = FrameName("GameMenuRoot");
pub(super) const MENU_MOUNT: FrameName = FrameName("GameMenuMount");
pub(super) const MENU_PANEL: FrameName = FrameName("GameMenuPanel");
pub(super) const TITLE_FRAME: FrameName = FrameName("GameMenuTitleFrame");
pub(super) const TITLE_LABEL: FrameName = FrameName("GameMenuTitleLabel");
pub(super) const BUTTON_W: f32 = 200.0;
pub(super) const BUTTON_H: f32 = 36.0;
pub(super) const PANEL_W: f32 = 260.0;
const TOP_PADDING: f32 = 48.0;
const BOTTOM_PADDING: f32 = 34.0;
const SIDE_PADDING: f32 = 28.0;
const SECTION_GAP: f32 = 20.0;

pub const ACTION_OPTIONS: &str = "menu_options";
pub const ACTION_SUPPORT: &str = "menu_support";
pub const ACTION_ADDONS: &str = "menu_addons";
pub const ACTION_LOGOUT: &str = "menu_logout";
pub const ACTION_EXIT: &str = "menu_exit";
pub const ACTION_RESUME: &str = "menu_resume";

fn panel_title() -> Element {
    // DialogHeaderMixin:UpdateWidth: rendered text width + headerTextPadding64.
    let text = "Game Menu";
    let width = ui_toolkit::text_measure::measure_text(
        text,
        crate::ui::widgets::font_string::GameFont::FrizQuadrata,
        14.0,
    )
    .expect("DialogHeader requires loaded GameFontNormalMed1")
    .0 + 64.0;
    rsx! {
        r#frame {
            name: TITLE_FRAME, width, height: 39.0, pos_type: "absolute",
            left: "50%", top: -11.0, translate_x: "-50%",
            {named_atlas_texture("GameMenuHeaderLeft".into(), "UI-Frame-DiamondMetal-Header-CornerLeft", (0.0, 0.0, 32.0, 39.0))}
            {named_atlas_texture("GameMenuHeaderRight".into(), "UI-Frame-DiamondMetal-Header-CornerRight", (width - 32.0, 0.0, 32.0, 39.0))}
            {named_atlas_texture("GameMenuHeaderCenter".into(), "_UI-Frame-DiamondMetal-Header-Tile", (32.0, 0.0, width - 64.0, 39.0))}
            fontstring {
                name: TITLE_LABEL, text, font_size: 14.0, color: "1,0.82,0,1",
                width: {width - 64.0}, height: 16.0, pos_type: "absolute",
                left: "50%", top: 13.0, translate_x: "-50%",
            }
        }
    }
}

pub fn dialog_border(prefix: &str, width: f32, height: f32) -> Element {
    // Dialog NineSlice layout: source atlas corners/edges are64px, no external offsets.
    const CORNER: f32 = 64.0;
    [
        (
            "TopLeft",
            "UI-Frame-DiamondMetal-CornerTopLeft",
            (0.0, 0.0, CORNER, CORNER),
        ),
        (
            "TopRight",
            "UI-Frame-DiamondMetal-CornerTopRight",
            (width - CORNER, 0.0, CORNER, CORNER),
        ),
        (
            "BottomLeft",
            "UI-Frame-DiamondMetal-CornerBottomLeft",
            (0.0, height - CORNER, CORNER, CORNER),
        ),
        (
            "BottomRight",
            "UI-Frame-DiamondMetal-CornerBottomRight",
            (width - CORNER, height - CORNER, CORNER, CORNER),
        ),
        (
            "Top",
            "_UI-Frame-DiamondMetal-EdgeTop",
            (CORNER, 0.0, width - 2.0 * CORNER, CORNER),
        ),
        (
            "Bottom",
            "_UI-Frame-DiamondMetal-EdgeBottom",
            (CORNER, height - CORNER, width - 2.0 * CORNER, CORNER),
        ),
        (
            "Left",
            "!UI-Frame-DiamondMetal-EdgeLeft",
            (0.0, CORNER, CORNER, height - 2.0 * CORNER),
        ),
        (
            "Right",
            "!UI-Frame-DiamondMetal-EdgeRight",
            (width - CORNER, CORNER, CORNER, height - 2.0 * CORNER),
        ),
    ]
    .into_iter()
    .flat_map(|(part, atlas, rect)| named_atlas_texture(format!("{prefix}{part}"), atlas, rect))
    .collect()
}

fn menu_button(name: &str, text: &str, action: &str) -> Element {
    // Native SharedButton projection scales the authored128-RedButton three slices.
    rsx! { button {
        name: DynName(name.into()), width: BUTTON_W, height: BUTTON_H, text, font_size: 16.0,
        strata: FrameStrata::Fullscreen, frame_level: 20.0, onclick: action,
        button_default_skin: false, button_atlas_up: "128-redbutton-left",
    } }
}

pub fn sync_menu_button_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    crate::options_menu_component::sync_options_art(registry);
    if let Some(id) = registry.get_by_name("GameMenuBackground")
        && let Some(frame) = registry.get_mut(id)
        && let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = frame.widget_data.as_mut()
    {
        texture.horiz_tile = true;
        texture.vert_tile = true;
    }
}

fn section_spacer(name: &str) -> Element {
    rsx! { r#frame { name: DynName(name.into()), width: BUTTON_W, height: SECTION_GAP, } }
}
fn menu_buttons(logged_in: bool) -> Element {
    let mut items = vec![
        menu_button("MenuBtnOptions", "Options", ACTION_OPTIONS),
        section_spacer("Spacer1"),
        menu_button("MenuBtnSupport", "Support", ACTION_SUPPORT),
        menu_button("MenuBtnAddons", "AddOns", ACTION_ADDONS),
        section_spacer("Spacer2"),
    ];
    if logged_in {
        items.push(menu_button("MenuBtnLogout", "Log Out", ACTION_LOGOUT));
    }
    items.extend([
        menu_button("MenuBtnExit", "Exit Game", ACTION_EXIT),
        section_spacer("Spacer3"),
        menu_button("MenuBtnResume", "Return to Game", ACTION_RESUME),
    ]);
    items.into_iter().flatten().collect()
}

pub fn main_menu_view(logged_in: bool) -> Element {
    let height = TOP_PADDING
        + BOTTOM_PADDING
        + SECTION_GAP * 3.0
        + BUTTON_H * if logged_in { 6.0 } else { 5.0 };
    rsx! {
        r#frame {
            name: GAME_MENU_ROOT, stretch: true, background_color: "0.01,0.01,0.02,0.75",
            strata: FrameStrata::Dialog, mouse_enabled: true,
            r#frame {
                name: MENU_MOUNT, width: PANEL_W, height, pos_type: "absolute",
                left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%",
                r#frame {
                    name: MENU_PANEL, width: PANEL_W, height, strata: FrameStrata::Fullscreen,
                    texture { name: "GameMenuBackground", width: "auto", height: "auto", texture_fdid: 131071,
                        pos_type: "absolute", left: 7.0, right: 7.0, top: 7.0, bottom: 7.0, }
                    {dialog_border("GameMenuBorder", PANEL_W, height)}
                    r#frame {
                        name: "GameMenuContents", width: {PANEL_W - SIDE_PADDING * 2.0}, height: {height - TOP_PADDING - BOTTOM_PADDING},
                        layout: "flex-column", gap: 0.0, align: "start",
                        margin_top: TOP_PADDING, margin_left: SIDE_PADDING,
                        {menu_buttons(logged_in)}
                    }
                    {panel_title()}
                }
            }
        }
    }
}

/// UiTextureAtlasElementSliceData SliceMode0: corners retain source size; middle spans stretch.
/// Margins passed from the pinned local export, never inferred from image dimensions.
pub fn sliced_atlas(
    prefix: &str,
    atlas: &str,
    [w, h]: [f32; 2],
    [l, t, r, b]: [f32; 4],
) -> Element {
    let region = ui_toolkit::atlas::get_region(atlas).expect("Glue sliced atlas must exist");
    let source_x = [0.0, l, region.width - r, region.width];
    let source_y = [0.0, t, region.height - b, region.height];
    let target_x = [0.0, l, w - r, w];
    let target_y = [0.0, t, h - b, h];
    (0..3)
        .flat_map(|y| {
            (0..3).flat_map(move |x| {
                let left = source_x[x] / region.width;
                let right = source_x[x + 1] / region.width;
                let top = source_y[y] / region.height;
                let bottom = source_y[y + 1] / region.height;
                let tex_coords = format!("{left},{right},{top},{bottom}");
                rsx! { texture {
                    name: DynName(format!("{prefix}{y}{x}")), texture_atlas: atlas, tex_coords,
                    width: {target_x[x+1] - target_x[x]}, height: {target_y[y+1] - target_y[y]},
                    pos_type: "absolute", left: target_x[x], top: target_y[y],
                } }
            })
        })
        .collect()
}
