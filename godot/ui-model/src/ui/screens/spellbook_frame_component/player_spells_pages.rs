//! PlayerSpellsFrame.xml:20-47 and ClassSpecializationsFrame.xml:54-165,311-315.
//! Forever uses the same Retail page mechanics, not its Classic talent templates.

use super::*;
use crate::ui::screens::quest_art::panel_button;

/// PlayerSpellsFrame.xml:20-29: `TabSystem` TOPLEFT at the frame's BOTTOMLEFT 22,2.
/// `TabSystemButtonTemplate` (TabSystemTemplates.xml:3-96): 32-high buttons; left cap
/// TOPLEFT, right cap TOPRIGHT x 6 (active x 7), middle tiled between them; text 10 high
/// at CENTER y +2, selected y -3 (TabSystemTemplates.lua:30-39), `width - 10` wide
/// (`UpdateTabWidth`, .lua:154-177).
pub(super) fn bottom_tabs(state: &SpellbookFrameState, s: f32) -> Element {
    let mut left = 22.0;
    let mut children = Vec::new();
    for (index, tab) in PlayerSpellsTab::ALL.into_iter().enumerate() {
        let frame = format!("PlayerSpellsTab{}", index + 1);
        let width = tab_width(tab.title());
        let selected = state.tab == tab;
        let (l, m, r, height, right_x, text_offset) = if selected {
            (
                ACTIVE_TAB_LEFT,
                ACTIVE_TAB_MIDDLE,
                ACTIVE_TAB_RIGHT,
                42.0,
                7.0,
                -3.0,
            )
        } else {
            (TAB_LEFT, TAB_MIDDLE, TAB_RIGHT, 36.0, 6.0, 2.0)
        };
        let right = width + right_x - TAB_RIGHT_W;
        let pieces: Element = [
            art(
                format!("{frame}Left"),
                &l,
                [0.0, 0.0, TAB_LEFT_W, height],
                s,
            ),
            art(
                format!("{frame}Middle"),
                &m,
                [TAB_LEFT_W, 0.0, right - TAB_LEFT_W, height],
                s,
            ),
            art(
                format!("{frame}Right"),
                &r,
                [right, 0.0, TAB_RIGHT_W, height],
                s,
            ),
            label(
                Label {
                    name: format!("{frame}Text"),
                    text: tab.title(),
                    rect: [5.0, (TAB_H - 10.0) / 2.0 - text_offset, width - 10.0, 10.0],
                    size: TAB_TEXT_SIZE,
                    color: if selected {
                        TAB_TEXT_SELECTED
                    } else {
                        TAB_TEXT
                    },
                    justify: "CENTER",
                },
                s,
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        children.extend(rsx! {
            r#frame {
                name: {DynName(frame)}, width: {width * s}, height: {TAB_H * s},
                onclick: {format!("{ACTION_PLAYER_SPELLS_TAB}{index}")},
                pos_type: "absolute", pos_x: {left * s}, pos_y: {(FRAME_H - 2.0) * s},
                {pieces}
            }
        });
        left += width + TAB_SPACING;
    }
    children
}

fn atlas_texture(name: String, atlas: &str, rect: [f32; 4], s: f32) -> Element {
    let [x, y, width, height] = rect;
    rsx! {
        texture {
            name: {DynName(name)}, texture_atlas: atlas,
            width: {width * s}, height: {height * s},
            pos_type: "absolute", pos_x: {x * s}, pos_y: {y * s},
        }
    }
}

/// ClassSpecFrameMixin:UpdateSpecContents (.lua:161-177): equal-width columns.
pub(super) fn specializations(state: &SpellbookFrameState, s: f32) -> Element {
    let width = BOOK_W / state.specializations.len().max(1) as f32;
    let mut children = atlas_texture(
        "ClassSpecBackground".into(),
        "spec-background",
        [0.0, 0.0, BOOK_W, BOOK_H],
        s,
    );
    for (index, spec) in state.specializations.iter().enumerate() {
        children.extend(spec_column(
            spec,
            width,
            index as f32 * width,
            state.can_activate_spec,
            s,
        ));
    }
    rsx! {
        r#frame {
            name: "ClassSpecFrame", width: {BOOK_W * s}, height: {BOOK_H * s},
            pos_type: "absolute", pos_x: 0.0, pos_y: {BOOK_Y * s},
            {children}
        }
    }
}

fn spec_column(
    spec: &SpecializationChoice,
    width: f32,
    left: f32,
    enabled: bool,
    s: f32,
) -> Element {
    let name = format!("ClassSpec{}", spec.id);
    let mut children: Element = [
        atlas_texture(
            format!("{name}Thumbnail"),
            &spec.thumbnail,
            [(width - 306.0) / 2.0, 38.0, 306.0, 186.0],
            s,
        ),
        atlas_texture(
            format!("{name}Border"),
            if spec.active {
                "spec-thumbnailborder-on"
            } else {
                "spec-thumbnailborder-off"
            },
            [(width - 306.0) / 2.0, 38.0, 306.0, 186.0],
            s,
        ),
        label(
            Label {
                name: format!("{name}Name"),
                text: &spec.name,
                rect: [0.0, 278.0, width, 38.0],
                size: 30.0,
                color: TAB_TEXT_SELECTED,
                justify: "CENTER",
            },
            s,
        ),
        label(
            Label {
                name: format!("{name}Role"),
                text: &spec.role,
                rect: [0.0, 327.0, width, 24.0],
                size: 14.0,
                color: TAB_TEXT,
                justify: "CENTER",
            },
            s,
        ),
        label(
            Label {
                name: format!("{name}Description"),
                text: &spec.description,
                rect: [(width - 280.0) / 2.0, 378.0, 280.0, 220.0],
                size: 14.0,
                color: TAB_TEXT,
                justify: "CENTER",
            },
            s,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    if spec.icon_fdid != 0 {
        children.extend(file_texture(
            format!("{name}Icon"),
            spec.icon_fdid,
            [(width - 32.0) / 2.0, 238.0, 32.0, 32.0],
            s,
        ));
    }
    if spec.active {
        children.extend(label(
            Label {
                name: format!("{name}Active"),
                text: "Active",
                rect: [0.0, BOOK_H - 119.0, width, 24.0],
                size: 18.0,
                color: "0.0,1.0,0.0,1.0",
                justify: "CENTER",
            },
            s,
        ));
    } else {
        children.extend(panel_button(
            format!("{name}Activate"),
            "Activate",
            &format!("{ACTION_ACTIVATE_SPEC}{}", spec.id),
            enabled,
            (
                ((width - 160.0) / 2.0) * s,
                (BOOK_H - 117.0) * s,
                160.0 * s,
                22.0 * s,
            ),
        ));
    }
    rsx! {
        r#frame {
            name: {DynName(name)}, width: {width * s}, height: {BOOK_H * s},
            pos_type: "absolute", pos_x: {left * s}, pos_y: 0.0,
            {children}
        }
    }
}
