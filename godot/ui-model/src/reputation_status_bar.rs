//! Retail ReputationBar.lua/Overrides and StatusTrackingBarContainerTemplate.
//! XP has priority4; reputation1 occupies container2, or container1 without XP.
use crate::quest_art::named_atlas_texture;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_flare::{
    flare_border_tinted, flare_layer,
};
use shared::protocol_snapshots::WatchedReputationSnapshot;
use shared::reputation::Standing;
use ui_toolkit::{atlas::ActiveSkin, rsx, screen::SharedContext, widget_def::Element};

const CONTAINER: (f32, f32) = (571., 17.);
const BAR: (f32, f32) = (565., 11.);
const INSET: f32 = 1.;
/// Standard/EditModePresetLayoutConstants.lua: STATUS_BAR_2_ANCHOR_OFFSET_Y.
const SECONDARY_OFFSET: f32 = 17.;
const BORDER_OUTSET: f32 = 4.;
const BORDER_EDGE: f32 = 16.;
const BORDER_COLOR: &str = "0.8,0.6,0.34,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct ReputationBarState {
    pub watched: Option<WatchedReputationSnapshot>,
    pub xp_visible: bool,
    pub hovered: bool,
}

pub fn reputation_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ReputationBarState>()
        .expect("ReputationBarState required");
    let Some(watched) = state
        .watched
        .as_ref()
        .filter(|watched| !watched.name.is_empty())
    else {
        return Vec::new();
    };
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    place_bar(state, watched, skin)
}

fn place_bar(
    state: &ReputationBarState,
    watched: &WatchedReputationSnapshot,
    skin: ActiveSkin,
) -> Element {
    let bottom = if state.xp_visible {
        SECONDARY_OFFSET
    } else {
        0.
    };
    let mut parts = bar_parts(watched, skin);
    parts.extend(hover_text(watched, state.hovered));
    rsx! {r#frame {name:"ReputationStatusBar",width:{CONTAINER.0},height:{CONTAINER.1},
        mouse_enabled:true,pos_type:"absolute",left:"50%",margin_left:{-CONTAINER.0/2.},bottom,
        {parts}
    }}
}

fn fill_atlas(watched: &WatchedReputationSnapshot) -> &'static str {
    if watched.is_major_faction {
        return "UI-HUD-ExperienceBar-Fill-Reputation-Faction-Blue";
    }
    match watched.reaction {
        Standing::Hated | Standing::Hostile => "UI-HUD-ExperienceBar-Fill-Reputation-Faction-Red",
        Standing::Unfriendly => "UI-HUD-ExperienceBar-Fill-Reputation-Faction-Orange",
        Standing::Neutral => "UI-HUD-ExperienceBar-Fill-Reputation-Faction-Yellow",
        Standing::Friendly | Standing::Honored | Standing::Revered | Standing::Exalted => {
            "UI-HUD-ExperienceBar-Fill-Reputation-Faction-Green"
        }
    }
}

fn bar_parts(watched: &WatchedReputationSnapshot, skin: ActiveSkin) -> Element {
    assert!(
        watched.maximum > 0,
        "watched reputation maximum must be normalized above zero"
    );
    let ratio = (watched.value as f32 / watched.maximum as f32).clamp(0., 1.);
    let mut parts = named_atlas_texture(
        "ReputationStatusBarBackground".into(),
        "UI-HUD-ExperienceBar-Background",
        (INSET, INSET, BAR.0, BAR.1),
    );
    parts.extend(named_atlas_texture(
        "ReputationStatusBarFill".into(),
        fill_atlas(watched),
        (INSET, INSET, BAR.0 * ratio, BAR.1),
    ));
    parts.extend(frame_art(skin));
    parts
}

fn frame_art(skin: ActiveSkin) -> Element {
    if skin == ActiveSkin::Modern {
        return named_atlas_texture(
            "ReputationStatusBarFrame".into(),
            "UI-HUD-ExperienceBar-Frame",
            (0., 0., CONTAINER.0, CONTAINER.1),
        );
    }
    // Same supplied bronze border as the existing Forever XP skin; no new art or geometry.
    let border = flare_border_tinted(
        "ReputationStatusBar",
        (BAR.0 + 2. * BORDER_OUTSET, BAR.1 + 2. * BORDER_OUTSET),
        BORDER_EDGE,
        BORDER_COLOR,
    );
    flare_layer(
        "ReputationStatusBarFrame".into(),
        (
            INSET - BORDER_OUTSET,
            INSET - BORDER_OUTSET,
            BAR.0 + 2. * BORDER_OUTSET,
            BAR.1 + 2. * BORDER_OUTSET,
        ),
        border,
    )
}

fn hover_text(watched: &WatchedReputationSnapshot, hovered: bool) -> Element {
    let text = if watched.capped {
        watched.name.clone()
    } else {
        format!("{} {} / {}", watched.name, watched.value, watched.maximum)
    };
    let hidden = !hovered;
    rsx! {r#frame {name:"ReputationStatusBarOverlay",left:INSET,top:INSET,width:{BAR.0},height:{BAR.1},pos_type:"absolute",
        fontstring {name:"ReputationStatusBarText",text:{text.as_str()},hidden,width:{BAR.0},height:12.0,
            font:"FrizQuadrata",font_size:10.0,font_color:"1,1,1,1",outline:"OUTLINE",justify_h:"CENTER",
            pos_type:"absolute",left:0.0,top:{(BAR.1-12.)/2.-1.}
        }
    }}
}
