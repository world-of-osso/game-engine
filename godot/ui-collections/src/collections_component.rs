//! Retail Mainline CollectionsJournal: one portrait shell and shared tab bar.
use game_engine_ui_model::merchant_frame_component::tab_width;
use game_engine_ui_model::quest_art::{
    DynName, named_atlas_texture, portrait_border, window_portrait_slot, window_portrait_texture,
};
use game_engine_ui_model::ui::strata::FrameStrata;
use ui_toolkit::{rsx, widget_def::Element};

pub const SIZE: [f32; 2] = [703.0, 606.0];
pub const TABS: [&str; 6] = [
    "Mounts",
    "Pets",
    "Toy Box",
    "Heirlooms",
    "Appearances",
    "Warband Scenes",
];

pub fn collections_shell(viewport: [f32; 2], selected: usize, body: Element) -> Element {
    let title = if selected == 1 {
        "Pet Journal"
    } else {
        TABS[selected]
    };
    let mut chrome = portrait_background();
    chrome.extend(portrait_border(
        "CollectionsJournal",
        (SIZE[0], SIZE[1]),
        title,
        "collections:close",
    ));
    let portrait =
        window_portrait_texture(&window_portrait_slot("CollectionsJournalPortrait"), 454_046);
    let tabs = tabs(selected);
    let left = (viewport[0] - SIZE[0]) / 2.0;
    let top = (viewport[1] - SIZE[1]) / 2.0;
    rsx! {
        r#frame { name: "CollectionsRoot", stretch: true,
            r#frame { name: "CollectionsJournal", width: 703.0, height: 606.0,
                mouse_enabled: true, pos_type: "absolute", left, top, strata: FrameStrata::Dialog,
                {chrome} {portrait} {body} {tabs}
            }
        }
    }
}

// PortraitFrameTexturedBaseTemplate, SharedUIPanelTemplates.xml:597-614.
fn portrait_background() -> Element {
    let streaks = named_atlas_texture(
        "CollectionsJournalTopTileStreaks".into(),
        "_UI-Frame-TopTileStreaks",
        (6.0, 21.0, SIZE[0] - 8.0, 43.0),
    );
    rsx! {
        texture { name: "CollectionsJournalBg", texture_fdid: 374_155,
            width: {SIZE[0] - 4.0}, height: {SIZE[1] - 23.0},
            pos_type: "absolute", left: 2.0, top: 21.0 }
        {streaks}
    }
}

/// RSX does not expose Retail Texture's horizTile/vertTile attributes.
pub fn apply_collections_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    // These source tabs remain unavailable; keep their disabled label state while
    // the shared PanelTab renderer owns all art and geometry.
    for index in 4..=6 {
        if let Some(id) = registry.get_by_name(&format!("CollectionsJournalTab{index}Text")) {
            let frame = registry.get_mut(id).expect("collections tab label");
            let Some(ui_toolkit::frame::WidgetData::FontString(label)) = &mut frame.widget_data
            else {
                panic!("collections tab label must be text");
            };
            label.color = [0.5, 0.5, 0.5, 1.0];
        }
    }
    align_pet_loadout_names(registry);
    for (name, horizontal, vertical) in [
        ("CollectionsJournalBg", true, true),
        ("CollectionsJournalTopTileStreaks", true, false),
        ("PetJournalSpellSelectBgTiled", false, true),
    ] {
        if let Some(id) = registry.get_by_name(name) {
            let frame = registry.get_mut(id).expect("collections texture");
            let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = &mut frame.widget_data
            else {
                panic!("{name} must be a texture");
            };
            texture.horiz_tile = horizontal;
            texture.vert_tile = vertical;
        }
    }
}

// CompanionLoadOutTemplate names use BOTTOM; retire when RSX supports justify_v.
fn align_pet_loadout_names(registry: &mut ui_toolkit::registry::FrameRegistry) {
    for slot in 0..3 {
        let name = format!("PetJournalBattleSlot{slot}Name");
        let Some(id) = registry.get_by_name(&name) else {
            continue;
        };
        let frame = registry.get_mut(id).expect("loadout name");
        let Some(ui_toolkit::frame::WidgetData::FontString(text)) = &mut frame.widget_data else {
            panic!("loadout name must be text");
        };
        text.justify_v = ui_toolkit::widgets::font_string::JustifyV::Bottom;
    }
}

/// SharedCollectionTemplates.xml:68-202, two source-sized shadow rings and corners.
pub fn collection_background_shadows(prefix: &str, rect: (f32, f32, f32, f32)) -> Element {
    let mut out = Vec::new();
    for (suffix, atlas, edges) in [
        ("Shadow", "collections-background-shadow-large", true),
        ("OverlayShadow", "collections-background-shadow-small", true),
        ("BGCorner", "collections-background-corner", false),
    ] {
        out.extend(atlas_corner_ring(prefix, suffix, atlas, rect, edges));
    }
    out
}

fn atlas_corner_ring(
    prefix: &str,
    suffix: &str,
    atlas: &str,
    (x, y, width, height): (f32, f32, f32, f32),
    edges: bool,
) -> Element {
    use ui_toolkit::atlas::{resolve_region, thread_skin};
    let region = resolve_region(atlas, thread_skin()).expect("collection shadow atlas");
    let w = region.width;
    let h = region.height;
    let mut pieces = vec![
        ("TopLeft", (x, y, w, h), [0.0, 1.0, 0.0, 1.0]),
        ("TopRight", (x + width - w, y, w, h), [1.0, 0.0, 0.0, 1.0]),
        (
            "BottomLeft",
            (x, y + height - h, w, h),
            [0.0, 1.0, 1.0, 0.0],
        ),
        (
            "BottomRight",
            (x + width - w, y + height - h, w, h),
            [1.0, 0.0, 1.0, 0.0],
        ),
    ];
    if edges {
        pieces.extend([
            (
                "Top",
                (x + w, y, width - 2.0 * w, h),
                [0.9999, 1.0, 0.0, 1.0],
            ),
            (
                "Bottom",
                (x + w, y + height - h, width - 2.0 * w, h),
                [0.9999, 1.0, 1.0, 0.0],
            ),
            (
                "Left",
                (x, y + h, w, height - 2.0 * h),
                [0.0, 1.0, 0.9999, 1.0],
            ),
            (
                "Right",
                (x + width - w, y + h, w, height - 2.0 * h),
                [1.0, 0.0, 0.9999, 1.0],
            ),
        ]);
    }
    pieces
        .into_iter()
        .flat_map(|(part, (left, top, width, height), [l, r, t, b])| {
            let uv = format!(
                "{},{},{},{}",
                region.left + (region.right - region.left) * l,
                region.left + (region.right - region.left) * r,
                region.top + (region.bottom - region.top) * t,
                region.top + (region.bottom - region.top) * b
            );
            let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
                panic!("collection shadow requires FDID");
            };
            rsx! { texture { name: {DynName(format!("{prefix}{suffix}{part}"))},
            texture_fdid: fdid, tex_coords: {uv.as_str()}, width, height,
            left, top, pos_type: "absolute" } }
        })
        .collect()
}

fn tabs(selected: usize) -> Element {
    let mut left = 11.0;
    let mut tabs: Vec<_> = TABS
        .into_iter()
        .enumerate()
        .map(|(index, title)| {
            let width = tab_width(title);
            let tab = journal_tab(index, selected, title, left, width);
            // Retail AnchorTabs uses a +3px gap after SetNumTabs.
            left += width + 3.0;
            tab
        })
        .collect();
    let active = tabs.remove(selected);
    tabs.into_iter().flatten().chain(active).collect()
}

fn journal_tab(index: usize, active: usize, title: &str, left: f32, width: f32) -> Element {
    let action = match index {
        0 => "collections:mounts",
        1 => "collections:pets",
        2 => "collections:toys",
        _ => "",
    };
    game_engine_ui_model::merchant_frame_component::tab(
        &format!("CollectionsJournalTab{}", index + 1),
        title,
        (left, SIZE[1] - 2.0, width),
        index == active,
        action,
    )
}
