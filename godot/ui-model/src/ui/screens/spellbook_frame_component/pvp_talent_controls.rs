//! Mainline PvPTalentSlotTemplates (three58x52 buttons) and220x40 flyout rows.
use super::*;
use shared::protocol::{PvpTalentSlotSnapshot, PvpTalentSnapshot, PvpTalentsSnapshot};

const SLOT_W: f32 = 58.0;
const SLOT_H: f32 = 52.0;
const FLYOUT_W: f32 = 220.0;
const ROW_H: f32 = 40.0;

fn origin(index: u8) -> [f32; 2] {
    // ClassTalentsFrame.xml RIGHT -114; PvPTalentSlotTemplates.xml tray200x70,
    // first slot LEFT4, subsequent LEFT to previous RIGHT +4.
    [
        BOOK_W - 114.0 - 200.0 + 4.0 + f32::from(index - 1) * (SLOT_W + 4.0),
        BOOK_H - FOOTER_HEIGHT / 2.0 - SLOT_H / 2.0,
    ]
}
pub(super) fn render(view: &TalentView, scale: f32) -> Element {
    let mut children: Element = (1..=3)
        .flat_map(|index| {
            let slot = view
                .pvp
                .as_ref()
                .and_then(|p| p.slots.iter().find(|s| s.index == index));
            let choice = slot
                .and_then(|s| s.selected)
                .and_then(|id| view.pvp.as_ref()?.choices.iter().find(|c| c.id == id));
            render_slot(index, slot, choice, view.pvp_pending, scale)
        })
        .collect();
    children.extend(render_label(scale));
    children.extend(flyout(view, scale));
    children
}
fn render_label(scale: f32) -> Element {
    // PvPTalentSlotTemplates.xml Label RIGHT to tray LEFT -16.
    let text = "PvP Talents";
    let (width, _) = measure_text(text, GameFont::FrizQuadrata, 16.0).expect("PvP label font");
    label(
        Label {
            font_object: "GameFontHighlightLarge",
            name: "PvPTalentLabel".into(),
            text,
            rect: [
                BOOK_W - 114.0 - 200.0 - 16.0 - width,
                BOOK_H - FOOTER_HEIGHT / 2.0 - 12.0,
                width,
                24.0,
            ],
            size: 16.0,
            color: TAB_TEXT,
            justify: "RIGHT",
        },
        scale,
    )
}
fn render_slot(
    index: u8,
    slot: Option<&PvpTalentSlotSnapshot>,
    choice: Option<&PvpTalentSnapshot>,
    pending: bool,
    scale: f32,
) -> Element {
    let [x, y] = origin(index);
    let unlocked = slot.is_some_and(|s| s.unlocked);
    let disabled = !unlocked || pending;
    let action = if disabled {
        String::new()
    } else {
        format!("pvp:talent-slot:{index}")
    };
    let border = if !unlocked {
        "talents-node-pvp-locked"
    } else if choice.is_some() {
        "talents-node-pvp-filled"
    } else {
        "talents-node-pvp-green"
    };
    let mut children = retail_atlas(
        &format!("PvPTalentSlot{index}Shadow"),
        "talents-node-pvp-shadow",
        [x, y, SLOT_W, SLOT_H],
        scale,
    );
    if let Some(choice) = choice {
        children.extend(icon(
            &format!("PvPTalentSlot{index}Icon"),
            choice.icon,
            [x + 11.0, y + 8.0],
            scale,
        ));
    } else if unlocked {
        children.extend(retail_atlas(
            &format!("PvPTalentSlot{index}Empty"),
            "pvptalents-talentborder-empty",
            [x + 11.0, y + 8.0, 36.0, 36.0],
            scale,
        ));
    }
    children.extend(retail_atlas(
        &format!("PvPTalentSlot{index}Border"),
        border,
        [x, y, SLOT_W, SLOT_H],
        scale,
    ));
    children.extend(rsx! {button {name:{DynName(format!("PvPTalentSlot{index}"))},onclick:{action.as_str()},disabled,
        width:{SLOT_W*scale},height:{SLOT_H*scale},button_default_skin:false,
        pos_type:"absolute",pos_x:{x*scale},pos_y:{y*scale},frame_level:1003,
    }});
    children
}
fn icon(name: &str, fdid: u32, position: [f32; 2], scale: f32) -> Element {
    if fdid == 0 {
        return vec![];
    }
    art(
        name.into(),
        &AtlasArt {
            fdid,
            atlas: (1.0, 1.0),
            rect: (0.0, 1.0, 0.0, 1.0),
        },
        [position[0], position[1], 36.0, 36.0],
        scale,
    )
}
fn choices_for_slot<'a>(talents: &'a PvpTalentsSnapshot, slot: u8) -> Vec<&'a PvpTalentSnapshot> {
    let selected_here = talents
        .slots
        .iter()
        .find(|s| s.index == slot)
        .and_then(|s| s.selected);
    let selected_other = |id| {
        talents
            .slots
            .iter()
            .any(|s| s.index != slot && s.selected == Some(id))
    };
    let mut choices: Vec<_> = talents
        .choices
        .iter()
        .filter(|c| c.slot_mask & (1 << slot) != 0)
        .collect();
    // PvPTalentListTemplates.lua:203: unlocked, required level, selected elsewhere, ID.
    choices.sort_by_key(|c| {
        (
            c.unavailable_reason.is_some(),
            if c.unavailable_reason.is_some() {
                c.required_level
            } else {
                0
            },
            selected_other(c.id) && selected_here != Some(c.id),
            c.id,
        )
    });
    choices
}
fn flyout(view: &TalentView, scale: f32) -> Element {
    let Some(index) = view.pvp_slot else {
        return vec![];
    };
    let Some(talents) = &view.pvp else {
        return vec![];
    };
    let Some(slot) = talents.slots.iter().find(|s| s.index == index) else {
        return vec![];
    };
    let choices = choices_for_slot(talents, index);
    let [slot_x, slot_y] = origin(index);
    let height = choices.len() as f32 * ROW_H;
    let [x, y] = [slot_x + SLOT_W / 2.0 - FLYOUT_W / 2.0, slot_y - height];
    let mut children = flyout_background(height, scale);
    for (row, choice) in choices.into_iter().enumerate() {
        let selected = talents.slots.iter().any(|s| s.selected == Some(choice.id));
        let disabled =
            view.pvp_pending || !slot.can_change || selected || choice.unavailable_reason.is_some();
        children.extend(choice_row(
            choice,
            row as f32 * ROW_H,
            disabled,
            selected,
            scale,
        ));
    }
    let mut overlay = rsx! {button {name:"PvPTalentListDismiss",onclick:"pvp:talents-close",button_default_skin:false,
        width:{BOOK_W*scale},height:{BOOK_H*scale},pos_type:"absolute",pos_x:0.0,pos_y:0.0,frame_level:1000,
    }};
    overlay.extend(
        rsx! {r#frame {name:"PvPTalentList",width:{FLYOUT_W*scale},height:{height*scale},
            pos_type:"absolute",pos_x:{x*scale},pos_y:{y*scale},frame_level:1002,{children}
        }},
    );
    overlay
}
fn flyout_background(height: f32, scale: f32) -> Element {
    let mut children = retail_atlas(
        "PvPTalentListMiddle",
        "talents-pvpflyout-background-middle",
        [0.0, 0.0, FLYOUT_W, height],
        scale,
    );
    for (name, atlas, bottom) in [
        (
            "PvPTalentListTop",
            "talents-pvpflyout-background-top",
            false,
        ),
        (
            "PvPTalentListBottom",
            "talents-pvpflyout-background-bottom",
            true,
        ),
    ] {
        let region = resolve_region(atlas, ActiveSkin::Modern).expect("Retail PvP flyout atlas");
        let top = if bottom {
            height + 30.0 - region.height
        } else {
            -25.0
        };
        children.extend(retail_atlas(
            name,
            atlas,
            [
                (FLYOUT_W - region.width) / 2.0,
                top,
                region.width,
                region.height,
            ],
            scale,
        ));
    }
    children
}
fn choice_row(
    choice: &PvpTalentSnapshot,
    y: f32,
    disabled: bool,
    selected: bool,
    scale: f32,
) -> Element {
    let action = if disabled {
        String::new()
    } else {
        format!("pvp:talent-choice:{}", choice.id)
    };
    let color = if disabled {
        "0.5,0.5,0.5,1.0"
    } else {
        TAB_TEXT
    };
    let mut children = icon(
        &format!("PvPTalentChoice{}Icon", choice.id),
        choice.icon,
        [2.0, y + 2.0],
        scale,
    );
    let border = if selected {
        "talents-node-pvpflyout-yellow"
    } else {
        "talents-node-pvpflyout-green"
    };
    children.extend(retail_atlas(
        &format!("PvPTalentChoice{}Border", choice.id),
        border,
        [2.0, y + 2.0, 36.0, 36.0],
        scale,
    ));
    children.extend(label(
        Label {
            font_object: "GameFontHighlightSmall",
            name: format!("PvPTalentChoice{}Name", choice.id),
            text: &choice.name,
            rect: [58.0, y + 14.0, 150.0, 12.0],
            size: 12.0,
            color,
            justify: "LEFT",
        },
        scale,
    ));
    children.extend(rsx! {button {name:{DynName(format!("PvPTalentChoice{}",choice.id))},onclick:{action.as_str()},disabled,
        button_default_skin:false,width:{FLYOUT_W*scale},height:{ROW_H*scale},pos_type:"absolute",pos_x:0.0,pos_y:{y*scale},frame_level:1004,
    }});
    children
}
