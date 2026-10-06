use super::*;

pub(super) fn header(group: &SpellbookGroup, rect: [f32; 2], s: f32) -> Element {
    let [x, y] = rect;
    let name = format!("SpellBookHeader{}", group.name.replace(' ', ""));
    let children: Element = [
        art_colored(
            format!("{name}Backplate"),
            &HEADER_BACKPLATE,
            [-85.0, (HEADER_H - 106.0) / 2.0 - 10.0, 416.0, 106.0],
            s,
            "1.0,1.0,1.0,0.65",
            false,
        ),
        label(
            Label {
                name: format!("{name}Text"),
                text: &group.name,
                rect: [-8.0, 6.0, VIEW_W - 60.0 + 8.0, 30.0],
                size: HEADER_SIZE,
                color: FONT_COLOR,
                justify: "LEFT",
            },
            s,
        ),
        art(
            format!("{name}Border"),
            &DIVIDER,
            [-32.0, HEADER_H - 11.0, VIEW_W - 60.0 + 32.0, 11.0],
            s,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: {VIEW_W * s},
            height: {HEADER_H * s},
            pos_type: "absolute",
            pos_x: {x * s},
            pos_y: {y * s},
            {children}
        }
    }
}

/// Button border per `SpellBookItemMixin.ArtSet`: Square (-11,1 / 1,-7 active;
/// -10,1 / 2,-5 inactive) or Circle for passives (all points).
fn item_border(name: &str, item: &SpellbookItemView, s: f32) -> Element {
    let top = (ITEM_H - BUTTON_SIZE) / 2.0;
    let (border, rect) = match (item.passive, item.available_at.is_some()) {
        (false, false) => (
            ICON_FRAME,
            [-11.0, top - 1.0, BUTTON_SIZE + 12.0, BUTTON_SIZE + 8.0],
        ),
        (false, true) => (
            ICON_FRAME_INACTIVE,
            [-10.0, top - 1.0, BUTTON_SIZE + 12.0, BUTTON_SIZE + 6.0],
        ),
        (true, false) => (PASSIVE_FRAME, [0.0, top, BUTTON_SIZE, BUTTON_SIZE]),
        (true, true) => (PASSIVE_FRAME_INACTIVE, [0.0, top, BUTTON_SIZE, BUTTON_SIZE]),
    };
    art(format!("{name}Border"), &border, rect, s)
}

fn item_texts(name: &str, item: &SpellbookItemView, s: f32) -> Element {
    let color = if item.available_at.is_some() {
        UNLEARNED_FONT_COLOR
    } else {
        FONT_COLOR
    };
    let level = item
        .available_at
        .map(|level| format!("{AVAILABLE_AT}{level}"));
    let lines: Vec<(&str, &str, f32)> = [
        Some(("Name", item.name.as_str(), NAME_SIZE)),
        (!item.subtext.is_empty()).then_some(("SubName", item.subtext.as_str(), SUBTEXT_SIZE)),
        level
            .as_deref()
            .map(|text| ("RequiredLevel", text, SUBTEXT_SIZE)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let line_h = |size: f32| size + 3.0;
    let block: f32 = lines.iter().map(|&(_, _, size)| line_h(size)).sum::<f32>()
        + 2.0 * (lines.len() as f32 - 1.0);
    let mut y = (ITEM_H - block) / 2.0 - 1.0;
    let mut out = Vec::new();
    for (part, text, size) in lines {
        out.extend(label(
            Label {
                name: format!("{name}{part}"),
                text,
                rect: [TEXT_LEFT, y, ITEM_W - TEXT_LEFT, line_h(size)],
                size,
                color,
                justify: "LEFT",
            },
            s,
        ));
        y += line_h(size) + 2.0;
    }
    out
}

pub(super) fn item(item: &SpellbookItemView, rect: [f32; 2], s: f32) -> Element {
    let [x, y] = rect;
    let name = spell_item_name(item.spell_id);
    let icon_top = (ITEM_H - ICON_SIZE) / 2.0;
    let icon_left = (BUTTON_SIZE - ICON_SIZE) / 2.0;
    let tint = if item.available_at.is_some() {
        UNLEARNED_TINT
    } else {
        "1.0,1.0,1.0,1.0"
    };
    let castable = !item.passive && item.available_at.is_none();
    let onclick = castable.then(|| format!("{ACTION_SPELLBOOK_CAST}{}", item.spell_id));
    let icon_name = format!("{name}Icon");
    // A spell whose icon is not drawable yet (FDID 0) has no icon texture.
    let icon: Element = if item.icon_fdid == 0 {
        Element::default()
    } else {
        rsx! {
            texture {
                name: {DynName(icon_name)},
                width: {ICON_SIZE * s},
                height: {ICON_SIZE * s},
                texture_fdid: {item.icon_fdid},
                vertex_color: tint,
                pos_type: "absolute",
                pos_x: {icon_left * s},
                pos_y: {icon_top * s},
            }
        }
    };
    let children: Element = [
        art_colored(
            format!("{name}Backplate"),
            &ITEM_BACKPLATE,
            [
                (ITEM_W - 256.0) / 2.0 + 5.0,
                (ITEM_H - 64.0) / 2.0 + 5.0,
                256.0,
                64.0,
            ],
            s,
            "1.0,1.0,1.0,0.25",
            false,
        ),
        icon,
        item_border(&name, item, s),
        item_texts(&name, item, s),
    ]
    .into_iter()
    .flatten()
    .collect();
    let button_name = format!("{name}Button");
    let button_y = (ITEM_H - BUTTON_SIZE) / 2.0 * s;
    let button = match onclick {
        Some(onclick) => rsx! {
            button {
                name: {DynName(button_name)},
                width: {BUTTON_SIZE * s},
                height: {BUTTON_SIZE * s},
                onclick,
                button_default_skin: false,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: button_y,
            }
        },
        None => rsx! {
            button {
                name: {DynName(button_name)},
                width: {BUTTON_SIZE * s},
                height: {BUTTON_SIZE * s},
                button_default_skin: false,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: button_y,
            }
        },
    };
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: {ITEM_W * s},
            height: {ITEM_H * s},
            pos_type: "absolute",
            pos_x: {x * s},
            pos_y: {y * s},
            {children}
            {button}
        }
    }
}

pub(super) fn view(
    state: &SpellbookFrameState,
    placements: Option<&Vec<Placement>>,
    index: usize,
    s: f32,
) -> Element {
    let groups = state
        .selected_category()
        .map_or(&[][..], |category| category.groups.as_slice());
    let children: Element = placements
        .into_iter()
        .flatten()
        .flat_map(|placement| match *placement {
            Placement::Header { group, y } => header(&groups[group], [0.0, y], s),
            Placement::Item {
                group,
                item: index,
                x,
                y,
            } => item(&groups[group].items[index], [x, y], s),
        })
        .collect();
    let left = if index == 0 { VIEW1_LEFT } else { VIEW2_LEFT };
    rsx! {
        r#frame {
            name: {DynName(format!("SpellBookView{}", index + 1))},
            width: {VIEW_W * s},
            height: {VIEW_H * s},
            pos_type: "absolute",
            pos_x: {left * s},
            pos_y: {VIEW_TOP * s},
            {children}
        }
    }
}

/// Fit only the name into its reserved line; subtext and level retain their layout.
fn fit_item_name(registry: &mut FrameRegistry, name: &str) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    let frame = registry.get_mut(id).expect("registered spell name");
    let (Dimension::Fixed(width), Dimension::Fixed(height)) = (frame.width, frame.height) else {
        panic!("spell name must have fixed bounds: {name}");
    };
    let Some(WidgetData::FontString(text)) = frame.widget_data.as_mut() else {
        panic!("spell name must be a FontString: {name}");
    };
    let (text_width, text_height) = measure_text(&text.text, text.font, text.font_size)
        .expect("spell name font must be available");
    let fit = (width.floor() / text_width)
        .min(height.floor() / text_height)
        .min(1.0);
    text.font_size *= fit;
}

/// Retail desaturates the icons of spells not learned yet (`SetDesaturated`).
pub fn apply_spellbook_postsetup(state: &SpellbookFrameState, registry: &mut FrameRegistry) {
    for spec in &state.specializations {
        let name = format!("ClassSpec{}Description", spec.id);
        if let Some(id) = registry.get_by_name(&name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::FontString(text)) = frame.widget_data.as_mut()
        {
            text.word_wrap = true;
        }
    }
    let Some(category) = state.selected_category() else {
        return;
    };
    for item in category.groups.iter().flat_map(|group| &group.items) {
        fit_item_name(registry, &format!("{}Name", spell_item_name(item.spell_id)));
        let name = format!("{}Icon", spell_item_name(item.spell_id));
        if let Some(id) = registry.get_by_name(&name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
        {
            texture.desaturated = item.available_at.is_some();
        }
    }
}
