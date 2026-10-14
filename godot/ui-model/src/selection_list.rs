//! Retail selectable list rows, separate from UIPanelButton action chrome.
//!
//! Every row shares one hit target, keyboard focus and Enter/Space activation. Its art
//! comes from the Retail template the list copies: a [`RowTemplate`] names that
//! template's normal, HIGHLIGHT-layer hover and selected textures with their source
//! anchors and blend. Atlas names resolve through the active skin, so Forever draws its
//! authored member for the same name and otherwise the Retail member; file art is the
//! Retail file in both skins.
use crate::bank_art::label;
use crate::quest_art::{DynName, NORMAL_FONT_COLOR};
use ui_toolkit::{
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    rsx,
    strata::DrawLayer,
    widget_def::Element,
    widgets::texture::{TextureData, TextureSource},
};

/// Source texture of one row region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Art {
    Atlas(&'static str),
    /// FileDataID with normalized `left,right,top,bottom` TexCoords.
    File(u32, [f32; 4]),
}

/// One template texture: source, anchor offsets inward from the row edges
/// (`left, top, right, bottom`; negative extends past the row), alpha and blend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowArt {
    pub art: Art,
    pub insets: [f32; 4],
    pub alpha: f32,
    pub additive: bool,
}

/// Normal, hover (HIGHLIGHT layer) and persistent selected art of one Retail template.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowTemplate {
    pub normal: Option<RowArt>,
    pub hover: Option<RowArt>,
    pub selected: Option<RowArt>,
}

const FULL: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
const FULL_CROP: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

const fn fill(art: Art, additive: bool) -> RowArt {
    RowArt {
        art,
        insets: FULL,
        alpha: 1.0,
        additive,
    }
}

/// `Interface\PVPFrame\PvPMegaQueue`.
const PVP_MEGA_QUEUE: u32 = 798_063;
/// `Interface\Common\bluemenu-main`.
const BLUE_MENU: u32 = 593_918;
/// `Interface\QuestFrame\UI-QuestTitleHighlight`.
const QUEST_TITLE_HIGHLIGHT: RowArt = fill(Art::File(136_810, FULL_CROP), true);

/// LFGList.xml:129-138 `LFGListSearchAutoCompleteButtonTemplate`: `search-highlight`
/// HighlightTexture (ADD) and OVERLAY `search-select` (BLEND). Idle rows stay flat on the
/// panel instead of its `_search-rowbg` NormalTexture: the user's selection-list design.
pub const SEARCH: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::Atlas("search-highlight"), true)),
    selected: Some(fill(Art::Atlas("search-select"), false)),
};

/// Blizzard_PVPUI.xml:193-256 `PVPInstanceListEntryButtonTemplate` (specific
/// battlegrounds, :292-297): PvPMegaQueue selected BORDER and HighlightTexture crops,
/// both ADD, TOPLEFT (2,0) / BOTTOMRIGHT (-2,1).
pub const PVP_INSTANCE: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(RowArt {
        art: Art::File(
            PVP_MEGA_QUEUE,
            [0.001_953_13, 0.638_671_88, 0.707_031_25, 0.767_578_13],
        ),
        insets: [2.0, 0.0, 2.0, 1.0],
        alpha: 1.0,
        additive: true,
    }),
    selected: Some(RowArt {
        art: Art::File(
            PVP_MEGA_QUEUE,
            [0.001_953_13, 0.638_671_88, 0.769_531_25, 0.830_078_13],
        ),
        insets: [2.0, 0.0, 2.0, 1.0],
        alpha: 1.0,
        additive: true,
    }),
};

/// Blizzard_PVPUI.xml:591-637 `PVPQueueFrameButtonTemplate`: 224x80 bluemenu-main
/// Background centred on the 203x60 button, HighlightTexture of the same crop ADD at 0.8;
/// PVPQueueFrame_SelectButton (Blizzard_PVPUI.lua:564-574) swaps the Background to the
/// selected crop. The client's category buttons are 168x50, so the source overhang
/// (10.5, 10) scales to (8.69, 8.33).
pub const PVP_CATEGORY: RowTemplate = RowTemplate {
    normal: Some(PVP_CATEGORY_BACKGROUND),
    hover: Some(RowArt {
        alpha: 0.8,
        additive: true,
        ..PVP_CATEGORY_BACKGROUND
    }),
    selected: Some(RowArt {
        art: Art::File(
            BLUE_MENU,
            [0.003_906_25, 0.878_906_25, 0.591_796_88, 0.669_921_88],
        ),
        ..PVP_CATEGORY_BACKGROUND
    }),
};
const PVP_CATEGORY_BACKGROUND: RowArt = RowArt {
    art: Art::File(
        BLUE_MENU,
        [0.003_906_25, 0.878_906_25, 0.751_953_13, 0.830_078_13],
    ),
    insets: [-8.69, -8.33, -8.69, -8.33],
    alpha: 1.0,
    additive: false,
};

/// MenuVariants.lua:41-49 `CreateHighlight`: whole-element UI-QuestTitleHighlight, ADD.
/// Selection is the radio/checkbox tick ([`menu_choice`]), not a row texture.
pub const MENU: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(QUEST_TITLE_HIGHLIGHT),
    selected: None,
};

/// GossipFrame.xml:90-102 `GossipTitleButtonArtTemplate` and QuestFrameTemplates.xml:132
/// `QuestTitleButtonTemplate`: UI-QuestTitleHighlight ADD HighlightTexture.
pub const QUEST_TITLE: RowTemplate = MENU;

/// RaidFrame.xml:39-80 `RaidInfoInstanceTemplate`: the same HighlightTexture, locked on the
/// selected instance (Mainline/RaidFrame.lua:173 LockHighlight).
pub const RAID_INSTANCE: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(QUEST_TITLE_HIGHLIGHT),
    selected: Some(QUEST_TITLE_HIGHLIGHT),
};

/// Blizzard_ProfessionsCustomerOrdersRecipeList.xml:9 and
/// Blizzard_ProfessionsCrafterOrderPage.xml:36: `auctionhouse-ui-row-highlight` ADD over
/// the whole element, shown on enter.
pub const AUCTION_ROW: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::Atlas("auctionhouse-ui-row-highlight"), true)),
    selected: None,
};

/// Mainline/Blizzard_AuctionHouseCategoriesList.xml:21: `auctionhouse-nav-button-highlight`
/// shown on enter (BLEND).
pub const AUCTION_CATEGORY: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::Atlas("auctionhouse-nav-button-highlight"), false)),
    selected: None,
};

/// Blizzard_TransmogTemplates.xml:121-132: `transmog-outfit-card` HighlightTexture ADD over
/// the 208x43 card; the selected card stays in the outfit row content.
pub const TRANSMOG_OUTFIT: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::Atlas("transmog-outfit-card"), true)),
    selected: None,
};

/// CommunitiesTabs.xml:45-46: source square hover and checked textures, both ADD.
pub const COMMUNITIES_TAB: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::File(130718, FULL_CROP), true)),
    selected: Some(fill(Art::File(130724, FULL_CROP), true)),
};

/// SharedUIPanelTemplates.xml:1217-1223 `ColumnDisplayButtonTemplate` ADD highlight.
pub const COLUMN_HEADER: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(RowArt {
        art: Art::File(136580, FULL_CROP),
        insets: [0.0, -5.0, 0.0, -7.0],
        alpha: 1.0,
        additive: true,
    }),
    selected: None,
};

/// CommunitiesMemberList.xml:238-241 `CommunitiesMemberListEntryTemplate`: GuildFrame
/// NormalTexture crop backs every member line; UI-FriendsFrame-HighlightBar ADD on hover.
pub const GUILD_MEMBER: RowTemplate = RowTemplate {
    normal: Some(fill(
        Art::File(
            410_251,
            [0.362_304_69, 0.381_835_94, 0.958_984_38, 0.998_046_88],
        ),
        false,
    )),
    hover: Some(fill(Art::File(131_128, FULL_CROP), true)),
    selected: None,
};

/// Blizzard_HousingHouseFinder.xml:4-115 `HouseFinderNeighborhoodButtonTemplate`.
pub const NEIGHBORHOOD: RowTemplate = RowTemplate {
    normal: Some(fill(
        Art::Atlas("housefinder_neighborhood-list-item-default"),
        false,
    )),
    hover: Some(fill(
        Art::Atlas("housefinder_neighborhood-list-item-highlight"),
        true,
    )),
    selected: None,
};

/// Blizzard_MountCollection.xml:80 `MountListButtonTemplate`: the 208x46 list button sits
/// right of its 42-wide icon button; `PetList-ButtonHighlight` HighlightTexture.
pub const MOUNT: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(RowArt {
        insets: [42.0, 0.0, 0.0, 0.0],
        ..fill(Art::Atlas("PetList-ButtonHighlight"), true)
    }),
    selected: None,
};

/// Shared/Blizzard_PetCollection.xml:7 `CompanionListButtonTemplate`.
pub const PET: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::Atlas("PetList-ButtonHighlight"), true)),
    selected: None,
};

/// Blizzard_AchievementUI.xml:650-656: UI-Achievement-Category-Highlight ADD, TOPLEFT (0,0)
/// / BOTTOMRIGHT (-1,-7), TexCoords right 0.6640625.
pub const ACHIEVEMENT_CATEGORY: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(RowArt {
        art: Art::File(235_400, [0.0, 0.664_062_5, 0.0, 1.0]),
        insets: [0.0, 0.0, 1.0, -7.0],
        alpha: 1.0,
        additive: true,
    }),
    selected: None,
};

/// Blizzard_ProfessionsRecipeList.xml:213-216 `HighlightOverlay`: Professions_Recipe_Hover
/// at alpha 0.5 where the selected `Professions_Recipe_Active` overlay sits.
pub const PROFESSION_RECIPE: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(RowArt {
        art: Art::Atlas("Professions_Recipe_Hover"),
        insets: [-10.0, 1.0, 0.0, 0.0],
        alpha: 0.5,
        additive: false,
    }),
    selected: None,
};

/// Rows whose source hover art is authored by their own screen content.
pub const AUTHORED: RowTemplate = RowTemplate {
    normal: None,
    hover: None,
    selected: None,
};

/// PVPUI.xml:647/730 casual/rated activity selected and hover art (ADD, all points).
/// Normal/pushed art is on the button, not a static normal child.
pub const PVP_RATED_ACTIVITY: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::Atlas("pvpqueue-button-highlight"), true)),
    selected: Some(fill(Art::Atlas("pvpqueue-button-selected"), true)),
};
pub const PVP_CASUAL_ACTIVITY: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(fill(Art::Atlas("pvpqueue-button-casual-highlight"), true)),
    selected: Some(fill(Art::Atlas("pvpqueue-button-casual-selected"), true)),
};

/// PetBattleUI.xml:96 / Patchwerks.xml:86, BOTTOM(2,6), 169x42 inside192x200.
pub const PET_BATTLE_SELECTION: RowTemplate = RowTemplate {
    normal: None,
    hover: Some(RowArt {
        art: Art::File(603587, [0.56347656, 0.72851563, 0.76757813, 0.84960938]),
        insets: [13.5, 152.0, 9.5, 6.0],
        alpha: 0.5,
        additive: false,
    }),
    selected: Some(RowArt {
        art: Art::File(603587, [0.56347656, 0.72851563, 0.76757813, 0.84960938]),
        insets: [13.5, 152.0, 9.5, 6.0],
        alpha: 1.0,
        additive: false,
    }),
};

const TEMPLATES: [&RowTemplate; 19] = [
    &SEARCH,
    &RAID_INSTANCE,
    &PVP_INSTANCE,
    &PVP_CATEGORY,
    &PVP_RATED_ACTIVITY,
    &PVP_CASUAL_ACTIVITY,
    &PET_BATTLE_SELECTION,
    &MENU,
    &AUCTION_ROW,
    &AUCTION_CATEGORY,
    &TRANSMOG_OUTFIT,
    &GUILD_MEMBER,
    &COMMUNITIES_TAB,
    &COLUMN_HEADER,
    &NEIGHBORHOOD,
    &MOUNT,
    &PET,
    &ACHIEVEMENT_CATEGORY,
    &PROFESSION_RECIPE,
];

/// Shared hit target with the template's art around `content`. The hover region is always
/// present on the HIGHLIGHT layer, empty for [`AUTHORED`] rows: it is what marks the
/// button as a list row for keyboard focus, independent of which art it draws.
pub fn row(
    name: String,
    action: &str,
    enabled: bool,
    rect: (f32, f32, f32, f32),
    (template, selected): (&RowTemplate, bool),
    content: Element,
) -> Element {
    let (left, top, width, height) = rect;
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    let area = (0.0, 0.0, width, height);
    let mut children = Vec::new();
    if let Some(art) = template.normal {
        let normal = (Some(art), true, DrawLayer::Background);
        children.extend(art_region(format!("{name}RowNormal"), normal, area));
    }
    if let Some(art) = template.selected {
        let chosen = (Some(art), selected, DrawLayer::Border);
        children.extend(art_region(format!("{name}RowSelected"), chosen, area));
    }
    children.extend(content);
    let hover = (template.hover, true, DrawLayer::Highlight);
    children.extend(art_region(format!("{name}RowHighlight"), hover, area));
    rsx! { button { name: {DynName(name)}, left, top, width, height, pos_type: "absolute",
        disabled, onclick: action, button_default_skin: false, {children}
    } }
}

/// A left-aligned single label row of `template`.
pub fn text_row(
    name: String,
    text: &str,
    action: &str,
    (enabled, selected): (bool, bool),
    rect: (f32, f32, f32, f32),
    template: &RowTemplate,
) -> Element {
    let (_, _, width, height) = rect;
    let color = if enabled {
        NORMAL_FONT_COLOR
    } else {
        "0.5,0.5,0.5,1"
    };
    let label_height = if text.contains('\n') {
        height
    } else {
        height.min(20.0)
    };
    let content = label(
        format!("{name}Name"),
        text,
        (
            8.0,
            (height - label_height) / 2.0,
            width - 16.0,
            label_height,
        ),
        (12.0, color, "LEFT"),
    );
    row(name, action, enabled, rect, (template, selected), content)
}

/// Template selected art outside a row, for screens that draw it as a sibling.
pub fn selection_background(name: String, selected: bool, rect: (f32, f32, f32, f32)) -> Element {
    art_region(name, (SEARCH.selected, selected, DrawLayer::Border), rect)
}

/// Which Retail dropdown tick a [`menu_choice`] draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuTick {
    /// MenuVariants.lua:26-40 `CreateRadio`.
    Radio,
    /// MenuVariants.lua:9-24 `CreateCheckbox`.
    Checkbox,
}

/// One WowStyle1 dropdown entry: tick box, yellow tick when chosen, label right of it.
pub fn menu_choice(
    name: String,
    text: &str,
    action: &str,
    (tick, checked): (MenuTick, bool),
    rect: (f32, f32, f32, f32),
) -> Element {
    let (_, _, width, height) = rect;
    let (box_atlas, tick_atlas) = match tick {
        MenuTick::Radio => (
            "common-dropdown-tickradial",
            "common-dropdown-icon-radialtick-yellow",
        ),
        MenuTick::Checkbox => (
            "common-dropdown-ticksquare",
            "common-dropdown-icon-checkmark-yellow",
        ),
    };
    let top = (height - 20.0) / 2.0;
    let hidden = !checked;
    let mut content = rsx! {
        texture { name: {DynName(format!("{name}Box"))}, width: 12.0, height: 12.0,
            left: 0.0, top: {top + 4.0}, pos_type: "absolute", texture_atlas: box_atlas }
        texture { name: {DynName(format!("{name}Tick"))}, width: 15.0, height: 14.0,
            left: 0.5, top: {top + 2.0}, pos_type: "absolute", hidden, texture_atlas: tick_atlas }
    };
    content.extend(label(
        format!("{name}Name"),
        text,
        (19.0, top - 1.0, width - 19.0, 20.0),
        (12.0, "1.0,1.0,1.0,1.0", "LEFT"),
    ));
    row(name, action, true, rect, (&MENU, false), content)
}

/// MenuStyle1Mixin:Generate: `common-dropdown-bg` at TOPLEFT (-10,3) / BOTTOMRIGHT (10,-3),
/// alpha 0.925, behind the menu's entries.
pub fn menu_background(name: String, (width, height): (f32, f32)) -> Element {
    rsx! { texture { name: {DynName(name)}, width: {width + 20.0}, height: {height + 6.0},
    left: -10.0, top: -3.0, pos_type: "absolute", alpha: "0.925",
    texture_atlas: "common-dropdown-bg" } }
}

/// One template region inside `area` (`x, y, width, height`), inset per its source anchors.
fn art_region(
    name: String,
    (art, shown, layer): (Option<RowArt>, bool, DrawLayer),
    (x, y, width, height): (f32, f32, f32, f32),
) -> Element {
    let [inset_left, inset_top, inset_right, inset_bottom] = art.map_or(FULL, |art| art.insets);
    let left = x + inset_left;
    let top = y + inset_top;
    let width = width - inset_left - inset_right;
    let height = height - inset_top - inset_bottom;
    let hidden = !shown;
    let alpha = art.map_or(1.0, |art| art.alpha);
    let draw_layer = layer.as_str();
    match art.map(|art| art.art) {
        Some(Art::Atlas(atlas)) => rsx! { texture { name: {DynName(name)}, width, height,
        left, top, pos_type: "absolute", hidden, alpha, draw_layer, texture_atlas: atlas } },
        Some(Art::File(fdid, [l, r, t, b])) => {
            let coords = format!("{l},{r},{t},{b}");
            rsx! { texture { name: {DynName(name)}, width, height, left, top,
            pos_type: "absolute", hidden, alpha, draw_layer, texture_fdid: fdid,
            tex_coords: {coords.as_str()} } }
        }
        None => rsx! { texture { name: {DynName(name)}, width, height, left, top,
        pos_type: "absolute", hidden, draw_layer } },
    }
}

/// A row: an unskinned button carrying a HIGHLIGHT-layer region. Rows opt into native
/// keyboard focus; ordinary push buttons keep their existing behaviour.
pub fn is_selection_row(registry: &FrameRegistry, frame: &Frame) -> bool {
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        return false;
    };
    !button.use_default_skin
        && frame.children.iter().any(|id| {
            registry
                .get(*id)
                .is_some_and(|child| child.draw_layer == DrawLayer::Highlight)
        })
}

/// HIGHLIGHT-layer textures show while their button is hovered or keyboard-focused
/// (`highlight_locked`) and enabled, as WoW shows a button's HIGHLIGHT layer.
pub fn highlight_layer_shown(registry: &FrameRegistry, frame: &Frame) -> bool {
    if frame.draw_layer != DrawLayer::Highlight {
        return true;
    }
    let parent = frame.parent_id.and_then(|id| registry.get(id));
    let Some(WidgetData::Button(button)) = parent.and_then(|parent| parent.widget_data.as_ref())
    else {
        return true;
    };
    let lit = button.hovered || button.highlight_locked;
    lit && button.state != ui_toolkit::widgets::button::ButtonState::Disabled
}

/// Template art authored with ADD blending (rsx textures cannot set a blend mode).
/// Hover art is matched only on the HIGHLIGHT layer, other art elsewhere, so a template
/// reusing one source for a BLEND card and an ADD highlight stays distinct.
pub fn is_additive_row_art(frame: &Frame, texture: &TextureData) -> bool {
    let hover = frame.draw_layer == DrawLayer::Highlight;
    TEMPLATES
        .iter()
        .flat_map(|template| {
            let arts = if hover {
                [template.hover, None]
            } else {
                [template.normal, template.selected]
            };
            arts.into_iter().flatten()
        })
        .any(|art| art.additive && art_matches(art.art, texture))
}

fn art_matches(art: Art, texture: &TextureData) -> bool {
    match (art, &texture.source) {
        (Art::Atlas(name), TextureSource::Atlas(source)) => name == source,
        (Art::File(fdid, coords), TextureSource::FileDataId(source)) => {
            fdid == *source && coords == texture.tex_coords
        }
        _ => false,
    }
}
