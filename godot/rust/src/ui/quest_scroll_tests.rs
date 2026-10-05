//! QuestFrame's detail page scrolls a quest taller than its parchment
//! (`QuestDetailScrollFrame`, `QuestFrame.xml:211-217`; docs/specs/quest-ui.md).

use game_engine_ui_model::panel_style_data::{MetalGeometry, MetalTopLeft, metal_frame_style};
use game_engine_ui_model::quest_frame_component::{
    QUEST_DETAIL_SCROLL, QuestFramePage, QuestFrameState, RewardItemView, RewardView,
    SCROLL_PAN_EXTENT, quest_frame_screen,
};
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::scroll_list::{thumb_name, track_name};
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

use super::tests::{rebuild, rect, shown};
use super::*;
use crate::ui::ui_parent::UiParent;
use crate::ui::{RegistryModel, ScreenPostsetup};

/// "Beating Them Back!" as Marshal McBride gives it, with two item rewards and money.
fn mcbride_detail(title: &str) -> QuestFrameState {
    let description = "So you're the new recruit from Stormwind, eh? I'm Marshal McBride, \
        commander of this garrison. Glad to have you on board...\n\n<McBride looks through \
        some papers.>\n\nShot. It is Shot, right?\n\nYou've arrived just in time. The \
        Blackrock orcs have managed to sneak into Northshire through a break in the mountain. \
        My soldiers are doing the best that they can to push them back, but I fear they will \
        be overwhelmed soon.\n\nHead northwest into the forest and kill the attacking \
        Blackrock worgs! Help my soldiers!";
    let item = |name: &str| RewardItemView {
        name: name.into(),
        count: 1,
        icon_fdid: None,
    };
    QuestFrameState {
        visible: true,
        npc_name: "Marshal McBride".into(),
        page: QuestFramePage::Detail {
            title: title.into(),
            description: description.into(),
            objectives_text: "Kill 6 Blackrock Worgs.".into(),
            rewards: RewardView {
                money: 55,
                items: vec![item("Recruit's Boots"), item("Recruit's Gloves")],
                choices: Vec::new(),
                selected_choice: None,
            },
        },
    }
}

fn short_detail() -> QuestFrameState {
    QuestFrameState {
        visible: true,
        npc_name: "Marshal McBride".into(),
        page: QuestFramePage::Detail {
            title: "A Threat Within".into(),
            description: "Defend Northshire.".into(),
            objectives_text: "Speak to McBride.".into(),
            rewards: RewardView::default(),
        },
    }
}

fn quest_frame(state: QuestFrameState) -> RegistryModel {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut registry = UiParent::for_viewport(1920.0, 1080.0).registry();
    registry.register_panel_style(
        MetalTopLeft::Portrait.style_name(),
        metal_frame_style(
            TextureSource::Dynamic(DynamicTextureId(1)),
            MetalGeometry::active().unwrap(),
        ),
    );
    let mut model = RegistryModel {
        screen: Screen::new(quest_frame_screen),
        shared,
        registry,
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    rebuild(&mut model);
    model
}

/// The reward frames the detail page draws.
const REWARDS: [&str; 4] = [
    "QuestInfoRewardsFrameHeader",
    "QuestInfoRewardsFrameQuestInfoItem1IconTexture",
    "QuestInfoRewardsFrameQuestInfoItem2IconTexture",
    "QuestInfoMoneyText",
];

fn inside(inner: &LayoutRect, outer: &LayoutRect) -> bool {
    inner.y >= outer.y - 0.01 && inner.y + inner.height <= outer.y + outer.height + 0.01
}

fn top(model: &RegistryModel, name: &str) -> f32 {
    rect(model, name).unwrap().y
}

fn wheel_down_to_end(model: &mut RegistryModel) -> usize {
    let mut notches = 0;
    while wheel(model, QUEST_DETAIL_SCROLL, false) {
        rebuild(model);
        notches += 1;
        assert!(notches < 100, "the wheel never stops");
    }
    notches
}

#[test]
fn a_long_quest_scrolls_its_rewards_into_the_parchment() {
    let mut model = quest_frame(mcbride_detail("Beating Them Back!"));
    let parchment = rect(&model, "QuestFrameParchment").unwrap();
    let area = rect(&model, QUEST_DETAIL_SCROLL).unwrap();
    let money = rect(&model, "QuestInfoMoneyText").unwrap();
    assert!(
        !inside(&money, &area),
        "the money line starts below the frame"
    );
    assert!(shown(&model, &thumb_name(QUEST_DETAIL_SCROLL)));
    assert!(
        !wheel(&mut model, QUEST_DETAIL_SCROLL, true),
        "already at the top"
    );

    let title = top(&model, "QuestInfoTitleHeader");
    assert!(wheel(&mut model, QUEST_DETAIL_SCROLL, false));
    rebuild(&mut model);
    let step = title - top(&model, "QuestInfoTitleHeader");
    assert!((step - SCROLL_PAN_EXTENT as f32).abs() < 0.01, "{step}");

    wheel_down_to_end(&mut model);
    for name in REWARDS {
        let reward = rect(&model, name).unwrap();
        assert!(inside(&reward, &area), "{name} {reward:?} inside {area:?}");
        assert!(inside(&reward, &parchment), "{name} inside the parchment");
    }
    let title = rect(&model, "QuestInfoTitleHeader").unwrap();
    assert!(title.y + title.height <= area.y, "the title scrolled out");
}

#[test]
fn the_forward_stepper_scrolls_the_details_and_a_new_quest_starts_at_its_top() {
    let mut model = quest_frame(mcbride_detail("Beating Them Back!"));
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(QUEST_DETAIL_SCROLL))
        .unwrap();
    let title = top(&model, "QuestInfoTitleHeader");
    assert!(press_stepper(&mut model, forward));
    rebuild(&mut model);
    let step = title - top(&model, "QuestInfoTitleHeader");
    assert!((step - SCROLL_PAN_EXTENT as f32).abs() < 0.01, "{step}");

    let next = mcbride_detail("Lions for Lambs");
    model.reset_quest_scroll_for(&next);
    model.shared.insert(next);
    rebuild(&mut model);
    assert_eq!(top(&model, "QuestInfoTitleHeader"), title);
}

/// A quest that fits keeps the bar's track without a thumb, since `QuestScrollFrameTemplate`
/// leaves `scrollBarHideIfUnscrollable` unset (`ScrollBar.lua:252-264`), and does not scroll.
#[test]
fn a_short_quest_shows_the_track_without_a_thumb_and_does_not_scroll() {
    let mut model = quest_frame(short_detail());
    assert!(shown(&model, &track_name(QUEST_DETAIL_SCROLL)));
    assert!(!shown(&model, &thumb_name(QUEST_DETAIL_SCROLL)));
    let title = top(&model, "QuestInfoTitleHeader");
    assert!(!wheel(&mut model, QUEST_DETAIL_SCROLL, false));
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(QUEST_DETAIL_SCROLL))
        .unwrap();
    assert!(press_stepper(&mut model, forward));
    rebuild(&mut model);
    assert_eq!(top(&model, "QuestInfoTitleHeader"), title);
}

/// The QuestFrame's texture list, which the client copies out of local CASC before the window
/// shows, holds the files the Modern MinimalScrollBar atlases resolve to: the bar's track,
/// thumb and arrows load like the window's FileDataID art.
#[test]
fn a_modern_scroll_bar_atlas_resolves_to_a_cached_texture() {
    use ui_toolkit::atlas::{ActiveSkin, AtlasSource, active_skin, resolve_region};
    let model = quest_frame(mcbride_detail("Beating Them Back!"));
    assert_eq!(active_skin(), ActiveSkin::Modern);
    let cached = crate::quests::screen_texture_fdids(
        mcbride_detail("Beating Them Back!"),
        quest_frame_screen,
    );
    let bar = model
        .registry
        .get_by_name(&format!("{QUEST_DETAIL_SCROLL}ScrollBar"))
        .unwrap();
    let mut atlases = Vec::new();
    let mut stack = vec![bar];
    while let Some(id) = stack.pop() {
        let frame = model.registry.get(id).unwrap();
        stack.extend(frame.children.iter().copied());
        if let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = &frame.widget_data
            && let TextureSource::Atlas(name) = &texture.source
        {
            atlases.push(name.clone());
        }
    }
    assert_eq!(atlases.len(), 8, "arrows, track and thumb art: {atlases:?}");
    for name in atlases {
        let region = resolve_region(&name, ActiveSkin::Modern).unwrap();
        let AtlasSource::FileDataId(fdid) = region.source else {
            panic!("{name} is DB2 art");
        };
        assert!(cached.contains(&fdid), "{name} file {fdid} is cached");
    }
}
