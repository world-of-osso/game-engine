//! EncounterInfoTemplate and the Overview paper regions from Retail EJ XML.
use crate::{
    adventure_guide::View,
    adventure_guide_art::{GOLD, SHEET},
    bank_art::{cropped, label, texture},
    quest_art::DynName,
};
use game_engine_core::adventure_guide::Section;
use ui_toolkit::{
    frame::WidgetData, registry::FrameRegistry, rsx, widget_def::Element,
    widgets::button::ButtonState,
};
const TILE: u32 = 522973;

#[cfg(test)]
#[path = "adventure_guide_sections_tests.rs"]
mod tests;

pub fn overview_header(top: f32) -> Element {
    let mut out = cropped(
        "JournalOverviewHeader".into(),
        SHEET,
        "0.359375,0.99609375,0.8525390625,0.880859375",
        (-3.5, top, 327., 30.),
    );
    out.extend(label(
        "JournalOverviewTitle".into(),
        crate::auction_mail::text("OVERVIEW"),
        (4.5, top + 14., 200., 10.),
        (12., "0.929,0.788,0.620,1", "LEFT"),
    ));
    out
}

/// EncounterJournal_SetBullets: authored description, then 13px paper bullets +4px text gap.
pub fn append_overview_body(content: &mut Element, y: &mut f32, name: &str, body: &str) {
    let mut parts = body.split("$bullet;");
    let description = parts.next().unwrap().trim();
    if !description.is_empty() {
        let height = crate::adventure_guide::paragraph_height(description, 45.);
        content.extend(crate::adventure_guide::paper_text(
            name,
            description,
            (0., *y, 315., height),
        ));
        *y += height + 9.;
    }
    for (index, part) in parts.enumerate() {
        let value = part.trim();
        let height = crate::adventure_guide::paragraph_height(value, 40.);
        content.extend(cropped(
            format!("{name}Bullet{index}"),
            SHEET,
            "0.974609375,1,0.7509765625,0.763671875",
            (13., *y, 13., 13.),
        ));
        content.extend(crate::adventure_guide::paper_text(
            &format!("{name}Bullet{index}Text"),
            value,
            (30., *y, 289., height),
        ));
        *y += height;
    }
}

pub fn section_header(
    section: &Section,
    title: &str,
    rect: (f32, f32, f32, f32),
    collapsed: bool,
) -> Element {
    let name = format!("JournalSection{}", section.id);
    let (left, top, width, height) = rect;
    let (flags, title_right) =
        crate::adventure_section_flags::header_flags(section, (width, height));
    let mut contents = paper_regions(&name, width, collapsed, false);
    contents.extend(label(
        format!("{name}ExpandedIcon"),
        if collapsed { "+" } else { "−" },
        (5., 6., 12., 12.),
        (16., GOLD, "CENTER"),
    ));
    if section.icon != 0 {
        contents.extend(texture(
            format!("{name}Icon"),
            section.icon,
            (22., 3., 18., 18.),
            "1,1,1,1",
        ));
    }
    contents.extend(flags);
    contents.extend(label(
        format!("{name}Name"),
        title,
        (45., 8., title_right - 45., 10.),
        (12., GOLD, "LEFT"),
    ));
    let glow_name = format!("{name}Glow");
    let glow = paper_highlight(&glow_name, width, true);
    contents.extend(rsx! {r#frame {name:{DynName(glow_name)},left:0.0,top:0.0,width,height,pos_type:"absolute",alpha:0.0,{glow}}});
    let action = format!("journal:section:{}", section.id);
    rsx! {button {name:{DynName(name)},left,top,width,height,pos_type:"absolute",onclick:{action.as_str()},button_default_skin:false,{contents}}}
}

fn paper_coords(collapsed: bool, pushed: bool) -> [&'static str; 3] {
    match (collapsed, pushed) {
        (true, false) => [
            "0.84960938,0.97460938,0.49023438,0.51855469",
            "0,1,0.34375000,0.40039063",
            "0.72656250,0.85156250,0.52441406,0.55273438",
        ],
        (true, true) => [
            "0.47460938,0.59960938,0.49316406,0.52148438",
            "0,1,0.28320313,0.33984375",
            "0.60351563,0.72851563,0.49316406,0.52148438",
        ],
        (false, false) => [
            "0.81445313,0.93945313,0.39453125,0.42285156",
            "0,1,0.22265625,0.27929688",
            "0.34570313,0.47070313,0.49316406,0.52148438",
        ],
        (false, true) => [
            "0.64062500,0.76562500,0.21386719,0.24218750",
            "0,1,0.40429688,0.46093750",
            "0.76953125,0.89453125,0.21386719,0.24218750",
        ],
    }
}
fn paper_regions(name: &str, width: f32, collapsed: bool, pushed: bool) -> Element {
    let coords = paper_coords(collapsed, pushed);
    let mut out = Vec::new();
    for (suffix, fdid, uv, rect) in [
        ("PaperMid", TILE, coords[1], (31., -1.5, width - 60., 29.)),
        ("PaperLeft", SHEET, coords[0], (-1., -1.5, 64., 29.)),
        (
            "PaperRight",
            SHEET,
            coords[2],
            (width - 61., -1.5, 64., 29.),
        ),
    ] {
        out.extend(cropped(format!("{name}{suffix}"), fdid, uv, rect));
    }
    out.extend(paper_highlight(name, width, false));
    out
}

fn paper_highlight(name: &str, width: f32, flash: bool) -> Element {
    let layer = if flash { "OVERLAY" } else { "HIGHLIGHT" };
    let blend = if flash { "BLEND" } else { "ADD" };
    let mut out = Vec::new();
    for (suffix, fdid, uv, rect) in [
        (
            "HighlightMid",
            TILE,
            "0,1,0.46484375,0.52148438",
            (31., -1.5, width - 60., 29.),
        ),
        (
            "HighlightLeft",
            SHEET,
            "0.74218750,0.86718750,0.15820313,0.18652344",
            (-1., -1.5, 64., 29.),
        ),
        (
            "HighlightRight",
            SHEET,
            "0.87109375,0.99609375,0.15820313,0.18652344",
            (width - 61., -1.5, 64., 29.),
        ),
    ] {
        let (left, top, width, height) = rect;
        out.extend(rsx! {texture {name:{DynName(format!("{name}{suffix}"))},texture_fdid:fdid,tex_coords:uv,left,top,width,height,pos_type:"absolute",draw_layer:{layer},blend_mode:{blend}}});
    }
    out
}

pub fn sync_header_art(view: &View, registry: &mut FrameRegistry) {
    for section in view.catalog.sections.values() {
        let name = format!("JournalSection{}", section.id);
        let Some(id) = registry.get_by_name(&name) else {
            continue;
        };
        let pushed = matches!(&registry.get(id).unwrap().widget_data,Some(WidgetData::Button(button)) if button.state==ButtonState::Pushed);
        let coords = paper_coords(view.collapsed.contains(&section.id), pushed);
        for (suffix, uv) in ["PaperLeft", "PaperMid", "PaperRight"]
            .into_iter()
            .zip(coords)
        {
            let Some(id) = registry.get_by_name(&format!("{name}{suffix}")) else {
                continue;
            };
            if let Some(WidgetData::Texture(texture)) =
                &mut registry.get_mut(id).unwrap().widget_data
            {
                let mut numbers = uv.split(',');
                texture.tex_coords =
                    std::array::from_fn(|_| numbers.next().unwrap().parse().unwrap());
            }
        }
    }
}
