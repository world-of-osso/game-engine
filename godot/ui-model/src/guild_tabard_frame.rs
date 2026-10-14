//! TabardFrame.xml + Mainline PortraitFrameBaseTemplate and UIPanelLayoutFrame anchors.
use crate::{
    bank_art::{label, money_display},
    guild_tabards::{GuildTabards, load_catalog},
    quest_art::{DynName, panel_button, window_chrome},
    ui::strata::FrameStrata,
};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const MODEL: &str = "TabardModel";
pub const PORTRAIT: crate::inworld_unit_frames_component::PortraitSlot =
    crate::inworld_unit_frames_component::PortraitSlot {
        frame: "TabardFramePortrait",
        rect: (7.0, 6.0, 60.0, 60.0),
        mask_fdid: 130924,
        mask_rect: (7.0, 6.0, 60.0, 60.0),
        tex_coords: crate::inworld_unit_frames_component::FULL_PORTRAIT,
    };
const SIZE: (f32, f32) = (338.0, 424.0);
const GREETING: &str = "Greetings! Choose the symbol and colors of your guild.";
const NONLEADER_GREETING: &str =
    "You must be a guild master to purchase a tabard, but feel free to browse.";

pub fn tabard_screen(shared: &SharedContext) -> Element {
    let Some(view) = shared.get::<GuildTabards>() else {
        return vec![];
    };
    let Some(vendor) = &view.vendor else {
        return vec![];
    };
    let mut children = window_chrome("TabardFrame", SIZE, "", "tabard:close");
    children.extend(crate::quest_art::window_portrait(&PORTRAIT));
    children.extend(label(
        "TabardFrameNameText".into(),
        &vendor.npc_name,
        (120.5, 2.0, 109.0, 16.0),
        (12.0, "1,0.82,0,1", "CENTER"),
    ));
    let greeting = if vendor.own_rank == Some(0) {
        GREETING
    } else {
        NONLEADER_GREETING
    };
    children.extend(rsx! { fontstring { name: "TabardFrameGreetingText", text: greeting,
        width: 250.0, height: "auto", left: 59.0, top: 28.0, pos_type: "absolute", font_size: 12.0,
        font_color: "1,1,1,1", shadow_color: "0,0,0,1", shadow_offset: "1,-1", justify_h: "CENTER", word_wrap: true }
        r#frame { name: {DynName(MODEL.into())}, width: 303.0, height: 317.0, left: {17.5}, top: 69.0, pos_type: "absolute" } });
    children.extend(outer_frame());
    children.extend(emblem_overlays(view));
    children.extend(customization());
    children.extend(money(view));
    children.extend(panel_button(
        "TabardFrameAcceptButton".into(),
        "Accept",
        "tabard:accept",
        view.can_save(),
        (173.0, 398.0, 80.0, 22.0),
    ));
    children.extend(panel_button(
        "TabardFrameCancelButton".into(),
        "Cancel",
        "tabard:close",
        true,
        (254.0, 398.0, 80.0, 22.0),
    ));
    children.extend(arrow(
        "TabardCharacterModelRotateLeftButton",
        "",
        (14.0, 356.0, 35.0, 35.0),
    ));
    children.extend(arrow(
        "TabardCharacterModelRotateRightButton",
        "",
        (44.0, 356.0, 35.0, 35.0),
    ));
    rsx! { r#frame { name: "TabardFrame", width: {SIZE.0}, height: {SIZE.1}, left: 16.0, top: 116.0,
    pos_type: "absolute", mouse_enabled: true, strata: FrameStrata::Dialog, {children} } }
}

fn outer_frame() -> Element {
    [
        (
            "TopLeft",
            (8.0, 63.0, 20.0, 39.0),
            "0.109375,0.1875,0,0.3046875",
        ),
        (
            "Top",
            (28.0, 63.0, 280.0, 6.0),
            "0.1875,0.8828125,0,0.046875",
        ),
        (
            "TopRight",
            (308.0, 63.0, 20.0, 39.0),
            "0.8828125,0.9609375,0,0.3046875",
        ),
        ("LeftTop", (8.0, 102.0, 7.0, 128.0), "0,0.02734375,0,1"),
        (
            "LeftBottom",
            (8.0, 230.0, 7.0, 128.0),
            "0.02734375,0.0546875,0,1",
        ),
        (
            "RightTop",
            (321.0, 102.0, 7.0, 128.0),
            "0.0546875,0.08203125,0,1",
        ),
        (
            "RightBottom",
            (321.0, 230.0, 7.0, 128.0),
            "0.08203125,0.109375,0,1",
        ),
        (
            "BottomLeft",
            (8.0, 358.0, 20.0, 39.0),
            "0.109375,0.1875,0.3046875,0.6171875",
        ),
        (
            "Bottom",
            (28.0, 389.0, 280.0, 8.0),
            "0.1875,0.8828125,0.5546875,0.6171875",
        ),
        (
            "BottomRight",
            (308.0, 358.0, 20.0, 39.0),
            "0.8828125,0.9609375,0.3046875,0.6171875",
        ),
    ]
    .into_iter()
    .flat_map(|(name, rect, uv)| texture(&format!("TabardFrameOuterFrame{name}"), 136863, uv, rect))
    .collect()
}

fn emblem_overlays(view: &GuildTabards) -> Element {
    let Some(design) = view.draft else {
        return vec![];
    };
    let textures = load_catalog()
        .and_then(|catalog| catalog.textures(design, 0))
        .expect("source tabard preview emblem textures");
    [
        ("TopRight", textures.upper[1], "0,0.5,0,1", (237.0,57.0,100.0,119.0)),
        ("TopLeft", textures.upper[1], "0.5,0,0,1", (137.0,57.0,100.0,119.0)),
        ("BottomRight", textures.lower[1], "0,0.5,0,1", (237.0,176.0,100.0,74.0)),
        ("BottomLeft", textures.lower[1], "0.5,0,0,1", (137.0,176.0,100.0,74.0)),
    ].into_iter().flat_map(|(name, fdid, uv, (left,top,width,height))| {
        rsx! { texture { name: {DynName(format!("TabardFrameEmblem{name}"))}, texture_fdid: fdid, tex_coords: uv,
            left, top, width, height, pos_type: "absolute", alpha: {0.4}, draw_layer: "OVERLAY" } }
    }).collect()
}

fn customization() -> Element {
    let mut out = texture(
        "TabardFrameCustomizationBorder",
        136862,
        "0,1,0,1",
        (108.0, 196.0, 256.0, 256.0),
    );
    for (index, text) in ["Icon", "Icon Color", "Border", "Border Color", "Background"]
        .into_iter()
        .enumerate()
    {
        let id = index + 1;
        let prefix = format!("TabardFrameCustomization{id}");
        let mut children = texture(
            &format!("{prefix}Middle"),
            131143,
            "0.1953125,0.8046875,0,1",
            (34.5, -17.0, 95.0, 64.0),
        );
        children.extend(texture(
            &format!("{prefix}Left"),
            131143,
            "0,0.1953125,0,1",
            (9.5, -17.0, 25.0, 64.0),
        ));
        children.extend(texture(
            &format!("{prefix}Right"),
            131143,
            "0.8046875,1,0,1",
            (129.5, -17.0, 25.0, 64.0),
        ));
        children.extend(label(
            format!("{prefix}Text"),
            text,
            (39.5, 8.0, 85.0, 10.0),
            (10.0, "1,1,1,1", "CENTER"),
        ));
        children.extend(arrow(
            &format!("{prefix}LeftButton"),
            &format!("tabard:cycle:{id}:-1"),
            (-1.5, -3.0, 32.0, 32.0),
        ));
        children.extend(arrow(
            &format!("{prefix}RightButton"),
            &format!("tabard:cycle:{id}:1"),
            (135.5, -3.0, 32.0, 32.0),
        ));
        out.extend(rsx! { r#frame { name: {DynName(prefix)}, width: 164.0, height: 20.0, left: 156.0,
            top: {261.0 + index as f32 * 25.0}, frame_level: 2.0, pos_type: "absolute", {children} } });
    }
    out
}

fn money(view: &GuildTabards) -> Element {
    use crate::merchant_frame_component::{MoneyAlign, money_width, player_money};
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let cost = view.vendor.as_ref().expect("visible tabard vendor").cost;
    let (label_width, label_height) =
        measure_text("Cost:", GameFont::FrizQuadrata, 12.0).expect("source cost font");
    let mut cost_children = crate::game_tooltip::render::nine_slice(
        "TabardFrameCostFrame",
        (170.0, 23.0),
        "0.4,0.4,0.4,1",
        "0.09,0.09,0.188,1",
    );
    cost_children.extend(money_display(
        "TabardFrameCostMoneyFrame",
        cost,
        (170.0, 18.0),
        false,
    ));
    cost_children.extend(label(
        "TabardFrameCostLabel".into(),
        "Cost:",
        (
            170.0 - money_width(cost) - 5.0 - label_width,
            23.0 / 2.0 - 1.0 - label_height / 2.0,
            label_width,
            label_height,
        ),
        (12.0, "1,1,1,1", "RIGHT"),
    ));
    let mut out = rsx! { r#frame { name: "TabardFrameCostFrame", width: 170.0, height: 23.0, left: 151.0, top: 68.0, pos_type: "absolute", {cost_children} } };
    out.extend(texture(
        "TabardFrameMoneyInsetBg",
        374154,
        "0,1,0,1",
        (4.0, 399.0, 166.0, 21.0),
    ));
    out.extend(crate::auction_house_frame_component::inset_border(
        "TabardFrameMoneyInset",
        (4.0, 399.0, 166.0, 21.0),
    ));
    for (name, uv, rect) in [
        (
            "Left",
            "0.953125,0.9921875,0,0.296875",
            (7.0, 400.0, 7.0, 18.0),
        ),
        (
            "Middle",
            "0,0.9921875,0.3125,0.609375",
            (14.0, 400.0, 145.0, 18.0),
        ),
        ("Right", "0,0.0546875,0,0.296875", (159.0, 400.0, 7.0, 18.0)),
    ] {
        out.extend(texture(
            &format!("TabardFrameMoneyBg{name}"),
            525911,
            uv,
            rect,
        ));
    }
    if let Some(gold) = view.gold {
        out.extend(player_money(
            "TabardFrameMoneyFrame",
            gold,
            (175.0, 416.0),
            MoneyAlign::Right,
        ));
    }
    out
}

fn texture(
    name: &str,
    fdid: u32,
    uv: &str,
    (left, top, width, height): (f32, f32, f32, f32),
) -> Element {
    rsx! { texture { name: {DynName(name.into())}, texture_fdid: fdid, tex_coords: uv,
    left, top, width, height, pos_type: "absolute" } }
}
fn arrow(name: &str, action: &str, (left, top, width, height): (f32, f32, f32, f32)) -> Element {
    rsx! { button { name: {DynName(name.into())}, onclick: action, left, top, width, height,
    pos_type: "absolute", mouse_enabled: true } }
}

pub fn apply_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{
        atlas::{ActiveSkin, thread_skin},
        frame::WidgetData,
        widgets::texture::{BlendMode, TextureSource},
    };
    let ids: Vec<_> = registry.frames_iter().map(|frame| frame.id).collect();
    for name in ids {
        let frame = registry.get_mut(name).expect("registered tabard frame");
        let Some(name) = frame.name.as_deref() else {
            continue;
        };
        if name == "TabardFrameMoneyInsetBg" {
            if let Some(WidgetData::Texture(texture)) = &mut frame.widget_data {
                texture.horiz_tile = true;
                texture.vert_tile = true;
            }
        }
        if name.starts_with("TabardFrameEmblem") {
            if let Some(WidgetData::Texture(texture)) = &mut frame.widget_data {
                texture.blend_mode = BlendMode::Additive;
            }
        }
        let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
            continue;
        };
        let textures = if name.starts_with("TabardFrameCustomization")
            && name.ends_with("LeftButton")
        {
            Some((130869, 130868, Some(130867), 130757))
        } else if name.starts_with("TabardFrameCustomization") && name.ends_with("RightButton") {
            Some((130866, 130865, Some(130864), 130757))
        } else if name == "TabardCharacterModelRotateLeftButton" {
            Some((130845, 130844, None, 130717))
        } else if name == "TabardCharacterModelRotateRightButton" {
            Some((130847, 130846, None, 130717))
        } else {
            None
        };
        if let Some((normal, pushed, disabled, highlight)) = textures {
            button.normal_texture = Some(TextureSource::FileDataId(normal));
            button.pushed_texture = Some(TextureSource::FileDataId(pushed));
            button.disabled_texture = disabled.map(TextureSource::FileDataId);
            button.highlight_texture = Some(TextureSource::FileDataId(highlight));
        } else if thread_skin() == ActiveSkin::Modern
            && matches!(name, "TabardFrameAcceptButton" | "TabardFrameCancelButton")
        {
            button.normal_texture = Some(TextureSource::FileDataId(130828));
            button.pushed_texture = Some(TextureSource::FileDataId(130825));
            button.disabled_texture = Some(TextureSource::FileDataId(130824));
            button.highlight_texture = None;
        }
    }
}
