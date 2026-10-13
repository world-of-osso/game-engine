//! GlueCollections.xml703x595; GlueWarbandSceneCollection.xml journal anchors;
//! SharedCollectionTemplates.xml215x173 cards; slice margins from local FDID5319042.
use super::char_select_component::{CampsitePreview, CampsiteState, CharSelectAction};
use crate::quest_art::DynName;
use crate::ui::{
    strata::FrameStrata,
    widgets::font_string::{FontColor, GameFont},
};
use ui_toolkit::{rsx, widget_def::Element};

pub const CAMPSITE_PANEL_WIDTH: f32 = 703.0;
pub const CAMPSITE_PANEL_TOP_OFFSET: f32 = -94.0;
const PANEL_HEIGHT: f32 = 595.0;
const CARD_WIDTH: f32 = 215.0;
const CARD_HEIGHT: f32 = 173.0;
// Icons frame:4,60 ->697,542; Icons inset13,38 and12,25 =668x419.
// Three215px entries with11px x-padding, two173px rows with34px y-padding.
pub const CAMPSITES_PER_PAGE: usize = 6;
const GRID_COLUMNS: usize = 3;

fn card(
    id: u32,
    name: &str,
    preview: Option<CampsitePreview>,
    selected: bool,
    index: usize,
) -> Element {
    let art = preview
        .map(|image| {
            let [l, r, t, b] = image.tex_coords;
            rsx! { texture { name: DynName(format!("CampsiteCard_{id}")), texture_fdid: image.fdid,
            tex_coords: format!("{l},{r},{t},{b}"), width: CARD_WIDTH, height: CARD_HEIGHT,
            pos_type: "absolute", left: 1.0, top: 0.0, } }
        })
        .unwrap_or_default();
    let hide_selected = !selected;
    let label_bar = ui_toolkit::atlas::get_region("campcollection-bg-text")
        .expect("WarbandSceneTemplate name background");
    rsx! {
        button {
            name: DynName(format!("CampsiteScene_{id}")), width: CARD_WIDTH, height: CARD_HEIGHT,
            button_default_skin: false, onclick: CharSelectAction::SelectCampsite(id),
            pos_type: "absolute", left: {(index % GRID_COLUMNS) as f32 * (CARD_WIDTH+11.0)},
            top: {(index / GRID_COLUMNS) as f32 * (CARD_HEIGHT+34.0)},
            {art}
            texture { name: DynName(format!("CampsiteLabelBar_{id}")), texture_atlas: "campcollection-bg-text",
                width: label_bar.width, height: label_bar.height, pos_type: "absolute",
                left: "50%", translate_x: "-50%", bottom: 4.0, margin_left: 1.0, }
            r#frame { name: DynName(format!("CampsiteBorder_{id}")), width: 224.0, height: 183.0,
                pos_type: "absolute", left: -4.0, top: -5.0,
                {crate::game_menu_main::sliced_atlas(&format!("CampsiteBorderArt_{id}"), "campcollection-frame", [224.0,183.0], [12.0;4])}
            }
            r#frame { name: DynName(format!("CampsiteSelected_{id}")), width: 224.0, height: 183.0,
                hidden: hide_selected, pos_type: "absolute", left: -4.0, top: -5.0,
                {crate::game_menu_main::sliced_atlas(&format!("CampsiteSelectedArt_{id}"), "campcollection-frame-selected", [224.0,183.0], [18.0;4])}
            }
            r#frame { name: DynName(format!("CampsiteHover_{id}")), width: 224.0, height: 183.0,
                hidden: true, pos_type: "absolute", left: -4.0, top: -5.0,
                {crate::game_menu_main::sliced_atlas(&format!("CampsiteHoverArt_{id}"), "campcollection-frame-hover", [224.0,183.0], [17.0;4])}
            }
            fontstring { name: DynName(format!("CampsiteLabel_{id}")), width: 200.0, height: 32.0,
                text: name, font: GameFont::FrizQuadrata, font_size: 12.0, font_color: FontColor::new(1.0,1.0,1.0,1.0),
                pos_type: "absolute", left: "50%", translate_x: "-50%", bottom: 4.5, margin_left: 1.0, frame_level: 100.0, }
        }
    }
}
pub fn campsite_page_count(count: usize) -> usize {
    count.div_ceil(CAMPSITES_PER_PAGE).max(1)
}
fn visible_page(state: &CampsiteState) -> usize {
    state.page.min(campsite_page_count(state.scenes.len()) - 1)
}
pub fn campsite_panel_height(_: usize) -> f32 {
    PANEL_HEIGHT
}

pub fn campsite_panel(state: &CampsiteState) -> Element {
    let hide = !state.panel_visible;
    let page = visible_page(state);
    let pages = campsite_page_count(state.scenes.len());
    let page_text = format!("Page {}/{}", page + 1, pages);
    let page_text_width =
        ui_toolkit::text_measure::measure_text(&page_text, GameFont::FrizQuadrata, 12.0)
            .expect("PagingControls PageText font")
            .0;
    let cards: Element = state
        .scenes
        .iter()
        .skip(page * CAMPSITES_PER_PAGE)
        .take(CAMPSITES_PER_PAGE)
        .enumerate()
        .flat_map(|(index, e)| {
            card(
                e.id,
                &e.name,
                e.preview_image,
                state.selected_id == Some(e.id),
                index,
            )
        })
        .collect();
    rsx! {
        r#frame {
            name: "CampsitePanel", width: CAMPSITE_PANEL_WIDTH, height: PANEL_HEIGHT, hidden: hide,
            strata: FrameStrata::Dialog, pos_type: "absolute", left: "50%", top: "50%",
            translate_x: "-50%", translate_y: "-50%", margin_top: CAMPSITE_PANEL_TOP_OFFSET,
            {crate::quest_art::window_chrome("Campsite", (CAMPSITE_PANEL_WIDTH,PANEL_HEIGHT), "Campsites", &CharSelectAction::CampsiteToggle.to_string())}
            r#frame { name: "CampsiteIconsFrame", width: 693.0, height: 482.0, pos_type: "absolute", left: 4.0, top: 60.0,
                texture { name: "CampsiteBackgroundTile", texture_atlas: "collections-background-tile", width: 685.0, height: 474.0,
                    pos_type: "absolute", left: 4.0, top: 4.0, }
                {collection_inset()}
                {collection_shadows()}
                r#frame { name: "CampsiteGrid", width: 668.0, height: 419.0, pos_type: "absolute", left: 13.0, top: 38.0, {cards} }
                r#frame { name: "CampsitePaging", width: {page_text_width + 74.0}, height: 32.0, pos_type: "absolute", left: "50%", top: 436.0, translate_x: "-50%",
                    fontstring { name: "CampsitePageText", width: page_text_width, height: 32.0, text: {&page_text}, font_size: 12.0, }
                    {page_button("CampsitePrevPage", if page>0 {130869} else {130867}, page.saturating_sub(1), page_text_width+5.0)}
                    {page_button("CampsiteNextPage", if page+1<pages {130866} else {130864}, (page+1).min(pages-1), page_text_width+42.0)}
                }
            }
        }
    }
}
fn page_button(name: &str, fdid: u32, page: usize, x: f32) -> Element {
    let disabled = matches!(fdid, 130867 | 130864);
    rsx! { button { name: DynName(name.into()), width: 32.0, height: 32.0, button_default_skin: false, disabled,
        onclick: CharSelectAction::CampsitePage(page), pos_type: "absolute", left: x, top: 0.0,

    } }
}

pub fn sync_campsite_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    let cards: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let id = frame.name.as_deref()?.strip_prefix("CampsiteScene_")?;
            let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
                return None;
            };
            Some((id.to_string(), button.hovered))
        })
        .collect();
    for (id, hovered) in cards {
        if let Some(frame) = registry.get_by_name(&format!("CampsiteHover_{id}")) {
            registry.set_hidden(frame, !hovered);
        }
    }
    for (name, up, down, disabled) in [
        ("CampsitePrevPage", 130869, 130868, 130867),
        ("CampsiteNextPage", 130866, 130865, 130864),
    ] {
        if let Some(id) = registry.get_by_name(name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::Button(button)) = frame.widget_data.as_mut()
        {
            button.normal_texture =
                Some(ui_toolkit::widgets::texture::TextureSource::FileDataId(up));
            button.pushed_texture = Some(ui_toolkit::widgets::texture::TextureSource::FileDataId(
                down,
            ));
            button.disabled_texture = Some(
                ui_toolkit::widgets::texture::TextureSource::FileDataId(disabled),
            );
            button.highlight_texture = Some(
                ui_toolkit::widgets::texture::TextureSource::FileDataId(130757),
            );
        }
    }
    if let Some(id) = registry.get_by_name("CampsiteBackgroundTile")
        && let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
    {
        texture.horiz_tile = true;
        texture.vert_tile = true;
    }
}

fn collection_shadows() -> Element {
    // CollectionsBackgroundTemplate: four flipped corners and four single-edge strips.
    let mut art = Vec::new();
    for (layer, atlas) in [
        ("Large", "collections-background-shadow-large"),
        ("Small", "collections-background-shadow-small"),
    ] {
        let region = ui_toolkit::atlas::get_region(atlas).expect("Collections shadow atlas");
        let (w, h) = (region.width, region.height);
        for (part, x, y, width, height, coords) in [
            ("TL", 4.0, 4.0, w, h, "0,1,0,1"),
            ("TR", 689.0 - w, 4.0, w, h, "1,0,0,1"),
            ("BL", 4.0, 478.0 - h, w, h, "0,1,1,0"),
            ("BR", 689.0 - w, 478.0 - h, w, h, "1,0,1,0"),
            ("Top", 4.0 + w, 4.0, 685.0 - 2.0 * w, h, "0.9999,1,0,1"),
            (
                "Bottom",
                4.0 + w,
                478.0 - h,
                685.0 - 2.0 * w,
                h,
                "0.9999,1,1,0",
            ),
            ("Left", 4.0, 4.0 + h, w, 474.0 - 2.0 * h, "0,1,0.9999,1"),
            (
                "Right",
                689.0 - w,
                4.0 + h,
                w,
                474.0 - 2.0 * h,
                "1,0,0.9999,1",
            ),
        ] {
            art.extend(rsx! { texture { name: DynName(format!("CampsiteShadow{layer}{part}")), texture_atlas: atlas,
                width, height, tex_coords: coords, pos_type: "absolute", left: x, top: y, } });
        }
    }
    art
}

fn collection_inset() -> Element {
    // InsetFrameTemplate:6px corners/3px edges; bottom corners extend1below parent.
    [
        ("TL", "UI-Frame-InnerTopLeft", (0.0, 0.0, 6.0, 6.0)),
        ("TR", "UI-Frame-InnerTopRight", (687.0, 0.0, 6.0, 6.0)),
        ("BL", "UI-Frame-InnerBotLeftCorner", (0.0, 477.0, 6.0, 6.0)),
        ("BR", "UI-Frame-InnerBotRight", (687.0, 477.0, 6.0, 6.0)),
        ("Top", "_UI-Frame-InnerTopTile", (6.0, 0.0, 681.0, 3.0)),
        ("Bottom", "_UI-Frame-InnerBotTile", (6.0, 480.0, 681.0, 3.0)),
        ("Left", "!UI-Frame-InnerLeftTile", (0.0, 6.0, 3.0, 471.0)),
        (
            "Right",
            "!UI-Frame-InnerRightTile",
            (690.0, 6.0, 3.0, 471.0),
        ),
    ]
    .into_iter()
    .flat_map(|(part, atlas, rect)| {
        crate::quest_art::named_atlas_texture(format!("CampsiteInset{part}"), atlas, rect)
    })
    .collect()
}
