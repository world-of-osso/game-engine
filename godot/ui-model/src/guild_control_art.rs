//! GuildControlUI XML + Mainline UIPanelTemplates TranslucentFrame/RankChange art.
use crate::bank_art::{cropped, label, texture};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR};
use ui_toolkit::{rsx, widget_def::Element};

/// Source OnMouseDown/up appearance; called after native input mutates button state.
pub fn refresh_button_art(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::widgets::{button::ButtonState, texture::TextureSource};
    let buttons: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| {
            let name = frame.name.as_ref()?;
            if !name.starts_with("GuildControlUI") {
                return None;
            }
            let Some(WidgetData::Button(button)) = &frame.widget_data else {
                return None;
            };
            Some((name.clone(), button.state))
        })
        .collect();
    for (name, state) in buttons {
        let pushed = state == ButtonState::Pushed;
        if let Some(id) = registry.get_by_name(&format!("{name}Box")) {
            if let Some(WidgetData::Texture(texture)) =
                &mut registry.get_mut(id).expect("checkbox box").widget_data
            {
                texture.source = TextureSource::FileDataId(if pushed { 130752 } else { 130755 });
            }
        }
        if let Some(id) = registry.get_by_name(&format!("{name}Background")) {
            if let Some(WidgetData::Texture(texture)) =
                &mut registry.get_mut(id).expect("square background").widget_data
            {
                texture.tex_coords = if pushed {
                    [0.453125, 0.859375, 0.234375, 0.640625]
                } else {
                    [0.015625, 0.421875, 0.234375, 0.640625]
                };
            }
        }
        if let Some(id) = registry.get_by_name(&format!("{name}Icon")) {
            let frame = registry.get_mut(id).expect("square icon");
            frame.position.left =
                ui_toolkit::layout_values::Val::Px(if pushed { 6.0 } else { 7.0 });
            frame.position.top = ui_toolkit::layout_values::Val::Px(if pushed { 8.0 } else { 7.0 });
        }
    }
}

pub fn navigation_x() -> f32 {
    let width = ui_toolkit::text_measure::measure_text(
        "Guild Control",
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        12.0,
    )
    .expect("GuildControlUI title font")
    .0;
    24.0 + width + 10.0
}

pub fn shell() -> Element {
    let mut out = rsx! { texture { name: "GuildControlUIBg", width: 318.0, height: 424.0,
    left: 10.0, top: 10.0, pos_type: "absolute", vertex_color: "0,0,0,0.9" }
    texture { name: "GuildControlUITopBg", texture_fdid: 374155, horiz_tile: true, vert_tile: true,
    width: 314.0, height: 40.0, left: 12.0, top: 12.0, pos_type: "absolute" } };
    for (name, coords, x, y) in [
        ("TopLeft", "0,0.5,0,0.5", 0.0, 0.0),
        ("TopRight", "0.5,1,0,0.5", 306.0, 0.0),
        ("BottomLeft", "0,0.5,0.5,1", 0.0, 412.0),
        ("BottomRight", "0.5,1,0.5,1", 306.0, 412.0),
    ] {
        out.extend(cropped(
            format!("GuildControlUI{name}Corner"),
            407329,
            coords,
            (x, y, 32.0, 32.0),
        ));
    }
    out.extend(rsx! {
        texture { name: "GuildControlUITopBorder", texture_fdid: 407332, horiz_tile: true, width: 274.0, height: 16.0, left: 32.0, top: 1.0, pos_type: "absolute" }
        texture { name: "GuildControlUIBottomBorder", texture_fdid: 407328, horiz_tile: true, width: 274.0, height: 16.0, left: 32.0, top: 428.0, pos_type: "absolute" }
        texture { name: "GuildControlUILeftBorder", texture_fdid: 407330, vert_tile: true, width: 16.0, height: 380.0, left: 1.0, top: 32.0, pos_type: "absolute" }
        texture { name: "GuildControlUIRightBorder", texture_fdid: 407331, vert_tile: true, width: 16.0, height: 380.0, left: 322.0, top: 32.0, pos_type: "absolute" }
    });
    out.extend(label(
        "GuildControlUITitle".into(),
        "Guild Control",
        (24.0, 27.0, navigation_x() - 34.0, 12.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    // UIPanelCloseButton normal/pushed/highlight source atlases shared by native panels.
    out.extend(rsx! { button { name: "GuildControlUICloseButton", width: 32.0, height: 32.0, left: 299.0, top: 7.0, pos_type: "absolute",
        onclick: "guild:settings_close", button_default_skin: false,
        button_atlas_up: "uitools-closebutton", button_atlas_pressed: "uitools-closebutton-down", button_atlas_highlight: "uitools-closebutton-highlight" } });
    out.extend(horizontal_bar());
    out
}

/// HorizontalBarTemplate: four6px corners, top/bottom3px tiles; XML offsets retained.
fn horizontal_bar() -> Element {
    let mut children = Vec::new();
    for (name, atlas, rect) in [
        (
            "TopLeft",
            "UI-Frame-InnerBotLeftCorner",
            (0.0, -5.0, 6.0, 6.0),
        ),
        (
            "TopRight",
            "UI-Frame-InnerBotRight",
            (314.0, -5.0, 6.0, 6.0),
        ),
        ("BottomLeft", "UI-Frame-InnerTopLeft", (0.0, 2.0, 6.0, 6.0)),
        (
            "BottomRight",
            "UI-Frame-InnerTopRight",
            (314.0, 2.0, 6.0, 6.0),
        ),
        ("Top", "_UI-Frame-InnerBotTile", (6.0, -2.0, 308.0, 3.0)),
        ("Bottom", "_UI-Frame-InnerTopTile", (6.0, 2.0, 308.0, 3.0)),
    ] {
        children.extend(crate::quest_art::named_atlas_texture(
            format!("GuildControlUIHbar{name}"),
            atlas,
            rect,
        ));
    }
    rsx! {r#frame {name:"GuildControlUIHbar",width:320.0,height:3.0,left:9.0,top:54.0,pos_type:"absolute",{children}}}
}

pub fn panel(name: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    rsx! { texture { name: {DynName(name.into())}, width, height, left: x, top: y, pos_type: "absolute", vertex_color: "0.95,0.95,1,0.09" } }
}

/// GuildPermissionCheckboxTemplate: 20x20; label LEFT to RIGHT +(1,1).
pub fn checkbox(
    name: &str,
    text: &str,
    action: &str,
    checked: bool,
    enabled: bool,
    (x, y): (f32, f32),
) -> Element {
    let mut children = texture(
        format!("{name}Box"),
        130755,
        (0.0, 0.0, 20.0, 20.0),
        "1,1,1,1",
    );
    if checked {
        children.extend(texture(
            format!("{name}Check"),
            if enabled { 130751 } else { 130750 },
            (0.0, 0.0, 20.0, 20.0),
            "1,1,1,1",
        ));
    }
    let (text_width, text_height) = ui_toolkit::text_measure::measure_text(
        text,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        10.0,
    )
    .expect("GuildPermissionCheckboxTemplate font");
    children.extend(label(
        format!("{name}Text"),
        text,
        (
            21.0,
            (20.0 - text_height) / 2.0 - 1.0,
            text_width,
            text_height,
        ),
        (
            10.0,
            if enabled { "1,1,1,1" } else { "0.5,0.5,0.5,1" },
            "LEFT",
        ),
    ));
    children.extend(rsx! { texture { name: {DynName(format!("{name}Highlight"))}, texture_fdid: 130753, draw_layer: "HIGHLIGHT", blend_mode: "ADD", width: 20.0, height: 20.0, pos_type: "absolute", left: 0.0, top: 0.0 } });
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    rsx! { button { name: {DynName(name.into())}, width: 20.0, height: 20.0, left: x, top: y, pos_type: "absolute", onclick: action, disabled, button_default_skin: false, {children} } }
}

pub fn input(name: &str, text: &str, enabled: bool, rect: (f32, f32, f32, f32)) -> Element {
    if !enabled {
        return label(name.into(), text, rect, (12.0, "0.65,0.65,0.65,1", "LEFT"));
    }
    const TEMPLATE: &str = "GuildControlField";
    let mut children = crate::bank_art::edit_box(TEMPLATE, rect);
    for child in &mut children {
        if let ui_toolkit::widget_def::WidgetChild::Widget(widget) = child {
            if let Some(original) = &mut widget.name {
                *original = original.replace(TEMPLATE, name);
            }
        }
    }
    let input = children.last_mut().expect("InputBoxTemplate editbox");
    if let ui_toolkit::widget_def::WidgetChild::Widget(widget) = input {
        widget.attrs.push(ui_toolkit::widget_def::Attr::new_static(
            "text",
            text.into(),
        ));
    }
    children
}

/// UIPanelSquareButton: 26x26; RankChangeTemplate icon UVs for up/down/delete.
pub fn square(
    name: &str,
    action: &str,
    enabled: bool,
    coords: &str,
    (x, y): (f32, f32),
) -> Element {
    let mut children = cropped(
        format!("{name}Background"),
        423808,
        "0.015625,0.421875,0.234375,0.640625",
        (0.0, 0.0, 26.0, 26.0),
    );
    children.extend(cropped(
        format!("{name}Icon"),
        423808,
        coords,
        (7.0, 7.0, 12.0, 12.0),
    ));
    children.extend(rsx! { texture { name: {DynName(format!("{name}Highlight"))}, texture_fdid: 130757, draw_layer: "HIGHLIGHT", blend_mode: "ADD", width: 26.0, height: 26.0, pos_type: "absolute", left: 0.0, top: 0.0 } });
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    let alpha = if enabled { 1.0 } else { 0.5 };
    rsx! { button { name: {DynName(name.into())}, width: 26.0, height: 26.0, left: x, top: y, pos_type: "absolute", onclick: action, disabled, alpha, button_default_skin: false, {children} } }
}
