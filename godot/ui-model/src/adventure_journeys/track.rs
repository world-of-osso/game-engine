//! Blizzard_FrameXML/RewardTrackTemplates.xml/.lua ordinary renown track.
use crate::{
    bank_art::{WHITE, label, texture},
    quest_art::{DynName, NORMAL_FONT_COLOR, named_atlas_texture},
};
use shared::protocol::MajorFactionSnapshot;
use ui_toolkit::{rsx, widget_def::Element};

pub(super) fn track(faction: &MajorFactionSnapshot, selected: u32) -> Element {
    let levels = (1..=faction.max_level)
        .flat_map(|level| level_button(faction, selected, level))
        .collect::<Element>();
    let arrows = arrows(selected, faction.max_level);
    rsx! {r#frame {
        name:"JourneyRenownTrack",left:{109.5},top:90.0,width:570.0,height:72.0,pos_type:"absolute",
        r#frame {name:"JourneyRenownTrackClip",left:0.0,top:0.0,width:570.0,height:72.0,pos_type:"absolute",{levels}}
        {arrows}
    }}
}

fn level_button(faction: &MajorFactionSnapshot, selected: u32, level: u32) -> Element {
    let prefix = format!("JourneyRenownLevel{level}");
    let left = 257.5 + (level as f32 - selected as f32) * 53.;
    let contents = level_art(faction, selected, level, &prefix);
    let action = format!("journal:journey-select-level:{level}");
    rsx! {button {
        name:{DynName(prefix)},left,top:{8.5},width:55.0,height:55.0,pos_type:"absolute",
        button_default_skin:false,onclick:{action.as_str()},{contents}
    }}
}

fn level_art(faction: &MajorFactionSnapshot, selected: u32, level: u32, prefix: &str) -> Element {
    let earned = level <= faction.level;
    let suffix = if level == selected {
        "-yellow"
    } else if earned {
        ""
    } else {
        "-grey"
    };
    let reward = faction.rewards.iter().find(|reward| reward.level == level);
    let shape = reward.map_or("square", |reward| {
        if reward.item_id > 0 || reward.mount_id > 0 || reward.title_id > 0 {
            "ring"
        } else {
            "square"
        }
    });
    let mut contents = Vec::new();
    if let Some(reward) = reward {
        contents.extend(texture(
            format!("{prefix}Icon"),
            reward.icon,
            (6.5, 6.5, 42., 42.),
            WHITE,
        ));
    }
    contents.extend(named_atlas_texture(
        format!("{prefix}Border"),
        &format!("ui-journeys-delve-rewardicon-{shape}-frame{suffix}"),
        (0., 0., 55., 55.),
    ));
    contents.extend(named_atlas_texture(
        format!("{prefix}Rectangle"),
        &format!("ui-journeys-delve-rewardicon-rectangle-frame{suffix}"),
        (10.5, 37., 34., 27.),
    ));
    let color = if earned {
        NORMAL_FONT_COLOR
    } else {
        "0.5,0.5,0.5,1"
    };
    contents.extend(label(
        format!("{prefix}Level"),
        &level.to_string(),
        (10.5, 41., 34., 20.),
        (12., color, "CENTER"),
    ));
    if earned {
        contents.extend(named_atlas_texture(
            format!("{prefix}Earned"),
            "activities-icon-checkmark-small",
            (31., 31., 19., 19.),
        ));
    }
    contents
}

fn arrows(selected: u32, max: u32) -> Element {
    let mut contents = arrow(
        "JourneyLevelPrev",
        "prev",
        (-26., 19., 22., 34.),
        selected <= 1,
        false,
    );
    contents.extend(arrow(
        "JourneyLevelNext",
        "next",
        (574., 19., 22., 34.),
        selected >= max,
        false,
    ));
    contents.extend(arrow(
        "JourneyLevelFirst",
        "first",
        (-65., 25., 22., 22.),
        selected <= 1,
        true,
    ));
    contents.extend(arrow(
        "JourneyLevelLast",
        "last",
        (613., 25., 22., 22.),
        selected >= max,
        true,
    ));
    contents
}

fn arrow(
    name: &str,
    step: &str,
    rect: (f32, f32, f32, f32),
    disabled: bool,
    small: bool,
) -> Element {
    let atlas = arrow_atlas(small, if disabled { "-disable" } else { "" });
    let parts = named_atlas_texture(
        format!("{name}Art"),
        &atlas,
        (-1.5, (rect.3 - 40.) / 2., 25., 40.),
    );
    let (left, top, width, height) = rect;
    let name = DynName(name.into());
    let action = format!("journal:journey-level:{step}");
    if disabled {
        return rsx! {button {name,left,top,width,height,pos_type:"absolute",button_default_skin:false,disabled:true,{parts}}};
    }
    rsx! {button {name,left,top,width,height,pos_type:"absolute",button_default_skin:false,onclick:{action.as_str()},{parts}}}
}

fn arrow_atlas(small: bool, suffix: &str) -> String {
    format!(
        "ui-journeys-delve-arrow-{}-left{suffix}",
        if small { "small" } else { "big" }
    )
}

pub(super) fn sync_arrow_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{
        atlas::{AtlasSource, resolve_region, thread_skin},
        frame::WidgetData,
        widgets::{button::ButtonState, texture::TextureSource},
    };
    for (name, small, right) in [
        ("JourneyLevelPrev", false, false),
        ("JourneyLevelNext", false, true),
        ("JourneyLevelFirst", true, false),
        ("JourneyLevelLast", true, true),
    ] {
        let Some(id) = registry.get_by_name(name) else {
            continue;
        };
        let Some(WidgetData::Button(button)) = &registry.get(id).unwrap().widget_data else {
            continue;
        };
        let suffix = match button.state {
            ButtonState::Disabled => "-disable",
            ButtonState::Pushed => "-pressed",
            ButtonState::Normal => "",
        };
        let atlas = arrow_atlas(small, suffix);
        let region = resolve_region(&atlas, thread_skin()).expect("renown track arrow atlas");
        let AtlasSource::FileDataId(fdid) = region.source else {
            panic!("renown arrow must use authored FDID")
        };
        let art = registry
            .get_by_name(&format!("{name}Art"))
            .expect("renown track arrow texture");
        if let Some(WidgetData::Texture(texture)) = &mut registry.get_mut(art).unwrap().widget_data
        {
            texture.source = TextureSource::FileDataId(fdid);
            texture.tex_coords = if right {
                [region.right, region.left, region.top, region.bottom]
            } else {
                [region.left, region.right, region.top, region.bottom]
            };
        }
    }
}
