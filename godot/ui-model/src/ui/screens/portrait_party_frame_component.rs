//! Static Retail PartyMemberFrame and Forever's conditional CharacterFrameOn art.
//! Geometry/source provenance: docs/wiki/systems/portrait-party-frames.md.
//! No roster, portrait rendering or presentation-setting side effects.

use shared::components::PowerType;
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use super::inworld_unit_frames_art::{AtlasArt, art, power_bar_atlas, sized_atlas_texture};
use super::inworld_unit_frames_layout::TextAnchors;
use super::inworld_unit_frames_parts::{
    BarSpec, WHITE, cropped_status_bar, portrait_slot, status_bar, unit_label,
};
use super::{FULL_PORTRAIT, PortraitSlot, dyn_name};
use crate::hud_layout::{HudAnchor, hud_layout};
use crate::status_text_data::StatusBarText;

pub const PARTY_FRAME: &str = "PartyFrame";
pub const MEMBER_WIDTH: f32 = 120.0;
pub const MEMBER_HEIGHT: f32 = 53.0;
pub const MAX_MEMBERS: usize = 4;
const RETAIL_ART: &str = "UI-HUD-UnitFrame-Party-PortraitOn";
const CAMELOT_ART: &str = "UI-HUD-UnitFrame-CharacterFrameOnParty-PortraitOn";
const RETAIL_HEALTH: &str = "UI-HUD-UnitFrame-Party-PortraitOn-Bar-Health";
// Forever-only set-0 records are not ingested by the shared atlas-name loader.
// Exact 1x source crops, not replacement art; provenance and retirement in the wiki.
const CHARACTER_SHEET: u32 = 4_631_591;
// Same FDID as Retail, different layout. Local 70205 bytes match these 69913 crops.
const CAMELOT_SHEET_FILE: &str = "data/forever-1.60.1.70205/textures/4631591.blp";
const CHARACTER_SHEET_SIZE: (f32, f32) = (1024.0, 512.0);
const CAMELOT_HEALTH: AtlasArt = art(
    CHARACTER_SHEET,
    CHARACTER_SHEET_SIZE,
    (195.0, 266.0, 340.0, 350.0),
);
// ToCharacterStyleArt:106 selects the player mana fill, not the party mana crop.
const CAMELOT_MANA: AtlasArt = art(
    CHARACTER_SHEET,
    CHARACTER_SHEET_SIZE,
    (815.0, 939.0, 326.0, 336.0),
);
const PORTRAIT_NAMES: [&str; MAX_MEMBERS] = [
    "PartyMemberFrame1Portrait",
    "PartyMemberFrame2Portrait",
    "PartyMemberFrame3Portrait",
    "PartyMemberFrame4Portrait",
];
const PET_PORTRAIT_NAMES: [&str; MAX_MEMBERS] = [
    "PartyMemberFrame1PetPortrait",
    "PartyMemberFrame2PetPortrait",
    "PartyMemberFrame3PetPortrait",
    "PartyMemberFrame4PetPortrait",
];

#[derive(Clone, Debug, PartialEq)]
pub struct PortraitPartyMemberView {
    pub name: String,
    pub health_fraction: f32,
    pub power_fraction: f32,
    pub power_type: PowerType,
    /// Name tint; Retail health art retains its locked green colour.
    pub class_rgb: [f32; 3],
    pub leader: bool,
    /// LFG-restricted leader draws the guide mark instead of the crown.
    pub guide: bool,
    pub role: GroupRoleSnapshot,
    pub offline: bool,
    pub dead: bool,
    pub pet: Option<PortraitPartyPetView>,
}

impl PortraitPartyMemberView {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            health_fraction: 0.0,
            power_fraction: 0.0,
            power_type: PowerType::Mana,
            class_rgb: [1.0, 0.82, 0.0],
            leader: false,
            guide: false,
            role: GroupRoleSnapshot::None,
            offline: false,
            dead: false,
            pet: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortraitPartyPetView {
    pub health_fraction: f32,
    pub dead: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PortraitPartyFrameState {
    /// Non-self members in presentation order; at most four are drawn.
    pub members: Vec<PortraitPartyMemberView>,
    /// Retail showPartyPets controls both visibility and the 26 vs 10 px gap.
    pub show_pets: bool,
}

/// Slot metadata consumed by the existing rendered-head/mask machinery in step 3.
pub fn party_portrait_slot(index: usize, skin: ActiveSkin) -> PortraitSlot {
    PortraitSlot {
        frame: PORTRAIT_NAMES[index],
        rect: (7.0, 6.0, 37.0, 37.0),
        mask_rect: (7.0, 6.0, 37.0, 37.0),
        tex_coords: FULL_PORTRAIT,
        mask_fdid: match skin {
            ActiveSkin::Modern => 3_528_314,
            ActiveSkin::Forever => 4_682_541,
        },
    }
}

pub fn portrait_party_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<PortraitPartyFrameState>()
        .expect("PortraitPartyFrameState");
    let skin = *ctx.get::<ActiveSkin>().expect("active skin");
    portrait_party_frame(&state, &hud_layout(ctx).party, skin)
}

pub fn portrait_party_frame(
    state: &PortraitPartyFrameState,
    anchor: &HudAnchor,
    skin: ActiveSkin,
) -> Element {
    portrait_party_frame_with_settings(state, anchor, skin, Default::default())
}

pub fn portrait_party_frame_with_settings(
    state: &PortraitPartyFrameState,
    anchor: &HudAnchor,
    skin: ActiveSkin,
    settings: game_engine_core::ui_layout_data::PartyFrameSettings,
) -> Element {
    let count = state.members.len().min(MAX_MEMBERS);
    let show_pets = settings.show_pets.unwrap_or(state.show_pets);
    let horizontal = settings.horizontal.unwrap_or(false);
    let (width, member_height) = settings.size();
    // The existing compact preset dimensions are the reference for portrait overrides.
    let sx = width / 98.0;
    let sy = member_height / 44.0;
    let member_width = MEMBER_WIDTH * sx;
    let gap = if show_pets { 26.0 } else { 10.0 };
    let height = if horizontal {
        MEMBER_HEIGHT * sy
    } else {
        (count as f32 * MEMBER_HEIGHT + count.saturating_sub(1) as f32 * gap) * sy
    };
    let width = if horizontal {
        count as f32 * member_width + count.saturating_sub(1) as f32 * gap * sx
    } else {
        member_width
    };
    let at = anchor.place((width, height));
    let members: Element = state
        .members
        .iter()
        .take(MAX_MEMBERS)
        .enumerate()
        .flat_map(|(index, view)| {
            let mut member = party_member_frame(index, view, 0.0, show_pets, skin);
            super::group_frames_component::resize_party_member(&mut member, sx, sy);
            let (x, y) = if horizontal {
                (index as f32 * (member_width + gap * sx), 0.0)
            } else {
                (0.0, index as f32 * (MEMBER_HEIGHT + gap) * sy)
            };
            super::group_frames_component::place_party_member(&mut member, x, y);
            member
        })
        .collect();
    rsx! { r#frame {
        name: {dyn_name(PARTY_FRAME.into())}, width, height,
        alpha: {settings.alpha()},
        {super::group_frames_component::party_chrome(PARTY_FRAME, width, height, settings)},
        hidden: {count == 0}, pos_type: "absolute",
        left: {at.left.as_str()}, right: {at.right.as_str()}, top: {at.top.as_str()}, bottom: {at.bottom.as_str()},
        margin_left: {at.margin_left}, margin_top: {at.margin_top},
        {members}
    } }
}

/// Retail online status applies to the sampled health image, not its white tint.
/// Portrait slots remain empty until step 3; an installed portrait texture gets the
/// same desaturation. UnitFrame.lua:948-950 tints offline power without desaturating it.
pub fn apply_portrait_party_postsetup(
    state: &PortraitPartyFrameState,
    registry: &mut FrameRegistry,
) {
    for (index, member) in state.members.iter().take(MAX_MEMBERS).enumerate() {
        let root = format!("PartyMemberFrame{}", index + 1);
        if let Some(texture) = party_texture_mut(registry, &format!("{root}HealthBarFill")) {
            texture.desaturated = member.offline;
        }
        if let Some(texture) = party_texture_mut(registry, &format!("{root}ManaBarFill")) {
            texture.vertex_color = if member.offline {
                [0.5, 0.5, 0.5, 1.0]
            } else {
                [1.0; 4]
            };
        }
        if let Some(texture) = party_texture_mut(registry, &format!("{root}Portrait")) {
            texture.desaturated = member.offline;
        }
    }
}

fn party_texture_mut<'a>(
    registry: &'a mut FrameRegistry,
    name: &str,
) -> Option<&'a mut ui_toolkit::widgets::texture::TextureData> {
    let id = registry.get_by_name(name)?;
    match registry.get_mut(id)?.widget_data.as_mut()? {
        WidgetData::Texture(texture) => Some(texture),
        _ => None,
    }
}

fn party_member_frame(
    index: usize,
    view: &PortraitPartyMemberView,
    y: f32,
    show_pets: bool,
    skin: ActiveSkin,
) -> Element {
    let root = format!("PartyMemberFrame{}", index + 1);
    let art = match skin {
        ActiveSkin::Modern => RETAIL_ART,
        ActiveSkin::Forever => CAMELOT_ART,
    };
    let portrait = portrait_slot(&party_portrait_slot(index, skin));
    let art = sized_atlas_texture(
        format!("{root}Art"),
        art,
        skin,
        |(w, h)| (1.0, 2.0, w, h),
        WHITE,
        false,
    );
    let bars = member_bars(&root, view, skin);
    let labels = member_labels(&root, view, skin);
    let pet = view
        .pet
        .as_ref()
        .filter(|_| show_pets && !view.offline)
        .map(|pet| pet_frame(index, pet, skin))
        .unwrap_or_default();
    rsx! { r#frame {
        name: {dyn_name(root)}, width: MEMBER_WIDTH, height: MEMBER_HEIGHT,
        mouse_enabled: true, pos_type: "absolute", pos_x: 0.0, pos_y: y,
        {portrait} {art} {bars} {labels} {pet}
    } }
}

fn member_bars(root: &str, view: &PortraitPartyMemberView, skin: ActiveSkin) -> Element {
    let status = if view.offline {
        "Offline"
    } else if view.dead {
        "Dead"
    } else {
        ""
    };
    let text = StatusBarText {
        center: status.into(),
        ..Default::default()
    };
    // PartyMemberFrame.lua:576-579 fills disconnected health to its maximum.
    let health = if view.offline {
        1.0
    } else {
        view.health_fraction
    };
    let mut bars = member_health_bar(root, health, &text, skin);
    bars.extend(member_power_bar(root, view, skin));
    bars
}

fn member_health_bar(root: &str, fraction: f32, text: &StatusBarText, skin: ActiveSkin) -> Element {
    let spec = BarSpec {
        name: format!("{root}HealthBar"),
        rect: (45.0, 19.0, 70.0, 10.0),
        fraction,
        art: (skin == ActiveSkin::Modern).then_some(RETAIL_HEALTH),
        text,
        anchors: TextAnchors::new(0.0, 0.0, 0.0),
        font_size: 10.0,
        hidden: false,
    };
    match skin {
        ActiveSkin::Modern => status_bar(spec),
        ActiveSkin::Forever => cropped_status_bar(spec, CAMELOT_HEALTH, CAMELOT_SHEET_FILE),
    }
}

fn member_power_bar(root: &str, view: &PortraitPartyMemberView, skin: ActiveSkin) -> Element {
    let rect = match skin {
        ActiveSkin::Modern => (41.0, 30.0, 74.0, 7.0),
        ActiveSkin::Forever => (46.0, 30.0, 69.0, 7.0),
    };
    let text = StatusBarText::default();
    let spec = BarSpec {
        name: format!("{root}ManaBar"),
        rect,
        // UnitFrame.lua:944-950 fills disconnected power to its maximum.
        fraction: if view.offline {
            1.0
        } else {
            view.power_fraction
        },
        art: party_power_atlas(view.power_type, skin),
        text: &text,
        anchors: TextAnchors::new(2.0, 4.0, 0.0),
        font_size: 10.0,
        hidden: false,
    };
    match (skin, character_power_art(view.power_type)) {
        (ActiveSkin::Forever, Some(art)) => cropped_status_bar(spec, art, CAMELOT_SHEET_FILE),
        _ => status_bar(spec),
    }
}

fn character_power_art(power: PowerType) -> Option<AtlasArt> {
    let (top, bottom) = match power {
        PowerType::Mana => return Some(CAMELOT_MANA),
        PowerType::Energy => (436.0, 443.0),
        PowerType::Focus => (445.0, 452.0),
        PowerType::Rage => (472.0, 479.0),
        PowerType::RunicPower => (481.0, 488.0),
        _ => return None,
    };
    Some(art(
        CHARACTER_SHEET,
        CHARACTER_SHEET_SIZE,
        (195.0, 265.0, top, bottom),
    ))
}

fn party_power_atlas(power: PowerType, skin: ActiveSkin) -> Option<&'static str> {
    use PowerType::*;
    match (skin, power) {
        (ActiveSkin::Modern, Mana) => Some("UI-HUD-UnitFrame-Party-PortraitOn-Bar-Mana"),
        (ActiveSkin::Modern, Rage) => Some("UI-HUD-UnitFrame-Party-PortraitOn-Bar-Rage"),
        (ActiveSkin::Modern, Focus) => Some("UI-HUD-UnitFrame-Party-PortraitOn-Bar-Focus"),
        (ActiveSkin::Modern, Energy) => Some("UI-HUD-UnitFrame-Party-PortraitOn-Bar-Energy"),
        (ActiveSkin::Modern, RunicPower) => {
            Some("UI-HUD-UnitFrame-Party-PortraitOn-Bar-RunicPower")
        }
        // UnitFrameManaBar_UpdateType uses info.atlas for these spec powers.
        (_, LunarPower | Maelstrom | Insanity | Fury | Pain) => power_bar_atlas(power),
        _ => None,
    }
}

fn member_labels(root: &str, view: &PortraitPartyMemberView, skin: ActiveSkin) -> Element {
    let [r, g, b] = view.class_rgb;
    let color = format!("{r},{g},{b},1");
    let mut labels = unit_label(
        dyn_name(format!("{root}Name")),
        &view.name,
        (46.0, 6.0, 57.0, 12.0),
        (&color, 10.0),
        "LEFT",
    );
    for (suffix, atlas, hidden) in [
        (
            "LeaderIcon",
            "UI-HUD-UnitFrame-Player-Group-LeaderIcon",
            !view.leader || view.guide,
        ),
        (
            "GuideIcon",
            "UI-HUD-UnitFrame-Player-Group-GuideIcon",
            !view.leader || !view.guide,
        ),
    ] {
        labels.extend(sized_atlas_texture(
            format!("{root}{suffix}"),
            atlas,
            skin,
            // PartyFrameTemplates.xml:302-311: BOTTOM to parent TOP (-10,-6).
            // WoW y points up; bottom is 6 px below the member's top.
            |(w, h)| (MEMBER_WIDTH / 2.0 - 10.0 - w / 2.0, 6.0 - h, w, h),
            WHITE,
            hidden,
        ));
    }
    let role = match view.role {
        GroupRoleSnapshot::Tank => Some("roleicon-tiny-tank"),
        GroupRoleSnapshot::Healer => Some("roleicon-tiny-healer"),
        GroupRoleSnapshot::Damage => Some("roleicon-tiny-dps"),
        GroupRoleSnapshot::None => None,
    };
    if let Some(role) = role {
        labels.extend(super::inworld_unit_frames_parts::art_texture(
            dyn_name(format!("{root}RoleIcon")),
            role,
            (103.0, 5.0, 12.0, 12.0),
            false,
        ));
    }
    labels
}

fn pet_frame(index: usize, view: &PortraitPartyPetView, skin: ActiveSkin) -> Element {
    let root = format!("PartyMemberFrame{}Pet", index + 1);
    let slot = PortraitSlot {
        frame: PET_PORTRAIT_NAMES[index],
        rect: (3.0, 3.0, 18.0, 18.0),
        mask_fdid: 3_528_314,
        mask_rect: (3.0, 3.0, 18.0, 18.0),
        tex_coords: FULL_PORTRAIT,
    };
    let portrait = portrait_slot(&slot);
    let art = sized_atlas_texture(
        format!("{root}Art"),
        RETAIL_ART,
        skin,
        |(w, h)| (0.0, 1.0, w * 0.5, h * 0.5),
        WHITE,
        false,
    );
    let text = StatusBarText {
        center: if view.dead {
            "Dead".into()
        } else {
            String::new()
        },
        ..Default::default()
    };
    let health = status_bar(BarSpec {
        name: format!("{root}HealthBar"),
        rect: (21.5, 9.0, 35.5, 5.0),
        fraction: view.health_fraction,
        art: Some(RETAIL_HEALTH),
        text: &text,
        anchors: TextAnchors::new(0.0, 0.0, 0.0),
        font_size: 5.0,
        hidden: false,
    });
    rsx! { r#frame { name: {dyn_name(root)}, width: 64.0, height: 23.0, pos_type: "absolute", pos_x: 23.0, pos_y: 43.0,
        {portrait} {art} {health}
    } }
}
